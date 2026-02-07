use std::fs;

use clap::Parser;

mod analyzing;
mod cli;
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

    // 5. 获取模块名称
    let module_name = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let opt_level = args.opt_level.into();

    // 6. 代码生成和输出
    match args.emit {
        EmitTarget::Ir => {
            let output_path = args.output_dir.join(format!("{}.ll", module_name));
            if let Err(e) = codegen::compiler::compile_to_ir_file(
                module_name,
                green_node,
                &analyzer,
                opt_level,
                &output_path,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        EmitTarget::Exe => {
            // 生成目标文件字节
            let object_bytes = match codegen::compiler::compile_to_object_bytes(
                module_name,
                green_node,
                &analyzer,
                opt_level,
            ) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };

            // 链接生成可执行文件
            if let Err(e) = linking::link_executable(
                &object_bytes,
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
