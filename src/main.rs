use std::env;
use std::fs;
use std::path::Path;

use airyc_codegen::llvm_ir::Program;
use airyc_parser::ast::{AstNode, CompUnit};
use airyc_parser::visitor::Visitor as _;
use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetTriple};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input_file> [-o <output_file>]", args[0]);
        return;
    }

    let input_path = &args[1];
    let input = fs::read_to_string(input_path).expect("Failed to read input file");

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
    let module_name = Path::new(input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    // analyzer
    let root = airyc_parser::parser::Parser::new_root(green_node);
    let mut analyzer = airyc_analyzer::module::Module::default();
    analyzer.walk(&root);

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

    let comp_unit = CompUnit::cast(root).expect("Root node is not CompUnit");
    program.compile_comp_unit(comp_unit);

    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetTriple::create("x86_64-pc-linux-gnu");
    let target = Target::from_triple(&triple).expect("Failed to get target from triple");
    let tm = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .expect("Failed to create target machine");

    module.set_triple(&tm.get_triple());
    module.set_data_layout(&tm.get_target_data().get_data_layout());

    // 3. Write LLVM IR to file
    let output_path = if let Some(idx) = args.iter().position(|x| x == "-o") {
        if idx + 1 < args.len() {
            Path::new(&args[idx + 1]).to_path_buf()
        } else {
            Path::new(input_path).with_extension("ll")
        }
    } else {
        Path::new(input_path).with_extension("ll")
    };

    program
        .module
        .print_to_file(&output_path)
        .expect("Failed to write LLVM IR");

    if let Err(e) = module.verify() {
        panic!("{}", e.to_string_lossy());
    }
}
