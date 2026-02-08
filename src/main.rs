use std::fs;

use clap::Parser;

mod analyzing;
mod cli;
mod error;
mod linking;
mod parsing;

use syntax::SyntaxNode;

use cli::{Args, EmitTarget};

fn main() {
    let args = Args::parse();

    // 检查是否有输入文件
    if args.input_path.is_empty() {
        eprintln!("Error: no input files specified");
        std::process::exit(1);
    }

    compile(args);
}

fn compile(args: Args) {
    // 如果只需要 AST，使用简单的解析流程
    if args.emit == EmitTarget::Ast {
        let input_path = &args.input_path[0];
        let input = match fs::read_to_string(input_path) {
            Ok(input) => input,
            Err(e) => {
                eprintln!("Error: failed to read input file: {}", e);
                std::process::exit(1);
            }
        };

        let (green_node, _parser_errors, _lexer_errors) = match parsing::parse(&input) {
            Ok(result) => result,
            Err(e) => {
                e.report(input_path, input);
                std::process::exit(1);
            }
        };

        println!("{:#?}", SyntaxNode::new_root(green_node));
        return;
    }

    // 语义分析（使用 Project 架构）
    let project = match analyzing::analyze_project(&args.input_path) {
        Ok(project) => project,
        Err(e) => {
            // 报告第一个文件的错误
            let first_file = &args.input_path[0];
            let input = fs::read_to_string(first_file).unwrap_or_default();
            e.report(first_file, input);
            std::process::exit(1);
        }
    };

    if args.emit == EmitTarget::Check {
        if args.input_path.len() > 1 {
            println!("✓ All files checked successfully");
        } else {
            println!("✓ File checked successfully");
        }
        return;
    }

    let opt_level = args.opt_level.into();

    // 代码生成
    match args.emit {
        EmitTarget::Ir => {
            // 为每个模块生成 IR 文件
            for (module_id, module) in project.modules.iter() {
                // 获取模块名称
                let module_name = project
                    .file_index
                    .iter()
                    .find(|(_, mid)| mid.0 == module_id)
                    .and_then(|(file_id, _)| project.vfs.files.get(file_id.0))
                    .and_then(|file| {
                        std::path::Path::new(&file.path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                    })
                    .unwrap_or("unknown");

                let output_path = args.output_dir.join(format!("{}.ll", module_name));
                if let Err(e) = codegen::compiler::compile_to_ir_file(
                    module_name,
                    module.green_tree.clone(),
                    module,
                    opt_level,
                    &output_path,
                ) {
                    eprintln!("Error generating IR for {}: {}", module_name, e);
                    std::process::exit(1);
                }
            }
        }
        EmitTarget::Exe => {
            // 生成所有模块的目标文件
            let object_files =
                match codegen::compiler::compile_project_to_object_bytes(&project, opt_level) {
                    Ok(files) => files,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                };

            // 确定输出文件名（使用第一个文件的名称）
            let output_name = args.input_path[0]
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("a.out");

            // 链接所有目标文件
            if let Err(e) = linking::link_multiple_objects(
                &object_files,
                &args.output_dir,
                output_name,
                &args.runtime,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        EmitTarget::Ast | EmitTarget::Check => {}
    }
}
