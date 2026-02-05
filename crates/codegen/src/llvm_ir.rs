use std::collections::HashMap;

use inkwell::basic_block::BasicBlock;
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::{builder::Builder, context::Context};
use parser::ast::*;

use crate::error::Result;

mod decl;
mod expr;
mod func;
mod stmt;

/// 变量和函数的符号表
#[derive(Default)]
pub struct SymbolTable<'a, 'ctx> {
    pub current_function: Option<FunctionValue<'ctx>>,
    pub scopes: Vec<HashMap<String, Symbol<'a, 'ctx>>>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub globals: HashMap<String, Symbol<'a, 'ctx>>,
    pub loop_stack: Vec<LoopContext<'ctx>>,
}

pub struct Program<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a inkwell::module::Module<'ctx>,
    pub analyzer: &'a analyzer::module::Module,
    pub symbols: SymbolTable<'a, 'ctx>,
}

#[derive(Clone, Copy)]
pub struct Symbol<'a, 'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: &'a analyzer::r#type::NType,
}

impl<'a, 'ctx> Symbol<'a, 'ctx> {
    pub fn new(ptr: PointerValue<'ctx>, ty: &'a analyzer::r#type::NType) -> Self {
        Self { ptr, ty }
    }
}

pub struct LoopContext<'ctx> {
    pub cond_bb: BasicBlock<'ctx>,
    pub end_bb: BasicBlock<'ctx>,
}

impl<'a, 'ctx> Program<'a, 'ctx> {
    pub fn compile_comp_unit(&mut self, node: CompUnit) -> Result<()> {
        self.declare_sysy_runtime();
        for global in node.global_decls() {
            match global {
                GlobalDecl::VarDef(decl) => self.compile_var_def(decl)?,
                GlobalDecl::FuncDef(func) => self.compile_func_def(func)?,
                GlobalDecl::StructDef(_) => {}
            }
        }
        Ok(())
    }
}
