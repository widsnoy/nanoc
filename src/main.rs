use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use airyc_codegen::llvm_ir::Program;
use airyc_parser::ast::{AstNode, CompUnit};
use airyc_parser::visitor::Visitor as _;
use clap::{Parser, ValueEnum};
use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::TargetMachine;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target};

#[derive(Parser, Debug)]
#[command(name = "airyc", version = "0.0.1", about = "airyc compiler")]
struct Args {
    /// source file (.yc) path
    #[arg(short, long)]
    input_path: PathBuf,

    /// output dir, (default .)
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// runtime path, default /usr/local/lib/libsysy.a
    #[arg(short, long, default_value = "/usr/local/lib/libairyc_runtime.a")]
    runtime: PathBuf,

    /// emit target
    #[arg(short, long, value_enum, default_value_t = EmitTarget::Exe)]
    emit: EmitTarget,

    /// optimization level
    #[arg(short = 'O', default_value = "o0")]
    opt_level: OptLevel,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum EmitTarget {
    Ir,
    Exe,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl From<OptLevel> for OptimizationLevel {
    fn from(level: OptLevel) -> Self {
        match level {
            OptLevel::O0 => OptimizationLevel::None,
            OptLevel::O1 => OptimizationLevel::Less,
            OptLevel::O2 => OptimizationLevel::Default,
            OptLevel::O3 => OptimizationLevel::Aggressive,
        }
    }
}

fn main() -> Result<(), Cow<'static, str>> {
    let args = Args::parse();

    let input_path = args.input_path;
    let input =
        fs::read_to_string(&input_path).map_err(|_| Cow::Borrowed("Failed to read input file"))?;

    // 1. Parse
    let parser = airyc_parser::parser::Parser::new(&input);
    let (green_node, errors) = parser.parse();
    if !errors.is_empty() {
        eprintln!("Parser errors:");
        for error in errors {
            eprintln!("- {}", error);
        }
        std::process::exit(1);
    }

    // 2. Codegen (LLVM IR)
    let context = Context::create();
    // Use filename as module name
    let module_name = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unkown");
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    // analyzer
    let root = airyc_parser::parser::Parser::new_root(green_node);
    let mut analyzer = airyc_analyzer::module::Module::default();
    analyzer.walk(&root);

    // todo: fixme
    if !analyzer.analyzing.errors.is_empty() {
        panic!("{:?}", analyzer.analyzing.errors);
    }

    let mut program = Program {
        context: &context,
        builder: &builder,
        module: &module,
        analyzer: &analyzer,
        current_function: None,
        scopes: Vec::new(),
        functions: Default::default(),
        globals: Default::default(),
        loop_stack: Vec::new(),
    };

    let comp_unit = CompUnit::cast(root).ok_or("Root node is not CompUnit")?;
    program.compile_comp_unit(comp_unit);

    let opt_level: OptimizationLevel = args.opt_level.into();
    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).map_err(|e| Cow::Owned(e.to_string()))?;
    let cpu = "generic";
    let machine = target
        .create_target_machine(
            &triple,
            cpu,
            "",
            opt_level,
            RelocMode::Default,
            CodeModel::Default,
        )
        .expect("Failed to create target machine");

    module.set_triple(&machine.get_triple());
    module.set_data_layout(&machine.get_target_data().get_data_layout());

    // 3. Write LLVM IR to file
    let mut file_name = PathBuf::from_str(module_name).map_err(|e| Cow::Owned(e.to_string()))?;
    match args.emit {
        EmitTarget::Ir => {
            file_name.add_extension("ll");

            let output_path = args.output_dir.join(file_name);
            program
                .module
                .print_to_file(output_path)
                .expect("faild to write llvm ir");
        }
        EmitTarget::Exe => {
            let output_path = args.output_dir.join(&file_name);
            file_name.add_extension("o");
            let object_path = args.output_dir.join(file_name);
            machine
                .write_to_file(&module, inkwell::targets::FileType::Object, &object_path)
                .map_err(|e| Cow::Owned(e.to_string()))?;
            std::process::Command::new("clang")
                .arg(object_path.as_os_str())
                .arg(args.runtime.as_os_str())
                .arg("-o")
                .arg(output_path.as_os_str())
                .status()
                .expect("link error");
        }
    }

    if let Err(e) = module.verify() {
        panic!("{}", e.to_string_lossy());
    }
    Ok(())
}
