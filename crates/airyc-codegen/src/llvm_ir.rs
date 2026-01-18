use std::collections::HashMap;

use airyc_analyzer::array::{ArrayTree, ArrayTreeValue};
use airyc_analyzer::r#type::NType;
use airyc_parser::ast::*;
use airyc_parser::syntax_kind::SyntaxKind;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{builder::Builder, context::Context};

use crate::error::{CodegenError, Result};
use crate::utils::*;

pub struct Program<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a inkwell::module::Module<'ctx>,

    pub analyzer: &'a airyc_analyzer::module::Module,

    /// function/variable environment
    pub current_function: Option<FunctionValue<'ctx>>,
    pub scopes: Vec<HashMap<String, Symbol<'a, 'ctx>>>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub globals: HashMap<String, Symbol<'a, 'ctx>>,

    pub loop_stack: Vec<LoopContext<'ctx>>,
}

#[derive(Clone, Copy)]
pub struct Symbol<'a, 'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: &'a NType,
}
impl<'a, 'ctx> Symbol<'a, 'ctx> {
    pub fn new(ptr: PointerValue<'ctx>, ty: &'a NType) -> Self {
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
                GlobalDecl::Decl(decl) => self.compile_global_decl(decl)?,
                GlobalDecl::FuncDef(func) => self.compile_func_def(func)?,
            }
        }
        Ok(())
    }

    fn compile_global_decl(&mut self, decl: Decl) -> Result<()> {
        match decl {
            Decl::ConstDecl(c) => self.compile_const_decl(c),
            Decl::VarDecl(v) => self.compile_var_decl(v),
        }
    }

    fn compile_const_decl(&mut self, decl: ConstDecl) -> Result<()> {
        for def in decl.const_defs() {
            self.compile_const_def(def)?;
        }
        Ok(())
    }

    fn compile_const_def(&mut self, def: ConstDef) -> Result<()> {
        let name_token = get_ident_node(
            &def.const_index_val()
                .ok_or(CodegenError::Missing("const name"))?,
        )
        .ok_or(CodegenError::Missing("identifier"))?;
        let name = name_token.text();

        let var = self
            .analyzer
            .get_varaible(name_token.text_range())
            .ok_or(CodegenError::Missing("variable info"))?;

        let var_ty: &'a NType = &var.ty;
        let basic_ty = self.convert_ntype_to_type(var_ty)?;
        let init_node = def.init().ok_or(CodegenError::Missing("initial value"))?;
        let value = self.compile_const_init_val(init_node, basic_ty)?;

        if self.current_function.is_none() {
            let global = self.module.add_global(basic_ty, None, name);
            global.set_initializer(&value);
            global.set_constant(true);
            self.globals.insert(
                name.to_string(),
                Symbol::new(global.as_pointer_value(), &var.ty),
            );
        } else {
            let func = self
                .current_function
                .ok_or(CodegenError::Missing("current function"))?;
            let alloca = self.create_entry_alloca(func, basic_ty, name)?;
            self.builder
                .build_store(alloca, value)
                .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            self.insert_var(name.to_string(), alloca, var_ty);
        }
        Ok(())
    }

    fn compile_const_init_val(
        &mut self,
        init: ConstInitVal,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        if let Some(expr) = init.expr() {
            return self.get_const_var_value(&expr);
        }
        let range = init.syntax().text_range();
        let array_tree = self
            .analyzer
            .expand_array
            .get(&range)
            .ok_or(CodegenError::Missing("array init info"))?;
        self.convert_array_tree_to_const_value(array_tree, ty)
    }

    fn compile_var_decl(&mut self, decl: VarDecl) -> Result<()> {
        for def in decl.var_defs() {
            self.compile_var_def(def)?;
        }
        Ok(())
    }

    fn compile_var_def(&mut self, def: VarDef) -> Result<()> {
        let name_token = get_ident_node(
            &def.const_index_val()
                .ok_or(CodegenError::Missing("variable name"))?,
        )
        .ok_or(CodegenError::Missing("identifier"))?;

        let var = self
            .analyzer
            .get_varaible(name_token.text_range())
            .ok_or(CodegenError::Missing("variable info"))?;
        let name = name_token.text();
        let var_ty = &var.ty;
        let basic_ty = self.convert_ntype_to_type(var_ty)?;

        let is_global_var = self.current_function.is_none();
        if is_global_var {
            let init_val = self.const_init_or_zero(def.init(), basic_ty)?;
            let global = self.module.add_global(basic_ty, None, name);
            global.set_initializer(&init_val);
            self.globals.insert(
                name.to_string(),
                Symbol::new(global.as_pointer_value(), var_ty),
            );
        } else {
            let (init_val, array_tree) = if let Some(init_node) = def.init() {
                if let Some(expr) = init_node.expr() {
                    (Some(self.compile_expr(expr)?), None)
                } else {
                    let range = init_node.syntax().text_range();
                    let array_tree = self
                        .analyzer
                        .expand_array
                        .get(&range)
                        .ok_or(CodegenError::Missing("array init info"))?;
                    if self.analyzer.is_constant(range) {
                        (
                            Some(self.convert_array_tree_to_const_value(array_tree, basic_ty)?),
                            None,
                        )
                    } else {
                        (None, Some(array_tree))
                    }
                }
            } else {
                (Some(basic_ty.const_zero()), None)
            };

            let func = self
                .current_function
                .ok_or(CodegenError::Missing("current function"))?;
            let alloca = self.create_entry_alloca(func, basic_ty, name)?;
            if let Some(init_val) = init_val {
                self.builder
                    .build_store(alloca, init_val)
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            } else {
                let array_tree = array_tree.ok_or(CodegenError::Missing("array tree"))?;
                self.builder
                    .build_store(alloca, basic_ty.const_zero())
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                let mut indices = vec![self.context.i32_type().const_zero()];
                self.walk_on_array_tree(array_tree, &mut indices, alloca, basic_ty)?;
            }
            self.insert_var(name.to_string(), alloca, var_ty);
        }
        Ok(())
    }

    /// Walk ArrayTree leaves and store initial values
    fn walk_on_array_tree(
        &mut self,
        array_tree: &ArrayTree,
        indices: &mut Vec<IntValue<'ctx>>,
        ptr: PointerValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    ) -> Result<()> {
        if let ArrayTree::Val(ArrayTreeValue::Expr(expr)) = array_tree {
            let value = self.compile_expr(expr.clone())?;
            let gep = unsafe {
                self.builder
                    .build_gep(elem_ty, ptr, indices, "idx.gep")
                    .map_err(|_| CodegenError::LlvmBuild("gep failed"))?
            };
            self.builder
                .build_store(gep, value)
                .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
        } else if let ArrayTree::Children(children) = array_tree {
            let i32_type = self.context.i32_type();
            for (i, child) in children.iter().enumerate() {
                indices.push(i32_type.const_int(i as u64, false));
                self.walk_on_array_tree(child, indices, ptr, elem_ty)?;
                indices.pop();
            }
        }
        Ok(())
    }

    /// Global variable initialization (default 0)
    fn const_init_or_zero(
        &mut self,
        init: Option<InitVal>,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let Some(init) = init else {
            return Ok(ty.const_zero());
        };
        let range = init.syntax().text_range();
        if let Some(value) = self.analyzer.get_value(range) {
            return self.convert_value(value);
        }
        if let Some(array_tree) = self.analyzer.expand_array.get(&range) {
            return self.convert_array_tree_to_const_value(array_tree, ty);
        }
        Err(CodegenError::Missing("init value"))
    }

    // 3. Functions
    fn compile_func_def(&mut self, func: FuncDef) -> Result<()> {
        let name = name_text(&func.name().ok_or(CodegenError::Missing("function name"))?)
            .ok_or(CodegenError::Missing("identifier"))?;

        let (ret_ty, is_void) = func
            .func_type()
            .map(|t| self.compile_func_type(t))
            .transpose()?
            .unwrap_or((&NType::Int, false));

        let params: Vec<(String, &'a NType)> = func
            .params()
            .map(|ps| {
                ps.params()
                    .map(|p| -> Result<_> {
                        Ok((
                            name_text(&p.name().ok_or(CodegenError::Missing("param name"))?)
                                .ok_or(CodegenError::Missing("identifier"))?,
                            self.compile_func_f_param(p)?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();

        let basic_params = params
            .iter()
            .map(|(_, p)| self.convert_ntype_to_type(p).map(|t| t.into()))
            .collect::<Result<Vec<_>>>()?;

        let ret_ty = self.convert_ntype_to_type(ret_ty)?;
        let fn_type = if is_void {
            self.context.void_type().fn_type(&basic_params, false)
        } else {
            ret_ty.fn_type(&basic_params, false)
        };

        let function = self.module.add_function(&name, fn_type, None);
        self.functions.insert(name.clone(), function);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let prev_func = self.current_function;
        self.current_function = Some(function);
        self.push_scope();

        for (i, (pname, param_ty)) in params.into_iter().enumerate() {
            let param_val = function
                .get_nth_param(i as u32)
                .ok_or(CodegenError::Missing("parameter"))?;
            param_val.set_name(&pname);

            if param_ty.is_pointer() {
                self.insert_var(pname, param_val.into_pointer_value(), param_ty);
                continue;
            }

            let alloc_ty = param_val.get_type();
            let alloca = self.create_entry_alloca(function, alloc_ty, &pname)?;
            self.builder
                .build_store(alloca, param_val)
                .map_err(|_| CodegenError::LlvmBuild("parameterstore failed"))?;
            self.insert_var(pname, alloca, param_ty);
        }

        if let Some(block) = func.block() {
            self.compile_block(block)?;
        }

        let has_term = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_some();
        if !has_term {
            if is_void {
                self.builder.build_return(None).ok();
            } else {
                let zero = ret_ty.const_zero();
                self.builder.build_return(Some(&zero)).ok();
            }
        }

        self.pop_scope();
        self.current_function = prev_func;
        Ok(())
    }

    fn compile_func_type(&mut self, ty: FuncType) -> Result<(&'a NType, bool)> {
        if ty.void_token().is_some() {
            return Ok((&NType::Void, true));
        }
        let base = ty
            .ty()
            .map(|t| self.compile_type(t))
            .transpose()?
            .ok_or(CodegenError::Missing("return type"))?;
        Ok((base, false))
    }

    fn compile_func_f_param(&mut self, param: FuncFParam) -> Result<&'a NType> {
        let name_token = param
            .name()
            .and_then(|x| x.ident())
            .ok_or(CodegenError::Missing("param name"))?;
        let variable = self
            .analyzer
            .get_varaible(name_token.text_range())
            .ok_or(CodegenError::Missing("param info"))?;
        Ok(&variable.ty)
    }

    // 4. Block & Statements
    fn compile_block(&mut self, block: Block) -> Result<()> {
        self.push_scope();
        for item in block.items() {
            let is_terminal = if let BlockItem::Stmt(ref stmt) = item
                && matches!(
                    stmt,
                    Stmt::BreakStmt(_) | Stmt::ContinueStmt(_) | Stmt::ReturnStmt(_)
                ) {
                true
            } else {
                false
            };
            match item {
                BlockItem::Decl(decl) => self.compile_local_decl(decl)?,
                BlockItem::Stmt(stmt) => self.compile_stmt(stmt)?,
            }

            if is_terminal {
                break;
            }
        }
        self.pop_scope();
        Ok(())
    }

    fn compile_local_decl(&mut self, decl: Decl) -> Result<()> {
        match decl {
            Decl::ConstDecl(c) => self.compile_const_decl(c),
            Decl::VarDecl(v) => self.compile_var_decl(v),
        }
    }

    fn compile_stmt(&mut self, stmt: Stmt) -> Result<()> {
        match stmt {
            Stmt::AssignStmt(s) => self.compile_assign_stmt(s),
            Stmt::ExprStmt(s) => self.compile_expr_stmt(s),
            Stmt::Block(s) => self.compile_block(s),
            Stmt::IfStmt(s) => self.compile_if_stmt(s),
            Stmt::WhileStmt(s) => self.compile_while_stmt(s),
            Stmt::BreakStmt(s) => self.compile_break_stmt(s),
            Stmt::ContinueStmt(s) => self.compile_continue_stmt(s),
            Stmt::ReturnStmt(s) => self.compile_return_stmt(s),
        }
    }

    fn compile_assign_stmt(&mut self, stmt: AssignStmt) -> Result<()> {
        let rhs = self.compile_expr(stmt.rhs().ok_or(CodegenError::Missing("assign rhs"))?)?;
        let lhs_node = stmt.lhs().ok_or(CodegenError::Missing("assign lhs"))?;

        let lhs_ptr = match lhs_node {
            LVal::IndexVal(index_val) => {
                let (_, ptr, _) = self.get_element_ptr_by_index_val(&index_val)?;
                ptr
            }
            LVal::DerefExpr(_) => return Err(CodegenError::NotImplemented("deref assign")),
        };

        self.builder
            .build_store(lhs_ptr, rhs)
            .map_err(|_| CodegenError::LlvmBuild("assign store failed"))?;
        Ok(())
    }

    fn compile_expr_stmt(&mut self, stmt: ExprStmt) -> Result<()> {
        if let Some(expr) = stmt.expr() {
            self.compile_expr(expr)?;
        }
        Ok(())
    }

    fn compile_if_stmt(&mut self, stmt: IfStmt) -> Result<()> {
        let cond_val = self.compile_expr(
            stmt.condition()
                .ok_or(CodegenError::Missing("if condition"))?,
        )?;
        let func = self
            .current_function
            .ok_or(CodegenError::Missing("current function"))?;

        let then_bb = self.context.append_basic_block(func, "then");
        let else_bb = self.context.append_basic_block(func, "else");
        let merge_bb = self.context.append_basic_block(func, "merge");

        let bool_val = self.as_bool(cond_val)?;
        self.builder
            .build_conditional_branch(bool_val, then_bb, else_bb)
            .map_err(|_| CodegenError::LlvmBuild("if branch failed"))?;

        self.builder.position_at_end(then_bb);
        if let Some(t) = stmt.then_branch() {
            self.compile_stmt(t)?;
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::LlvmBuild("then branch failed"))?;
        }

        self.builder.position_at_end(else_bb);
        if let Some(e) = stmt.else_branch() {
            self.compile_stmt(e)?;
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::LlvmBuild("else branch failed"))?;
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    fn compile_while_stmt(&mut self, stmt: WhileStmt) -> Result<()> {
        let func = self
            .current_function
            .ok_or(CodegenError::Missing("current function"))?;
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let end_bb = self.context.append_basic_block(func, "while.end");

        self.push_loop(cond_bb, end_bb);

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::LlvmBuild("while entry branch failed"))?;

        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expr(
            stmt.condition()
                .ok_or(CodegenError::Missing("while condition"))?,
        )?;
        let bool_val = self.as_bool(cond_val)?;
        self.builder
            .build_conditional_branch(bool_val, body_bb, end_bb)
            .map_err(|_| CodegenError::LlvmBuild("while cond branch failed"))?;

        self.builder.position_at_end(body_bb);
        if let Some(body) = stmt.body() {
            self.compile_stmt(body)?;
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::LlvmBuild("while back branch failed"))?;
        }
        self.pop_loop();
        self.builder.position_at_end(end_bb);
        Ok(())
    }

    fn compile_break_stmt(&mut self, _stmt: BreakStmt) -> Result<()> {
        let end_bb = self
            .loop_stack
            .last()
            .ok_or(CodegenError::Unsupported("break not in loop".into()))?
            .end_bb;
        self.builder
            .build_unconditional_branch(end_bb)
            .map_err(|_| CodegenError::LlvmBuild("break branch failed"))?;
        Ok(())
    }

    fn compile_continue_stmt(&mut self, _stmt: ContinueStmt) -> Result<()> {
        let cond_bb = self
            .loop_stack
            .last()
            .ok_or(CodegenError::Unsupported("continue not in loop".into()))?
            .cond_bb;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::LlvmBuild("continue branch failed"))?;
        Ok(())
    }

    fn compile_return_stmt(&mut self, stmt: ReturnStmt) -> Result<()> {
        if let Some(expr) = stmt.expr() {
            let val = self.compile_expr(expr)?.into_int_value();
            self.builder.build_return(Some(&val)).ok();
        } else {
            self.builder.build_return(None).ok();
        }
        Ok(())
    }

    pub(crate) fn compile_expr(&mut self, expr: Expr) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::DerefExpr(_e) => Err(CodegenError::NotImplemented("deref expression")),
            Expr::IndexVal(e) => self.compile_index_val(e, false),
            Expr::Literal(e) => self.compile_literal(e),
        }
    }

    fn compile_expr_func_call(&mut self, expr: Expr) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::DerefExpr(_e) => Err(CodegenError::NotImplemented("deref expression")),
            Expr::IndexVal(e) => self.compile_index_val(e, true),
            Expr::Literal(e) => self.compile_literal(e),
        }
    }
    fn compile_binary_expr(&mut self, expr: BinaryExpr) -> Result<BasicValueEnum<'ctx>> {
        use inkwell::FloatPredicate;
        use inkwell::IntPredicate;

        let op_token = expr
            .op()
            .ok_or(CodegenError::Missing("binary operator"))?
            .op();

        if let Some(func) = self.current_function
            && matches!(op_token.kind(), SyntaxKind::AMPAMP | SyntaxKind::PIPEPIPE)
        {
            let i32_zero = self.context.i32_type().const_zero();
            let rhs_bb = self.context.append_basic_block(func, "land.rhs");
            let merge_bb = self.context.append_basic_block(func, "land.phi");

            let lhs =
                self.compile_expr(expr.lhs().ok_or(CodegenError::Missing("left operand"))?)?;
            let lhs = lhs.into_int_value();

            let lhs_bb = self
                .builder
                .get_insert_block()
                .ok_or(CodegenError::LlvmBuild("no current basic block"))?;
            let eq_zero = self
                .builder
                .build_int_compare(IntPredicate::EQ, lhs, i32_zero, "land.i32_eq_0")
                .map_err(|_| CodegenError::LlvmBuild("int compare failed"))?;
            let short_circuit_val = if op_token.kind() == SyntaxKind::AMPAMP {
                let _ = self
                    .builder
                    .build_conditional_branch(eq_zero, merge_bb, rhs_bb);
                i32_zero
            } else {
                let _ = self
                    .builder
                    .build_conditional_branch(eq_zero, rhs_bb, merge_bb);
                self.context.i32_type().const_int(1, false)
            };

            self.builder.position_at_end(rhs_bb);
            let rhs =
                self.compile_expr(expr.rhs().ok_or(CodegenError::Missing("right operand"))?)?;
            let rhs_val = self.as_bool(rhs)?;
            let rhs_val = self.bool_to_i32(rhs_val)?;
            let rhs_end_bb = self
                .builder
                .get_insert_block()
                .ok_or(CodegenError::LlvmBuild("no current basic block"))?;
            let _ = self.builder.build_unconditional_branch(merge_bb);

            self.builder.position_at_end(merge_bb);
            let merge = self
                .builder
                .build_phi(self.context.i32_type(), "land.phi")
                .map_err(|_| CodegenError::LlvmBuild("phi build failed"))?;

            merge.add_incoming(&[(&short_circuit_val, lhs_bb), (&rhs_val, rhs_end_bb)]);
            return Ok(merge.as_basic_value());
        }

        let lhs = self.compile_expr(expr.lhs().ok_or(CodegenError::Missing("left operand"))?)?;
        let rhs = self.compile_expr(expr.rhs().ok_or(CodegenError::Missing("right operand"))?)?;

        match (lhs, rhs) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let res = match op_token.kind() {
                    SyntaxKind::PLUS => self
                        .builder
                        .build_int_add(l, r, "add")
                        .map_err(|_| CodegenError::LlvmBuild("int add"))?,
                    SyntaxKind::MINUS => self
                        .builder
                        .build_int_sub(l, r, "sub")
                        .map_err(|_| CodegenError::LlvmBuild("int sub"))?,
                    SyntaxKind::STAR => self
                        .builder
                        .build_int_mul(l, r, "mul")
                        .map_err(|_| CodegenError::LlvmBuild("int mul"))?,
                    SyntaxKind::SLASH => self
                        .builder
                        .build_int_signed_div(l, r, "div")
                        .map_err(|_| CodegenError::LlvmBuild("int div"))?,
                    SyntaxKind::PERCENT => self
                        .builder
                        .build_int_signed_rem(l, r, "rem")
                        .map_err(|_| CodegenError::LlvmBuild("int rem"))?,
                    SyntaxKind::LT => self.bool_to_i32(
                        self.builder
                            .build_int_compare(IntPredicate::SLT, l, r, "lt")
                            .map_err(|_| CodegenError::LlvmBuild("cmp"))?,
                    )?,
                    SyntaxKind::GT => self.bool_to_i32(
                        self.builder
                            .build_int_compare(IntPredicate::SGT, l, r, "gt")
                            .map_err(|_| CodegenError::LlvmBuild("cmp"))?,
                    )?,
                    SyntaxKind::LTEQ => self.bool_to_i32(
                        self.builder
                            .build_int_compare(IntPredicate::SLE, l, r, "le")
                            .map_err(|_| CodegenError::LlvmBuild("cmp"))?,
                    )?,
                    SyntaxKind::GTEQ => self.bool_to_i32(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, l, r, "ge")
                            .map_err(|_| CodegenError::LlvmBuild("cmp"))?,
                    )?,
                    SyntaxKind::EQEQ => self.bool_to_i32(
                        self.builder
                            .build_int_compare(IntPredicate::EQ, l, r, "eq")
                            .map_err(|_| CodegenError::LlvmBuild("cmp"))?,
                    )?,
                    SyntaxKind::NEQ => self.bool_to_i32(
                        self.builder
                            .build_int_compare(IntPredicate::NE, l, r, "ne")
                            .map_err(|_| CodegenError::LlvmBuild("cmp"))?,
                    )?,
                    SyntaxKind::AMPAMP => {
                        let lb = self.as_bool(l.into())?;
                        let rb = self.as_bool(r.into())?;
                        self.bool_to_i32(
                            self.builder
                                .build_and(lb, rb, "and")
                                .map_err(|_| CodegenError::LlvmBuild("and"))?,
                        )?
                    }
                    SyntaxKind::PIPEPIPE => {
                        let lb = self.as_bool(l.into())?;
                        let rb = self.as_bool(r.into())?;
                        self.bool_to_i32(
                            self.builder
                                .build_or(lb, rb, "or")
                                .map_err(|_| CodegenError::LlvmBuild("or"))?,
                        )?
                    }
                    _ => {
                        return Err(CodegenError::Unsupported(format!(
                            "int binary op {:?}",
                            op_token
                        )));
                    }
                };
                Ok(res.into())
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                let res: BasicValueEnum = match op_token.kind() {
                    SyntaxKind::PLUS => self
                        .builder
                        .build_float_add(l, r, "fadd")
                        .map_err(|_| CodegenError::LlvmBuild("fadd"))?
                        .into(),
                    SyntaxKind::MINUS => self
                        .builder
                        .build_float_sub(l, r, "fsub")
                        .map_err(|_| CodegenError::LlvmBuild("fsub"))?
                        .into(),
                    SyntaxKind::STAR => self
                        .builder
                        .build_float_mul(l, r, "fmul")
                        .map_err(|_| CodegenError::LlvmBuild("fmul"))?
                        .into(),
                    SyntaxKind::SLASH => self
                        .builder
                        .build_float_div(l, r, "fdiv")
                        .map_err(|_| CodegenError::LlvmBuild("fdiv"))?
                        .into(),
                    SyntaxKind::LT => self
                        .builder
                        .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                        .map_err(|_| CodegenError::LlvmBuild("fcmp"))?
                        .into(),
                    SyntaxKind::GT => self
                        .builder
                        .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                        .map_err(|_| CodegenError::LlvmBuild("fcmp"))?
                        .into(),
                    SyntaxKind::LTEQ => self
                        .builder
                        .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                        .map_err(|_| CodegenError::LlvmBuild("fcmp"))?
                        .into(),
                    SyntaxKind::GTEQ => self
                        .builder
                        .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                        .map_err(|_| CodegenError::LlvmBuild("fcmp"))?
                        .into(),
                    SyntaxKind::EQEQ => self
                        .builder
                        .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                        .map_err(|_| CodegenError::LlvmBuild("fcmp"))?
                        .into(),
                    SyntaxKind::NEQ => self
                        .builder
                        .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                        .map_err(|_| CodegenError::LlvmBuild("fcmp"))?
                        .into(),
                    _ => return Err(CodegenError::Unsupported("float binary op".into())),
                };
                Ok(res)
            }
            _ => Err(CodegenError::TypeMismatch(format!(
                "binary op lhs: {:?} rhs: {:?}",
                lhs, rhs
            ))),
        }
    }

    fn compile_unary_expr(&mut self, expr: UnaryExpr) -> Result<BasicValueEnum<'ctx>> {
        let op_token = expr
            .op()
            .ok_or(CodegenError::Missing("unary operator"))?
            .op();
        let val = self.compile_expr(expr.expr().ok_or(CodegenError::Missing("unary operand"))?)?;

        match val {
            BasicValueEnum::IntValue(i) => match op_token.kind() {
                SyntaxKind::PLUS => Ok(i.into()),
                SyntaxKind::MINUS => Ok(self
                    .builder
                    .build_int_neg(i, "ineg")
                    .map_err(|_| CodegenError::LlvmBuild("int neg"))?
                    .into()),
                SyntaxKind::BANG => {
                    let b = self.as_bool(val)?;
                    let nb = self
                        .builder
                        .build_not(b, "lnot")
                        .map_err(|_| CodegenError::LlvmBuild("not"))?;
                    Ok(self.bool_to_i32(nb)?.into())
                }
                _ => Err(CodegenError::Unsupported("int unary op".into())),
            },
            BasicValueEnum::FloatValue(f) => match op_token.kind() {
                SyntaxKind::PLUS => Ok(f.into()),
                SyntaxKind::MINUS => Ok(self
                    .builder
                    .build_float_neg(f, "fneg")
                    .map_err(|_| CodegenError::LlvmBuild("float neg"))?
                    .into()),
                _ => Err(CodegenError::Unsupported("float unary op".into())),
            },
            _ => Err(CodegenError::Unsupported("operand type".into())),
        }
    }

    fn compile_call_expr(&mut self, expr: CallExpr) -> Result<BasicValueEnum<'ctx>> {
        let name = name_text(&expr.name().ok_or(CodegenError::Missing("function name"))?)
            .ok_or(CodegenError::Missing("identifier"))?;
        let func = self
            .module
            .get_function(&name)
            .or_else(|| self.functions.get(&name).copied())
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;

        let args: Vec<BasicMetadataValueEnum<'ctx>> = expr
            .args()
            .map(|rps| {
                rps.args()
                    .map(|a| self.compile_expr_func_call(a).map(|v| v.into()))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();

        let call = self
            .builder
            .build_call(func, &args, "call")
            .map_err(|_| CodegenError::LlvmBuild("function call"))?;
        if func.get_type().get_return_type().is_some() {
            Ok(call.try_as_basic_value().unwrap_basic())
        } else {
            Ok(self.context.i32_type().const_zero().into())
        }
    }

    fn compile_paren_expr(&mut self, expr: ParenExpr) -> Result<BasicValueEnum<'ctx>> {
        self.compile_expr(
            expr.expr()
                .ok_or(CodegenError::Missing("paren expression"))?,
        )
    }

    fn compile_index_val(
        &mut self,
        expr: IndexVal,
        func_call_r_param: bool,
    ) -> Result<BasicValueEnum<'ctx>> {
        let (ty, ptr, name) = self.get_element_ptr_by_index_val(&expr)?;
        if !func_call_r_param || (!ty.is_array_type() && !ty.is_pointer_type()) {
            self.builder
                .build_load(ty, ptr, &name)
                .map_err(|_| CodegenError::LlvmBuild("load"))
        } else {
            Ok(ptr.into())
        }
    }

    fn compile_literal(&mut self, expr: Literal) -> Result<BasicValueEnum<'ctx>> {
        if let Some(int_token) = expr.int_token() {
            let s = int_token.text().to_string();
            let (num_str, radix) = match s.chars().next() {
                Some('0') => match s.chars().nth(1) {
                    Some('x') | Some('X') => (&s[2..], 16),
                    Some(_) => (&s[1..], 8),
                    None => (&s[..], 10),
                },
                _ => (&s[..], 10),
            };
            let v = i32::from_str_radix(num_str, radix)
                .map_err(|_| CodegenError::Unsupported(format!("invalid int: {}", s)))?;
            return Ok(self.context.i32_type().const_int(v as u64, true).into());
        }
        if let Some(float_token) = expr.float_token() {
            let s = float_token.text().to_string();
            let v: f32 = s
                .parse()
                .map_err(|_| CodegenError::Unsupported(format!("invalid float: {}", s)))?;
            return Ok(self.context.f32_type().const_float(v as f64).into());
        }
        Err(CodegenError::Unsupported("unknown literal".into()))
    }

    fn compile_type(&mut self, ty: Type) -> Result<&'a NType> {
        if ty.int_token().is_some() {
            return Ok(&NType::Int);
        }
        if ty.float_token().is_some() {
            return Ok(&NType::Float);
        }
        if ty.struct_token().is_some() {
            return Err(CodegenError::NotImplemented("struct type"));
        }
        Err(CodegenError::Unsupported("unknown type".into()))
    }
}
