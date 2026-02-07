use thiserror::Error;

pub type Result<T> = std::result::Result<T, CodegenError>;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("missing {0}")]
    Missing(&'static str),

    #[error("LLVM build failed: {0}")]
    LlvmBuild(&'static str),

    #[error("undefined variable: {0}")]
    UndefinedVar(String),

    #[error("undefined function: {0}")]
    UndefinedFunc(String),

    #[error("type mismatch: {0}")]
    TypeMismatch(String),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("not implemented: {0}")]
    NotImplemented(&'static str),

    #[error("LLVM verification failed: {0}")]
    LlvmVerification(String),

    #[error("failed to write LLVM output: {0}")]
    LlvmWrite(String),

    #[error("target machine error: {0}")]
    TargetMachine(String),

    #[error("root node is not CompUnit")]
    InvalidRoot,
}
