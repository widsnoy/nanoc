use std::collections::HashMap;

use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{builder::Builder, context::Context, module::Module};
use nanoc_parser::ast::*;
use nanoc_parser::syntax_kind::SyntaxKind;

use crate::utils::{
    apply_pointer, as_bool, const_index_dims, const_name, name_text, wrap_array_dims,
};

pub struct Program<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,

    /// 函数/变量环境
    pub current_function: Option<FunctionValue<'ctx>>,
    pub scopes: Vec<HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub globals: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,

    pub loop_stack: Vec<LoopContext<'ctx>>,
}

pub struct LoopContext<'ctx> {
    pub cond_bb: BasicBlock<'ctx>,
    pub end_bb: BasicBlock<'ctx>,
}

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// 新作用域
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 离开作用域
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn push_loop(&mut self, cond_bb: BasicBlock<'ctx>, end_bb: BasicBlock<'ctx>) {
        self.loop_stack.push(LoopContext { cond_bb, end_bb });
    }

    fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// 插入局部变量
    fn insert_var(&mut self, name: String, ptr: PointerValue<'ctx>, ty: BasicTypeEnum<'ctx>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, (ptr, ty));
        }
    }

    /// 查找变量（从内到外）
    fn lookup_var(&self, name: &str) -> Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        for scope in self.scopes.iter().rev() {
            if let Some(p) = scope.get(name) {
                return Some(*p);
            }
        }
        if let Some(g) = self.globals.get(name) {
            return Some(*g);
        }
        None
    }

    /// 在函数入口分配局部
    fn create_entry_alloca(
        &self,
        function: FunctionValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> PointerValue<'ctx> {
        let entry = function
            .get_first_basic_block()
            .expect("函数缺少入口基本块");
        let builder = self.context.create_builder();
        if let Some(instr) = entry.get_first_instruction() {
            builder.position_before(&instr);
        } else {
            builder.position_at_end(entry);
        }
        builder.build_alloca(ty, name).expect("创建 alloca 失败")
    }

    pub fn compile_comp_unit(&mut self, node: CompUnit) {
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
        let base_ty = decl
            .ty()
            .map(|t| self.compile_type(t))
            .expect("const 没有类型");

        for def in decl.const_defs() {
            let name_node = def.name().expect("const 缺名字");
            let name = const_name(&name_node);
            let dims = const_index_dims(&name_node).unwrap_or_default();
            let arr_ty = wrap_array_dims(base_ty, &dims);
            let full_ty = apply_pointer(arr_ty, def.pointer());
            let init = def.init();
            let value = init
                .and_then(|i| self.compile_const_init_val(i, full_ty))
                .unwrap_or_else(|| full_ty.const_zero());

            if self.current_function.is_none() {
                // global 常量
                let global = self.module.add_global(full_ty, None, &name);
                global.set_initializer(&value);
                global.set_constant(true);
                self.globals
                    .insert(name.clone(), (global.as_pointer_value(), full_ty));
            } else {
                // local 变量
                let func = self.current_function.unwrap();
                let alloca = self.create_entry_alloca(func, full_ty, &name);
                self.builder
                    .build_store(alloca, value)
                    .expect("存储 const 失败");
                self.insert_var(name, alloca, full_ty);
            }
        }
    }

    fn compile_const_init_val(
        &mut self,
        init: ConstInitVal,
        _ty: BasicTypeEnum<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        if let Some(expr) = init.expr() {
            return Some(self.compile_const_expr(expr));
        }
        todo!();
        // // 仅支持简单常量数组的 {} 初始化，复杂场景后续扩展
        // if init.inits().next().is_some() {
        //     Some(ty.const_zero())
        // } else {
        //     None
        // }
    }

    fn compile_var_decl(&mut self, decl: VarDecl) {
        let base_ty = decl
            .ty()
            .map(|t| self.compile_type(t))
            .expect("var 没有类型");

        for def in decl.var_defs() {
            self.compile_var_def(def, base_ty);
        }
    }

    fn compile_var_def(&mut self, def: VarDef, base_ty: BasicTypeEnum<'ctx>) {
        let name = const_name(&def.name().expect("var 没有名字"));
        let dims = const_index_dims(&def.name().expect("var 没有名字")).unwrap_or_default();
        let arr_ty = wrap_array_dims(base_ty, &dims);
        let full_ty = apply_pointer(arr_ty, def.pointer());
        let init = def.init();

        let init_val = if self.current_function.is_none() {
            self.const_init_or_zero(init, full_ty)
        } else {
            init.and_then(|i| self.compile_init_val(i))
                .unwrap_or_else(|| full_ty.const_zero())
        };

        if self.current_function.is_none() {
            // 全局变量
            let global = self.module.add_global(full_ty, None, &name);
            global.set_initializer(&init_val);
            self.globals
                .insert(name.clone(), (global.as_pointer_value(), full_ty));
        } else {
            // 局部变量
            let func = self.current_function.unwrap();
            let alloca = self.create_entry_alloca(func, full_ty, &name);
            self.builder
                .build_store(alloca, init_val)
                .expect("局部初始化失败");
            self.insert_var(name, alloca, full_ty);
        }
    }

    /// 全局变量的安全初始化（非常量退化为 0）
    /// todo: 应该支持对 const 的变量初始化
    fn const_init_or_zero(
        &mut self,
        init: Option<InitVal>,
        ty: BasicTypeEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        if let Some(init) = init
            && let Some(expr) = init.expr()
            && let Expr::Literal(lit) = expr
        {
            return self.compile_literal(lit);
        }

        ty.const_zero()
    }

    fn compile_init_val(&mut self, init: InitVal) -> Option<BasicValueEnum<'ctx>> {
        if let Some(expr) = init.expr() {
            Some(self.compile_expr(expr))
        } else if init.inits().next().is_some() {
            // 简化：数组初始化暂时返回零
            None
        } else {
            None
        }
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
            .unwrap_or((self.context.i32_type().into(), false));

        let params: Vec<BasicMetadataTypeEnum<'ctx>> = func
            .params()
            .map(|ps| {
                ps.params()
                    .map(|p| self.compile_func_f_param_type(p))
                    .collect()
            })
            .unwrap_or_default();

        let fn_type = if is_void {
            self.context.void_type().fn_type(&params, false)
        } else {
            ret_ty.fn_type(&params, false)
        };

        let function = self.module.add_function(&name, fn_type, None);
        self.functions.insert(name.clone(), function);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let prev_func = self.current_function;
        self.current_function = Some(function);
        self.push_scope();

        if let Some(params) = func.params() {
            for (i, param) in params.params().enumerate() {
                let param_val = function.get_nth_param(i as u32).expect("参数索引越界");
                let pname = name_text(&param.name().expect("参数缺名"));
                param_val.set_name(&pname);
                let alloc_ty = param_val.get_type();
                let alloca = self.create_entry_alloca(function, alloc_ty, &pname);
                self.builder
                    .build_store(alloca, param_val)
                    .expect("参数存储失败");
                self.insert_var(pname, alloca, alloc_ty);
            }
        }

        if let Some(block) = func.block() {
            self.compile_block(block);
        }

        // 如无显式 return，补一个
        let has_term = function
            .get_last_basic_block()
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

    fn compile_func_type(&mut self, ty: FuncType) -> (BasicTypeEnum<'ctx>, bool) {
        if ty.void_token().is_some() {
            return (self.context.i8_type().into(), true);
        }
        let base = ty
            .ty()
            .map(|t| self.compile_type(t))
            .expect("函数返回缺类型");
        let full = apply_pointer(base, ty.pointer());
        (full, false)
    }

    #[allow(deprecated)]
    fn compile_func_f_param_type(&mut self, param: FuncFParam) -> BasicMetadataTypeEnum<'ctx> {
        let base = param
            .ty()
            .map(|t| self.compile_type(t))
            .expect("参数缺类型");
        let full = apply_pointer(base, param.pointer());
        // 形参写成 a[] 等价指针
        if param.l_brack_token().is_some() {
            todo!("暂不支持数组形参");
        }
        full.into()
    }

    // 4. Block & Statements
    fn compile_block(&mut self, block: Block) {
        self.push_scope();
        for item in block.items() {
            match item {
                BlockItem::Decl(decl) => self.compile_local_decl(decl),
                BlockItem::Stmt(stmt) => self.compile_stmt(stmt),
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
        dbg!(&rhs);
        let lhs_ptr = self.lval_to_ptr(stmt.lhs().expect("赋值缺左值"), "暂不支持的左值");
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

        let bool_val = as_bool(self.builder, self.context, cond_val);
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
        let bool_val = as_bool(self.builder, self.context, cond_val);
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
            let val = self.compile_expr(expr);
            self.builder.build_return(Some(&val)).ok();
        } else {
            self.builder.build_return(None).ok();
        }
    }

    fn compile_expr(&mut self, expr: Expr) -> BasicValueEnum<'ctx> {
        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::DerefExpr(e) => self.compile_deref_expr(e),
            Expr::IndexVal(e) => self.compile_index_val(e),
            Expr::Literal(e) => self.compile_literal(e),
        }
    }

    fn compile_binary_expr(&mut self, expr: BinaryExpr) -> BasicValueEnum<'ctx> {
        use inkwell::FloatPredicate;
        use inkwell::IntPredicate;

        let lhs = expr
            .lhs()
            .map(|e| self.compile_expr(e))
            .expect("二元左值缺失");
        let rhs = expr
            .rhs()
            .map(|e| self.compile_expr(e))
            .expect("二元右值缺失");

        let op_token = expr
            .op()
            .and_then(|o| {
                o.syntax()
                    .children_with_tokens()
                    .filter_map(|it| {
                        it.into_token()
                            .and_then(|x| x.kind().is_binary_op().then_some(x))
                    })
                    .next()
            })
            .expect("二元运算符缺失");

        match (lhs, rhs) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let res = match op_token.kind() {
                    SyntaxKind::PLUS => self.builder.build_int_add(l, r, "add").unwrap(),
                    SyntaxKind::MINUS => self.builder.build_int_sub(l, r, "sub").unwrap(),
                    SyntaxKind::STAR => self.builder.build_int_mul(l, r, "mul").unwrap(),
                    SyntaxKind::SLASH => self.builder.build_int_signed_div(l, r, "div").unwrap(),
                    SyntaxKind::PERCENT => self.builder.build_int_signed_rem(l, r, "rem").unwrap(),
                    SyntaxKind::LT => self
                        .builder
                        .build_int_compare(IntPredicate::SLT, l, r, "lt")
                        .unwrap(),
                    SyntaxKind::GT => self
                        .builder
                        .build_int_compare(IntPredicate::SGT, l, r, "gt")
                        .unwrap(),
                    SyntaxKind::LTEQ => self
                        .builder
                        .build_int_compare(IntPredicate::SLE, l, r, "le")
                        .unwrap(),
                    SyntaxKind::GTEQ => self
                        .builder
                        .build_int_compare(IntPredicate::SGE, l, r, "ge")
                        .unwrap(),
                    SyntaxKind::EQEQ => self
                        .builder
                        .build_int_compare(IntPredicate::EQ, l, r, "eq")
                        .unwrap(),
                    SyntaxKind::NEQ => self
                        .builder
                        .build_int_compare(IntPredicate::NE, l, r, "ne")
                        .unwrap(),
                    SyntaxKind::AMPAMP => self.builder.build_and(l, r, "and").unwrap(),
                    SyntaxKind::PIPEPIPE => self.builder.build_or(l, r, "or").unwrap(),
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
            _ => panic!("类型不匹配的二元运算"),
        }
    }

    fn compile_unary_expr(&mut self, expr: UnaryExpr) -> BasicValueEnum<'ctx> {
        let op_token = expr
            .op()
            .and_then(|o| {
                o.syntax()
                    .children_with_tokens()
                    .filter_map(|it| it.into_token())
                    .next()
            })
            .expect("一元运算符缺失");
        let val = expr
            .expr()
            .map(|e| self.compile_expr(e))
            .expect("一元运算缺操作数");

        match val {
            BasicValueEnum::IntValue(i) => match op_token.kind() {
                SyntaxKind::PLUS => i.into(),
                SyntaxKind::MINUS => self.builder.build_int_neg(i, "ineg").unwrap().into(),
                SyntaxKind::BANG => self.builder.build_not(i, "inot").unwrap().into(),
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
            .map(|rps| rps.args().map(|a| self.compile_expr(a).into()).collect())
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

    fn compile_deref_expr(&mut self, expr: DerefExpr) -> BasicValueEnum<'ctx> {
        let ptr_val = expr
            .expr()
            .map(|e| self.compile_expr(e))
            .expect("解引用缺操作数");
        let ptr = ptr_val.into_pointer_value();
        let elem_ty: BasicTypeEnum<'ctx> = self.context.i32_type().into();
        self.builder
            .build_load(elem_ty, ptr, "deref")
            .expect("解引用失败")
    }

    fn compile_index_val(&mut self, expr: IndexVal) -> BasicValueEnum<'ctx> {
        let name = name_text(&expr.name().expect("变量缺名"));
        let (ptr, elem_ty) = self.lookup_var(&name).expect("变量未定义");
        let ptr_ty = ptr.get_type();

        if expr.indices().next().is_none() {
            return self
                .builder
                .build_load(elem_ty, ptr, &name)
                .expect("变量读取失败");
        }

        let indices: Vec<_> = expr
            .indices()
            .map(|e| self.compile_expr(e).into_int_value())
            .collect();

        let i32_ty = self.context.i32_type();
        let idx_vals: Vec<_> = indices
            .into_iter()
            .map(|v| self.builder.build_int_cast(v, i32_ty, "idx").unwrap())
            .collect();

        let elem_ptr = unsafe {
            self.builder
                .build_gep(ptr_ty, ptr, &idx_vals, "idx.gep")
                .expect("数组 GEP 失败")
        };
        self.builder
            .build_load(elem_ty, elem_ptr, "idx.load")
            .unwrap()
    }

    fn compile_literal(&mut self, expr: Literal) -> BasicValueEnum<'ctx> {
        if let Some(int_token) = expr.int_token() {
            let text = int_token.text().to_string();
            let v: i64 = text.parse().unwrap_or(0);
            return self.context.i32_type().const_int(v as u64, true).into();
        }
        if let Some(float_token) = expr.float_token() {
            let text = float_token.text().to_string();
            let v: f64 = text.parse().unwrap_or(0.0);
            return self.context.f32_type().const_float(v).into();
        }
        panic!("未知字面量");
    }

    fn compile_const_expr(&mut self, expr: ConstExpr) -> BasicValueEnum<'ctx> {
        // 仅支持字面量/常量二元表达式，复杂情况后续扩展
        if let Some(inner) = expr.expr() {
            match inner {
                Expr::Literal(lit) => self.compile_literal(lit),
                _ => self.context.i32_type().const_int(0, false).into(),
            }
        } else {
            self.context.i32_type().const_int(0, false).into()
        }
    }

    fn _compile_const_index_val(&mut self, _val: ConstIndexVal) {
        todo!();
    }

    // 6. Basic Elements
    fn compile_type(&mut self, ty: Type) -> BasicTypeEnum<'ctx> {
        if ty.int_token().is_some() {
            return self.context.i32_type().into();
        }
        if ty.float_token().is_some() {
            return self.context.f32_type().into();
        }
        if ty.struct_token().is_some() {
            todo!("暂不支持 struct 类型");
        }
        panic!("未知类型");
    }

    fn lval_to_ptr(&mut self, lval: LVal, err: &str) -> PointerValue<'ctx> {
        match lval {
            LVal::IndexVal(v) => {
                let name = name_text(&v.name().expect("左值缺名"));
                self.lookup_var(&name).map(|(p, _)| p).expect(err)
            }
            LVal::DerefExpr(_d) => {
                todo!()
            }
        }
    }
}
