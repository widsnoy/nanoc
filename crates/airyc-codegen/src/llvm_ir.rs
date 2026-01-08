use std::collections::HashMap;

use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{builder::Builder, context::Context};
use airyc_analyzer::array::{ArrayTree, ArrayTreeValue};
use airyc_analyzer::r#type::NType;
use airyc_parser::ast::*;
use airyc_parser::syntax_kind::SyntaxKind;

use crate::utils::*;

pub struct Program<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a inkwell::module::Module<'ctx>,

    pub analyzer: &'a airyc_analyzer::module::Module,

    /// 函数/变量环境
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
    pub fn compile_comp_unit(&mut self, node: CompUnit) {
        self.declare_sysy_runtime();
        for global in node.global_decls() {
            match global {
                GlobalDecl::Decl(decl) => self.compile_global_decl(decl),
                GlobalDecl::FuncDef(func) => self.compile_func_def(func),
            }
        }
    }

    fn compile_global_decl(&mut self, decl: Decl) {
        match decl {
            Decl::ConstDecl(c) => self.compile_const_decl(c),
            Decl::VarDecl(v) => self.compile_var_decl(v),
        }
    }

    fn compile_const_decl(&mut self, decl: ConstDecl) {
        for def in decl.const_defs() {
            self.compile_const_def(def);
        }
    }

    fn compile_const_def(&mut self, def: ConstDef) {
        let name_token = get_ident_node(&def.const_index_val().unwrap());
        let name = name_token.text();

        let var = self.analyzer.get_varaible(name_token.text_range()).unwrap();

        let var_ty: &'a NType = &var.ty;
        let basic_ty = self.convert_ntype_to_type(var_ty);
        let init_node = def.init().unwrap();
        let value = self.compile_const_init_val(init_node, basic_ty);

        if self.current_function.is_none() {
            // global 常量
            let global = self.module.add_global(basic_ty, None, name);
            global.set_initializer(&value);
            global.set_constant(true);
            self.globals.insert(
                name.to_string(),
                Symbol::new(global.as_pointer_value(), &var.ty),
            );
        } else {
            // local 变量
            let func = self.current_function.unwrap();
            let alloca = self.create_entry_alloca(func, basic_ty, name);
            self.builder
                .build_store(alloca, value)
                .expect("存储 const 失败");
            self.insert_var(name.to_string(), alloca, var_ty);
        }
    }

    fn compile_const_init_val(
        &mut self,
        init: ConstInitVal,
        ty: BasicTypeEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        if let Some(expr) = init.expr() {
            return self.get_const_var_value(&expr);
        }
        let range = init.syntax().text_range();
        let array_tree = self
            .analyzer
            .expand_array
            .get(&range)
            .expect("array type, must have an array tree");
        self.convert_array_tree_to_const_value(array_tree, ty)
    }

    fn compile_var_decl(&mut self, decl: VarDecl) {
        for def in decl.var_defs() {
            self.compile_var_def(def);
        }
    }

    fn compile_var_def(&mut self, def: VarDef) {
        let name_token = get_ident_node(&def.const_index_val().unwrap());

        let var = self.analyzer.get_varaible(name_token.text_range()).unwrap();
        let name = name_token.text();
        let var_ty = &var.ty;
        let basic_ty = self.convert_ntype_to_type(var_ty);

        let is_global_var = self.current_function.is_none();
        if is_global_var {
            let init_val = self.const_init_or_zero(def.init(), basic_ty);
            let global = self.module.add_global(basic_ty, None, name);
            global.set_initializer(&init_val);
            self.globals.insert(
                name.to_string(),
                Symbol::new(global.as_pointer_value(), var_ty),
            );
        } else {
            let (init_val, array_tree) = if let Some(init_node) = def.init() {
                if let Some(expr) = init_node.expr() {
                    (Some(self.compile_expr(expr)), None)
                } else {
                    // 否则是初始化列表
                    let range = init_node.syntax().text_range();
                    let array_tree = self.analyzer.expand_array.get(&range).unwrap();
                    // 如果是 constant，直接一整块初始化
                    if self.analyzer.is_constant(range) {
                        (
                            Some(self.convert_array_tree_to_const_value(array_tree, basic_ty)),
                            None,
                        )
                    } else {
                        (None, Some(array_tree))
                    }
                }
            } else {
                (Some(basic_ty.const_zero()), None)
            };

            // 局部变量
            let func = self.current_function.unwrap();
            let alloca = self.create_entry_alloca(func, basic_ty, name);
            if let Some(init_val) = init_val {
                self.builder.build_store(alloca, init_val).unwrap();
            } else {
                let array_tree = array_tree.unwrap();
                // 有运行时的变量，要一个个 load 再 store
                self.builder
                    .build_store(alloca, basic_ty.const_zero())
                    .unwrap();
                let mut indices = vec![self.context.i32_type().const_zero()];
                self.walk_on_array_tree(array_tree, &mut indices, alloca, basic_ty);
            }
            self.insert_var(name.to_string(), alloca, var_ty);
        }
    }

    /// 遍历 ArrayTree 每个叶子，store 初始值
    fn walk_on_array_tree(
        &mut self,
        array_tree: &ArrayTree,
        indices: &mut Vec<IntValue<'ctx>>,
        ptr: PointerValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    ) {
        if let ArrayTree::Val(ArrayTreeValue::Expr(expr)) = array_tree {
            let value = self.compile_expr(expr.clone());
            let gep = unsafe {
                self.builder
                    .build_gep(elem_ty, ptr, indices, "idx.gep")
                    .unwrap()
            };
            self.builder.build_store(gep, value).unwrap();
        } else if let ArrayTree::Children(children) = array_tree {
            let i32_type = self.context.i32_type();
            for (i, child) in children.iter().enumerate() {
                indices.push(i32_type.const_int(i as u64, false));
                self.walk_on_array_tree(child, indices, ptr, elem_ty);
                indices.pop();
            }
        }
    }

    /// 全局变量初始化（没有就是 0）
    fn const_init_or_zero(
        &mut self,
        init: Option<InitVal>,
        ty: BasicTypeEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        // 没有初始化节点
        let Some(init) = init else {
            return ty.const_zero();
        };
        let range = init.syntax().text_range();
        // 有初始化节点，只是 int / float 表达式
        if let Some(value) = self.analyzer.get_value(range) {
            return self.convert_value(value);
        }
        // 是一个数组的初始化列表，因为是全局变量，一定要是 const (analyzer 检查了)
        if let Some(array_tree) = self.analyzer.expand_array.get(&range) {
            return self.convert_array_tree_to_const_value(array_tree, ty);
        }

        panic!("add check in analyzer");
    }

    // 3. Functions
    fn compile_func_def(&mut self, func: FuncDef) {
        let name = func
            .name()
            .and_then(|n| n.ident())
            .map(|t| t.text().to_string())
            .expect("函数缺少名字");

        let (ret_ty, is_void) = func
            .func_type()
            .map(|t| self.compile_func_type(t))
            .unwrap_or((&NType::Int, false));

        let params: Vec<(String, &'a NType)> = func
            .params()
            .map(|ps| {
                ps.params()
                    .map(|p| (name_text(&p.name().unwrap()), self.compile_func_f_param(p)))
                    .collect()
            })
            .unwrap_or_default();

        let basic_params = params
            .iter()
            .map(|(_, p)| self.convert_ntype_to_type(p).into())
            .collect::<Vec<_>>();

        let ret_ty = self.convert_ntype_to_type(ret_ty);
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
            let param_val = function.get_nth_param(i as u32).expect("参数索引越界");
            param_val.set_name(&pname);

            // 如果本来就是引用，就不用再存了
            if param_ty.is_pointer() {
                self.insert_var(pname, param_val.into_pointer_value(), param_ty);
                continue;
            }

            let alloc_ty = param_val.get_type();
            let alloca = self.create_entry_alloca(function, alloc_ty, &pname);
            self.builder
                .build_store(alloca, param_val)
                .expect("参数存储失败");
            self.insert_var(pname, alloca, param_ty);
        }

        if let Some(block) = func.block() {
            self.compile_block(block);
        }

        // 如无显式 return，补一个
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
    }

    fn compile_func_type(&mut self, ty: FuncType) -> (&'a NType, bool) {
        if ty.void_token().is_some() {
            return (&NType::Void, true);
        }
        let base = ty
            .ty()
            .map(|t| self.compile_type(t))
            .expect("函数返回缺类型");
        // let full = apply_pointer(base, ty.pointer());
        (base, false)
    }

    fn compile_func_f_param(&mut self, param: FuncFParam) -> &'a NType {
        let name_token = param.name().and_then(|x| x.ident()).unwrap();
        let variable = self.analyzer.get_varaible(name_token.text_range()).unwrap();
        &variable.ty
    }

    // 4. Block & Statements
    fn compile_block(&mut self, block: Block) {
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
                BlockItem::Decl(decl) => self.compile_local_decl(decl),
                BlockItem::Stmt(stmt) => self.compile_stmt(stmt),
            }

            // 如果有跳转或者终止指令，后面的扔掉
            if is_terminal {
                break;
            }
        }
        self.pop_scope();
    }

    fn compile_local_decl(&mut self, decl: Decl) {
        match decl {
            Decl::ConstDecl(c) => self.compile_const_decl(c),
            Decl::VarDecl(v) => self.compile_var_decl(v),
        }
    }

    fn compile_stmt(&mut self, stmt: Stmt) {
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

    fn compile_assign_stmt(&mut self, stmt: AssignStmt) {
        let rhs = stmt
            .rhs()
            .map(|e| self.compile_expr(e))
            .expect("赋值缺右值");

        let lhs_node = stmt.lhs().unwrap();

        let lhs_ptr = match lhs_node {
            LVal::IndexVal(index_val) => {
                let (_, ptr, _) = self.get_element_ptr_by_index_val(&index_val);
                ptr
            }
            LVal::DerefExpr(_) => todo!(),
        };

        self.builder
            .build_store(lhs_ptr, rhs)
            .expect("赋值存储失败");
    }

    fn compile_expr_stmt(&mut self, stmt: ExprStmt) {
        if let Some(expr) = stmt.expr() {
            self.compile_expr(expr);
        }
    }

    fn compile_if_stmt(&mut self, stmt: IfStmt) {
        let cond_val = stmt
            .condition()
            .map(|e| self.compile_expr(e))
            .expect("if 条件缺失");
        let func = self.current_function.expect("if 无当前函数");

        let then_bb = self.context.append_basic_block(func, "then");
        let else_bb = self.context.append_basic_block(func, "else");
        let merge_bb = self.context.append_basic_block(func, "merge");

        let bool_val = self.as_bool(cond_val);
        self.builder
            .build_conditional_branch(bool_val, then_bb, else_bb)
            .expect("if 跳转失败");

        // then
        self.builder.position_at_end(then_bb);
        if let Some(t) = stmt.then_branch() {
            self.compile_stmt(t);
        }
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(merge_bb)
                .expect("then 跳转失败");
        }

        // else
        self.builder.position_at_end(else_bb);
        if let Some(e) = stmt.else_branch() {
            self.compile_stmt(e);
        }
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(merge_bb)
                .expect("else 跳转失败");
        }

        // merge
        self.builder.position_at_end(merge_bb);
    }

    fn compile_while_stmt(&mut self, stmt: WhileStmt) {
        let func = self.current_function.unwrap();
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let end_bb = self.context.append_basic_block(func, "while.end");

        self.push_loop(cond_bb, end_bb);

        self.builder
            .build_unconditional_branch(cond_bb)
            .expect("while 入口跳转失败");

        // 条件
        self.builder.position_at_end(cond_bb);
        let cond_val = stmt
            .condition()
            .map(|e| self.compile_expr(e))
            .expect("while 条件缺失");
        let bool_val = self.as_bool(cond_val);
        self.builder
            .build_conditional_branch(bool_val, body_bb, end_bb)
            .expect("while 条件跳转失败");

        // 主体
        self.builder.position_at_end(body_bb);
        if let Some(body) = stmt.body() {
            self.compile_stmt(body);
        }
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_bb)
                .expect("while 回跳失败");
        }
        self.pop_loop();
        self.builder.position_at_end(end_bb);
    }

    fn compile_break_stmt(&mut self, _stmt: BreakStmt) {
        let end_bb = self.loop_stack.last().expect("break 不在循环内").end_bb;
        self.builder.build_unconditional_branch(end_bb).unwrap();
    }

    fn compile_continue_stmt(&mut self, _stmt: ContinueStmt) {
        let cond_bb = self.loop_stack.last().expect("continue 不在循环内").cond_bb;
        self.builder.build_unconditional_branch(cond_bb).unwrap();
    }

    fn compile_return_stmt(&mut self, stmt: ReturnStmt) {
        if let Some(expr) = stmt.expr() {
            let val = self.compile_expr(expr).into_int_value(); // fixme
            self.builder.build_return(Some(&val)).ok();
        } else {
            self.builder.build_return(None).ok();
        }
    }

    pub(crate) fn compile_expr(&mut self, expr: Expr) -> BasicValueEnum<'ctx> {
        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::DerefExpr(_e) => todo!(),
            Expr::IndexVal(e) => self.compile_index_val(e, false),
            Expr::Literal(e) => self.compile_literal(e),
        }
    }

    /// fixme: woria
    fn compile_expr_func_call(&mut self, expr: Expr) -> BasicValueEnum<'ctx> {
        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::DerefExpr(_e) => todo!(),
            Expr::IndexVal(e) => self.compile_index_val(e, true),
            Expr::Literal(e) => self.compile_literal(e),
        }
    }
    fn compile_binary_expr(&mut self, expr: BinaryExpr) -> BasicValueEnum<'ctx> {
        use inkwell::FloatPredicate;
        use inkwell::IntPredicate;

        let op_token = expr.op().unwrap().op();

        if let Some(func) = self.current_function
            && matches!(op_token.kind(), SyntaxKind::AMPAMP | SyntaxKind::PIPEPIPE)
        {
            let i32_zero = self.context.i32_type().const_zero();

            let rhs_bb = self.context.append_basic_block(func, "land.rhs");
            let merge_bb = self.context.append_basic_block(func, "land.phi");

            let lhs = expr.lhs().map(|e| self.compile_expr(e)).unwrap();
            let lhs = lhs.into_int_value();

            let lhs_bb = self.builder.get_insert_block().unwrap();
            let eq_zero = self
                .builder
                .build_int_compare(IntPredicate::EQ, lhs, i32_zero, "land.i32_eq_0")
                .unwrap();
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
            let rhs = expr.rhs().map(|e| self.compile_expr(e)).unwrap();
            let rhs_val = self.as_bool(rhs);
            let rhs_val = self.bool_to_i32(rhs_val);
            let rhs_end_bb = self.builder.get_insert_block().unwrap();
            let _ = self.builder.build_unconditional_branch(merge_bb);

            self.builder.position_at_end(merge_bb);
            let merge = self
                .builder
                .build_phi(self.context.i32_type(), "land.phi")
                .unwrap();

            merge.add_incoming(&[(&short_circuit_val, lhs_bb), (&rhs_val, rhs_end_bb)]);
            return merge.as_basic_value();
        }

        let lhs = expr
            .lhs()
            .map(|e| self.compile_expr(e))
            .expect("二元左值缺失");

        let rhs = expr
            .rhs()
            .map(|e| self.compile_expr(e))
            .expect("二元右值缺失");

        match (lhs, rhs) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let res = match op_token.kind() {
                    SyntaxKind::PLUS => self.builder.build_int_add(l, r, "add").unwrap(),
                    SyntaxKind::MINUS => self.builder.build_int_sub(l, r, "sub").unwrap(),
                    SyntaxKind::STAR => self.builder.build_int_mul(l, r, "mul").unwrap(),
                    SyntaxKind::SLASH => self.builder.build_int_signed_div(l, r, "div").unwrap(),
                    SyntaxKind::PERCENT => self.builder.build_int_signed_rem(l, r, "rem").unwrap(),
                    SyntaxKind::LT => {
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, l, r, "lt")
                            .unwrap();
                        self.bool_to_i32(cmp)
                    }
                    SyntaxKind::GT => {
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, l, r, "gt")
                            .unwrap();
                        self.bool_to_i32(cmp)
                    }
                    SyntaxKind::LTEQ => {
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::SLE, l, r, "le")
                            .unwrap();
                        self.bool_to_i32(cmp)
                    }
                    SyntaxKind::GTEQ => {
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, l, r, "ge")
                            .unwrap();
                        self.bool_to_i32(cmp)
                    }
                    SyntaxKind::EQEQ => {
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, l, r, "eq")
                            .unwrap();
                        self.bool_to_i32(cmp)
                    }
                    SyntaxKind::NEQ => {
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::NE, l, r, "ne")
                            .unwrap();
                        self.bool_to_i32(cmp)
                    }
                    SyntaxKind::AMPAMP => {
                        let lb = self.as_bool(l.into());
                        let rb = self.as_bool(r.into());
                        let res = self.builder.build_and(lb, rb, "and").unwrap();
                        self.bool_to_i32(res)
                    }
                    SyntaxKind::PIPEPIPE => {
                        let lb = self.as_bool(l.into());
                        let rb = self.as_bool(r.into());
                        let res = self.builder.build_or(lb, rb, "or").unwrap();
                        self.bool_to_i32(res)
                    }
                    _ => panic!("未支持的整型二元操作 {op_token:?}"),
                };
                res.into()
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => match op_token.kind()
            {
                SyntaxKind::PLUS => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                SyntaxKind::MINUS => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                SyntaxKind::STAR => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                SyntaxKind::SLASH => self.builder.build_float_div(l, r, "fdiv").unwrap().into(),
                SyntaxKind::LT => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                SyntaxKind::GT => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                SyntaxKind::LTEQ => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                SyntaxKind::GTEQ => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
                    .into(),
                SyntaxKind::EQEQ => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                SyntaxKind::NEQ => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                _ => panic!("未支持的浮点二元操作"),
            },
            _ => panic!("类型不匹配的二元运算 lhs: {lhs:?} rhs: {rhs:?}"),
        }
    }

    fn compile_unary_expr(&mut self, expr: UnaryExpr) -> BasicValueEnum<'ctx> {
        let op_token = expr.op().unwrap().op();
        let val = expr
            .expr()
            .map(|e| self.compile_expr(e))
            .expect("一元运算缺操作数");

        match val {
            BasicValueEnum::IntValue(i) => match op_token.kind() {
                SyntaxKind::PLUS => i.into(),
                SyntaxKind::MINUS => self.builder.build_int_neg(i, "ineg").unwrap().into(),
                SyntaxKind::BANG => {
                    let b = self.as_bool(val);
                    let nb = self.builder.build_not(b, "lnot").unwrap();
                    self.bool_to_i32(nb).into()
                }
                _ => panic!("未支持的整型一元操作"),
            },
            BasicValueEnum::FloatValue(f) => match op_token.kind() {
                SyntaxKind::PLUS => f.into(),
                SyntaxKind::MINUS => self.builder.build_float_neg(f, "fneg").unwrap().into(),
                _ => panic!("未支持的浮点一元操作"),
            },
            _ => panic!("未支持的操作数类型"),
        }
    }

    fn compile_call_expr(&mut self, expr: CallExpr) -> BasicValueEnum<'ctx> {
        let name = name_text(&expr.name().expect("调用缺函数名"));
        let func = self
            .module
            .get_function(&name)
            .or_else(|| self.functions.get(&name).copied())
            .expect("函数未声明");

        let args: Vec<BasicMetadataValueEnum<'ctx>> = expr
            .args()
            .map(|rps| {
                rps.args()
                    .map(|a| self.compile_expr_func_call(a).into())
                    .collect()
            })
            .unwrap_or_default();

        let call = self
            .builder
            .build_call(func, &args, "call")
            .expect("函数调用失败");
        if func.get_type().get_return_type().is_some() {
            call.try_as_basic_value().unwrap_basic()
        } else {
            self.context.i32_type().const_zero().into()
        }
    }

    fn compile_paren_expr(&mut self, expr: ParenExpr) -> BasicValueEnum<'ctx> {
        expr.expr()
            .map(|e| self.compile_expr(e))
            .expect("括号表达式为空")
    }

    // fn compile_deref_expr(&mut self, expr: DerefExpr) -> BasicValueEnum<'ctx> {
    //     let ptr_val = expr
    //         .expr()
    //         .map(|e| self.compile_expr(e))
    //         .expect("解引用缺操作数");
    //     let ptr = ptr_val.into_pointer_value();
    //     let elem_ty: BasicTypeEnum<'ctx> = self.context.i32_type().into();
    //     self.builder
    //         .build_load(elem_ty, ptr, "deref")
    //         .expect("解引用失败")
    // }

    fn compile_index_val(
        &mut self,
        expr: IndexVal,
        func_call_r_param: bool,
    ) -> BasicValueEnum<'ctx> {
        let (ty, ptr, name) = self.get_element_ptr_by_index_val(&expr);
        if !func_call_r_param || (!ty.is_array_type() && !ty.is_pointer_type()) {
            self.builder.build_load(ty, ptr, &name).unwrap()
        } else {
            ptr.into()
        }
    }

    fn compile_literal(&mut self, expr: Literal) -> BasicValueEnum<'ctx> {
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
            let v = i32::from_str_radix(num_str, radix).unwrap();
            return self.context.i32_type().const_int(v as u64, true).into();
        }
        if let Some(float_token) = expr.float_token() {
            let s = float_token.text().to_string();
            let v: f32 = s.parse().unwrap();
            return self.context.f32_type().const_float(v as f64).into();
        }
        panic!("未知字面量");
    }

    fn _compile_const_index_val(&mut self, _val: ConstIndexVal) {
        todo!();
    }

    /// int, float or struct
    fn compile_type(&mut self, ty: Type) -> &'a NType {
        if ty.int_token().is_some() {
            return &NType::Int;
        }
        if ty.float_token().is_some() {
            return &NType::Float;
        }
        if ty.struct_token().is_some() {
            todo!("暂不支持 struct 类型");
        }
        panic!("未知类型");
    }
}
