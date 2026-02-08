use std::fs;

use clap::Parser;

mod analyzing;
mod cli;
mod error;
mod linking;
mod parsing;

use syntax::SyntaxNode;

use cli::{Args, EmitTarget};

use crate::error::CompilerError;

fn main() -> Result<(), CompilerError> {
    let args = Args::parse();

    // 检查是否有输入文件
    if args.input_path.is_empty() {
        eprintln!("Error: no input files specified");
        std::process::exit(1);
    }

    compile(args)
}

fn compile(args: Args) -> Result<(), CompilerError> {
    // 如果只需要 AST，使用简单的解析流程
    if args.emit == EmitTarget::Ast {
        if args.input_path.len() > 1 {
            eprintln!("Error: more than one file");
            std::process::exit(1);
        }

        let input_path = &args.input_path[0];
        let input = fs::read_to_string(input_path)?;

        let green_node = match parsing::parse(input_path, &input) {
            Ok(result) => result,
            Err(e) => {
                e.report();
                std::process::exit(1);
            }
        };

        println!("{:#?}", SyntaxNode::new_root(green_node));
        return Ok(());
    }

    // 语义分析
    let mut project = match analyzing::analyze_project(&args.input_path) {
        Ok(project) => project,
        Err(e) => {
            if let CompilerError::Io(_) = e {
                return Err(e);
            }

            e.report();
            std::process::exit(1);
        }
    };

    if args.emit == EmitTarget::Check {
        if args.input_path.len() > 1 {
            println!("✓ All files checked successfully");
        } else {
            println!("✓ File checked successfully");
        }
        return Ok(());
    }

    let opt_level = args.opt_level.into();

    // 为代码生成准备：重新设置 project 指针
    project.prepare_for_codegen();

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
                codegen::compiler::compile_to_ir_file(
                    module_name,
                    module.green_tree.clone(),
                    module,
                    opt_level,
                    &output_path,
                )?;
            }
        }
        EmitTarget::Exe => {
            // 生成所有模块的目标文件
            let object_files =
                codegen::compiler::compile_project_to_object_bytes(&project, opt_level)?;

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
    };
    Ok(())
}
