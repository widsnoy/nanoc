use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use codegen::compiler::OptLevel;

/// airyc 编译器命令行参数
#[derive(Parser, Debug)]
#[command(name = "airyc cli", version = "0.0.1", about = "airyc compiler")]
pub struct Args {
    /// source file (.ariy) path
    #[arg(short, long)]
    pub input_path: PathBuf,

    /// output dir, (default .)
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,

    /// runtime path, default /usr/local/lib/libsysy.a
    #[arg(short, long, default_value = "/usr/local/lib/libairyc_runtime.a")]
    pub runtime: PathBuf,

    /// emit target
    #[arg(short, long, value_enum, default_value_t = EmitTarget::Exe)]
    pub emit: EmitTarget,

    /// optimization level
    #[arg(short = 'O', default_value = "o0")]
    pub opt_level: CliOptLevel,
}

/// 编译输出目标
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum EmitTarget {
    /// 输出 LLVM IR (.ll 文件)
    Ir,
    /// 输出可执行文件
    Exe,
    /// 输出 AST
    Ast,
    /// 静态分析
    Check,
}

/// CLI 优化级别（用于 clap 解析）
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum CliOptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl From<CliOptLevel> for OptLevel {
    fn from(level: CliOptLevel) -> Self {
        match level {
            CliOptLevel::O0 => OptLevel::O0,
            CliOptLevel::O1 => OptLevel::O1,
            CliOptLevel::O2 => OptLevel::O2,
            CliOptLevel::O3 => OptLevel::O3,
        }
    }
}
