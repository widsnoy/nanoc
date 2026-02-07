use std::collections::HashMap;

use clap::Parser;

mod cli;
mod error;
mod linking;

use analyzer::module::ModuleID;
use analyzer::project::Project;
use cli::{Args, EmitTarget};
use error::CompilerError;
use syntax::SyntaxNode;
use vfs::FileID;

fn main() {
    let args = Args::parse();

    if compile(args).is_err() {
        std::process::exit(1);
    }
}

/// 自动处理单文件和多文件场景
fn compile(args: Args) -> Result<(), ()> {
    // 依赖发现（自动处理单文件和多文件）
    let discovery = match utils::discover_dependencies(&args.input_path) {
        Ok(d) => d,
        Err(e) => {
            let error = CompilerError::Discovery(e);
            error.report(&vfs::Vfs::new());
            return Err(());
        }
    };

    // 检查解析错误
    if !discovery.parse_errors.is_empty() {
        let error = CompilerError::Parser(discovery.parse_errors);
        error.report(&discovery.vfs);
        return Err(());
    }

    if !discovery.lexer_errors.is_empty() {
        let error = CompilerError::Lexer(discovery.lexer_errors);
        error.report(&discovery.vfs);
        return Err(());
    }

    // 如果只需要 AST
    if args.emit == EmitTarget::Ast {
        let entry_green = discovery.green_trees.get(&discovery.entry_file_id).unwrap();
        println!("{:#?}", SyntaxNode::new_root(entry_green.clone()));
        return Ok(());
    }

    // 创建 Project 并分析
    let project = Project::new(discovery.vfs, discovery.green_trees);

    // 收集语义错误
    let mut semantic_errors: HashMap<FileID, Vec<_>> = HashMap::new();
    for (module_id, module) in project.modules.iter() {
        if !module.semantic_errors.is_empty() {
            // 找到对应的 FileID
            if let Some((file_id, _)) = project
                .file_index
                .iter()
                .find(|(_, mid)| **mid == ModuleID(module_id))
            {
                semantic_errors.insert(*file_id, module.semantic_errors.clone());
            }
        }
    }

    // 报告语义错误
    if !semantic_errors.is_empty() {
        let error = CompilerError::Semantic(semantic_errors);
        error.report(&project.vfs);
        return Err(());
    }

    if args.emit == EmitTarget::Check {
        return Ok(());
    }

    // 获取入口模块
    let entry_module_id = project.file_index.get(&discovery.entry_file_id).unwrap();
    let entry_module = project.modules.get(**entry_module_id).unwrap();
    let entry_green = entry_module.get_green_tree();

    // 获取模块名称
    let module_name = project
        .vfs
        .get_file_by_file_id(&discovery.entry_file_id)
        .and_then(|f| f.path.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let opt_level = args.opt_level.into();

    // 代码生成
    match args.emit {
        EmitTarget::Ir => {
            let output_path = args.output_dir.join(format!("{}.ll", module_name));
            if let Err(e) = codegen::compiler::compile_to_ir_file(
                module_name,
                entry_green,
                entry_module,
                Some(&project),
                opt_level,
                &output_path,
            ) {
                eprintln!("Error: {}", e);
                return Err(());
            }
        }
        EmitTarget::Exe => {
            let object_bytes = match codegen::compiler::compile_to_object_bytes(
                module_name,
                entry_green,
                entry_module,
                Some(&project),
                opt_level,
            ) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return Err(());
                }
            };

            if let Err(e) = linking::link_executable(
                &object_bytes,
                &args.output_dir,
                module_name,
                &args.runtime,
            ) {
                eprintln!("Error: {}", e);
                return Err(());
            }
        }
        EmitTarget::Ast | EmitTarget::Check => {}
    }

    Ok(())
}
