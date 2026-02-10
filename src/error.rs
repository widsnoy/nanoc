use std::collections::HashMap;

use analyzer::error::AnalyzeError;
use codegen::error::CodegenError;
use miette::NamedSource;
use thiserror::Error;
use vfs::{FileID, Vfs};

pub type Result<T> = std::result::Result<T, CompilerError>;

/// 编译器错误
#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("failed to read input file: {0}")]
    Io(#[from] std::io::Error),

    #[error("analyze errors occurred")]
    Analyze(HashMap<FileID, Vec<AnalyzeError>>),

    #[error("codegen failed: {0}")]
    Codegen(#[from] CodegenError),

    #[error("link failed: {0}")]
    Link(String),

    #[error("linker returned non-zero status")]
    LinkerFailed,

    #[error("invalid path: {0}")]
    InvalidPath(#[from] std::path::StripPrefixError),
}

impl CompilerError {
    /// 报告编译错误
    pub fn report(self, vfs: Vfs) {
        match self {
            Self::Analyze(semantic_errors) => {
                for (file_id, errors) in semantic_errors {
                    if let Some(file) = vfs.get_file_by_file_id(&file_id) {
                        let source =
                            NamedSource::new(file.path.to_string_lossy(), file.text.clone());
                        for error in errors {
                            let report =
                                miette::Report::new(error).with_source_code(source.clone());
                            println!("{:?}", report);
                        }
                    }
                }
            }
            _ => println!("Error: {}", self),
        }
    }
}
