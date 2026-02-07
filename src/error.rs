use std::collections::HashMap;

use analyzer::error::SemanticError;
use codegen::error::CodegenError;
use lexer::LexerError;
use miette::NamedSource;
use parser::parse::ParserError;
use thiserror::Error;
use vfs::{FileID, Vfs};

/// 编译器统一错误类型
pub type Result<T> = std::result::Result<T, CompilerError>;

/// 编译器错误
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum CompilerError {
    #[error("failed to read input file: {0}")]
    Io(#[from] std::io::Error),

    #[error("lexer errors occurred")]
    Lexer(HashMap<FileID, Vec<LexerError>>),

    #[error("parser errors occurred")]
    Parser(HashMap<FileID, Vec<ParserError>>),

    #[error("semantic errors occurred")]
    Semantic(HashMap<FileID, Vec<SemanticError>>),

    #[error("codegen failed: {0}")]
    Codegen(#[from] CodegenError),

    #[error("link failed: {0}")]
    Link(String),

    #[error("linker returned non-zero status")]
    LinkerFailed,

    #[error("invalid path: {0}")]
    InvalidPath(#[from] std::path::StripPrefixError),

    #[error("dependency discovery failed: {0}")]
    Discovery(String),
}

impl CompilerError {
    pub fn report(&self, vfs: &Vfs) {
        match self {
            CompilerError::Lexer(errors_by_file) => {
                for (file_id, errors) in errors_by_file {
                    let file = vfs.get_file_by_file_id(file_id).unwrap();
                    let source =
                        NamedSource::new(file.path.display().to_string(), file.text.clone());

                    for error in errors {
                        let report =
                            miette::Report::new(error.clone()).with_source_code(source.clone());
                        eprintln!("{:?}", report);
                    }
                }
            }
            CompilerError::Parser(errors_by_file) => {
                for (file_id, errors) in errors_by_file {
                    let file = vfs.get_file_by_file_id(file_id).unwrap();
                    let source =
                        NamedSource::new(file.path.display().to_string(), file.text.clone());

                    for error in errors {
                        let report =
                            miette::Report::new(error.clone()).with_source_code(source.clone());
                        eprintln!("{:?}", report);
                    }
                }
            }
            CompilerError::Semantic(errors_by_file) => {
                for (file_id, errors) in errors_by_file {
                    let file = vfs.get_file_by_file_id(file_id).unwrap();
                    let source =
                        NamedSource::new(file.path.display().to_string(), file.text.clone());

                    for error in errors {
                        let report =
                            miette::Report::new(error.clone()).with_source_code(source.clone());
                        eprintln!("{:?}", report);
                    }
                }
            }
            CompilerError::Discovery(msg) => {
                eprintln!("Error: {}", msg);
            }
            _ => {
                // 其他错误直接输出
                eprintln!("Error: {}", self);
            }
        }
    }
}
