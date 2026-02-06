//! 主要进行类型推导和常量计算, 以及基本的检查

mod decl;
mod expr;
mod func;
mod stmt;
mod r#type;

use syntax::Visitor;

use crate::module::Module;

impl Visitor for Module {}
