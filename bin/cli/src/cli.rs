use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// airyc 编译器命令行参数
#[derive(Parser, Debug)]
#[command(name = "airyc-cli", version = "0.0.1", about = "airyc-lang cli tool")]
pub struct Args {
    /// source file(s) (.airy) path - can specify multiple files
    #[arg(short, long, num_args = 1..)]
    pub input_path: Vec<PathBuf>,

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
    #[arg(short = 'O', default_value = "default")]
    pub opt_level: OptLevel,
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

/// 优化级别
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OptLevel {
    None,
    Less,
    Default,
    Aggressive,
}
impl From<OptLevel> for inkwell::OptimizationLevel {
    fn from(level: OptLevel) -> Self {
        match level {
            OptLevel::None => inkwell::OptimizationLevel::None,
            OptLevel::Less => inkwell::OptimizationLevel::Less,
            OptLevel::Default => inkwell::OptimizationLevel::Default,
            OptLevel::Aggressive => inkwell::OptimizationLevel::Aggressive,
        }
    }
}
