use std::path::Path;

use analyzer::error::SemanticError;
use codegen::error::CodegenError;
use lexer::LexerError;
use miette::NamedSource;
use parser::parse::ParserError;
use thiserror::Error;

/// 编译器统一错误类型
pub type Result<T> = std::result::Result<T, CompilerError>;

/// 编译器错误
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum CompilerError {
    #[error("failed to read input file: {0}")]
    Io(#[from] std::io::Error),

    #[error("lexer errors occurred")]
    Lexer(Vec<LexerError>),

    #[error("parser errors occurred")]
    Parser(Vec<ParserError>),

    #[error("semantic errors occurred")]
    Semantic(Vec<SemanticError>),

    #[error("codegen failed: {0}")]
    Codegen(#[from] CodegenError),

    #[error("LLVM verification failed: {0}")]
    LlvmVerification(String),

    #[error("failed to write LLVM IR: {0}")]
    LlvmWrite(String),

    #[error("failed to create target machine")]
    TargetMachine,

    #[error("failed to create target from triple: {0}")]
    TargetFromTriple(String),

    #[error("link failed: {0}")]
    Link(String),

    #[error("linker returned non-zero status")]
    LinkerFailed,

    #[error("root node is not CompUnit")]
    InvalidRoot,

    #[error("invalid path: {0}")]
    InvalidPath(#[from] std::path::StripPrefixError),
}

impl CompilerError {
    /// 报告错误，使用 miette 格式化输出
    ///
    /// 对于包含多个子错误的错误类型（Lexer, Parser, Semantic），
    /// 会逐个输出每个子错误的详细信息
    pub fn report(&self, source_path: &Path, source_code: String) {
        let source = NamedSource::new(source_path.to_string_lossy(), source_code);

        match self {
            CompilerError::Lexer(errors) => {
                for error in errors {
                    let report =
                        miette::Report::new(error.clone()).with_source_code(source.clone());
                    println!("{:?}", report);
                }
            }
            CompilerError::Parser(errors) => {
                for error in errors {
                    let report =
                        miette::Report::new(error.clone()).with_source_code(source.clone());
                    println!("{:?}", report);
                }
            }
            CompilerError::Semantic(errors) => {
                for error in errors {
                    let report =
                        miette::Report::new(error.clone()).with_source_code(source.clone());
                    println!("{:?}", report);
                }
            }
            _ => {
                // 其他错误直接输出
                println!("Error: {}", self);
            }
        }
    }
}
