use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use inkwell::OptimizationLevel;

/// airyc 编译器命令行参数
#[derive(Parser, Debug)]
#[command(name = "airyc", version = "0.0.1", about = "airyc compiler")]
pub struct Args {
    /// source file (.yc) path
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
    pub opt_level: OptLevel,
}

/// 编译输出目标
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum EmitTarget {
    /// 输出 LLVM IR (.ll 文件)
    Ir,
    /// 输出可执行文件
    Exe,
    /// 输出 AST（调试用）
    Ast,
}

/// 优化级别
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl From<OptLevel> for OptimizationLevel {
    fn from(level: OptLevel) -> Self {
        match level {
            OptLevel::O0 => OptimizationLevel::None,
            OptLevel::O1 => OptimizationLevel::Less,
            OptLevel::O2 => OptimizationLevel::Default,
            OptLevel::O3 => OptimizationLevel::Aggressive,
        }
    }
}
