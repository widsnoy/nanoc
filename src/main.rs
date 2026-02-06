use std::fs;

use clap::Parser;
use inkwell::context::Context as LlvmContext;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target};

mod analyzing;
mod cli;
mod codegen;
mod error;
mod linking;
mod parsing;

use cli::{Args, EmitTarget};
use syntax::SyntaxNode;

fn main() {
    let args = Args::parse();

    // 1. 读取源文件
    let input_path = &args.input_path;
    let input = match fs::read_to_string(input_path) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("Error: failed to read input file: {}", e);
            std::process::exit(1);
        }
    };

    // 2. 语法分析
    let (green_node, _parser_errors, _lexer_errors) = match parsing::parse(&input) {
        Ok(result) => result,
        Err(e) => {
            e.report(input_path, input);
            std::process::exit(1);
        }
    };

    // 3. 如果只需要 AST，直接输出并退出
    if args.emit == EmitTarget::Ast {
        println!("{:#?}", SyntaxNode::new_root(green_node));
        return;
    }

    // 4. 语义分析
    let analyzer = match analyzing::analyze(green_node.clone()) {
        Ok(analyzer) => analyzer,
        Err(e) => {
            e.report(input_path, input);
            std::process::exit(1);
        }
    };

    if args.emit == EmitTarget::Check {
        return;
    }

    // 5. 初始化 LLVM
    let context = LlvmContext::create();
    let module_name = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // 6. 代码生成
    let codegen_ctx = match codegen::generate_ir(&context, module_name, green_node, &analyzer) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // 7. 初始化目标机器
    let opt_level = args.opt_level.into();
    Target::initialize_all(&InitializationConfig::default());
    let triple = inkwell::targets::TargetMachine::get_default_triple();
    let target = match Target::from_triple(&triple) {
        Ok(target) => target,
        Err(e) => {
            eprintln!("Error: failed to create target from triple: {}", e);
            std::process::exit(1);
        }
    };
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            opt_level,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap_or_else(|| {
            eprintln!("Error: failed to create target machine");
            std::process::exit(1);
        });

    // 8. 优化和验证
    if let Err(e) = codegen::optimize_and_verify(&codegen_ctx.module, &machine) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // 9. 输出
    match args.emit {
        EmitTarget::Ir => {
            let output_path = args.output_dir.join(format!("{}.ll", module_name));
            if let Err(e) = linking::write_ir(&codegen_ctx.module, &output_path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        EmitTarget::Exe => {
            if let Err(e) = linking::link_executable(
                &codegen_ctx.module,
                &machine,
                &args.output_dir,
                module_name,
                &args.runtime,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        EmitTarget::Ast | EmitTarget::Check => {}
    }
}
