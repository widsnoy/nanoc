use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use codegen::llvm_ir::Program;
use inkwell::OptimizationLevel;
use inkwell::context::Context as LlvmContext;
use inkwell::targets::TargetMachine;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target};
use miette::NamedSource;
use parser::SyntaxNode;
use parser::ast::{AstNode, CompUnit};

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
    Ast,
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

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = args.input_path;
    let input = fs::read_to_string(&input_path)
        .map_err(|e| anyhow::anyhow!("failed to read input file: {}", e))?;

    // Parse
    let parser = parser::parse::Parser::new(&input);
    let (green_node, parser_errors, lexer_errors) = parser.parse();

    // 报告 Parser 错误
    if !parser_errors.is_empty() {
        let source = NamedSource::new(input_path.to_string_lossy().to_string(), input.clone());
        eprintln!("Parser errors:");
        for error in &parser_errors {
            let report = miette::Report::new(error.clone()).with_source_code(source.clone());
            eprintln!("{:?}", report);
        }
        bail!("Parsing failed");
    }

    if args.emit == EmitTarget::Ast {
        // FIXME:
        println!("{:#?}", SyntaxNode::new_root(green_node));
        return Ok(());
    }

    // Codegen (LLVM IR)
    let context = LlvmContext::create();
    let module_name = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unkown");
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    // analyzer
    let root = SyntaxNode::new_root(green_node.clone());
    let mut analyzer = analyzer::module::Module::new(green_node);
    analyzer.analyze();

    if !analyzer.semantic_errors.is_empty() {
        let source = NamedSource::new(input_path.to_string_lossy().to_string(), input.clone());
        eprintln!("Semantic errors:");
        for err in &analyzer.semantic_errors {
            let report = miette::Report::new(err.clone()).with_source_code(source.clone());
            eprintln!("{:?}", report);
        }
        bail!("semantic analysis failed");
    }

    let mut program = Program {
        context: &context,
        builder: &builder,
        module: &module,
        analyzer: &analyzer,
        symbols: Default::default(),
    };

    let comp_unit =
        CompUnit::cast(root).ok_or_else(|| anyhow::anyhow!("Root node is not CompUnit"))?;
    program
        .compile_comp_unit(comp_unit)
        .map_err(|e| anyhow::anyhow!("codegen failed: {}", e))?;

    let opt_level: OptimizationLevel = args.opt_level.into();
    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).map_err(|e| anyhow::anyhow!("{}", e))?;
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
        .ok_or_else(|| anyhow::anyhow!("failed to create target machine"))?;

    module.set_triple(&machine.get_triple());
    module.set_data_layout(&machine.get_target_data().get_data_layout());

    //  Verify before output
    if let Err(e) = module.verify() {
        bail!("LLVM verification failed: {}", e.to_string_lossy());
    }

    //  Write output
    let mut file_name = PathBuf::from_str(module_name)?;
    match args.emit {
        EmitTarget::Ir => {
            file_name.set_extension("ll");
            let output_path = args.output_dir.join(file_name);
            program
                .module
                .print_to_file(&output_path)
                .map_err(|e| anyhow::anyhow!("failed to write LLVM IR: {}", e))?;
        }
        EmitTarget::Exe => {
            let output_path = args.output_dir.join(&file_name);
            file_name.set_extension("o");
            let object_path = args.output_dir.join(&file_name);
            machine
                .write_to_file(&module, inkwell::targets::FileType::Object, &object_path)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let status = std::process::Command::new("clang")
                .arg(&object_path)
                .arg(&args.runtime)
                .arg("-o")
                .arg(&output_path)
                .status()
                .map_err(|e| anyhow::anyhow!("link failed: {}", e))?;
            if !status.success() {
                bail!("linker returned non-zero status");
            }
        }
        _ => {}
    }

    Ok(())
}
