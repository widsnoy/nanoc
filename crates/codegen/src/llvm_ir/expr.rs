use analyzer::r#type::Ty;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue};
use syntax::ast::*;
use syntax::syntax_kind::SyntaxKind;

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;

impl<'a, 'ctx> Program<'a, 'ctx> {
    pub(crate) fn compile_expr(&mut self, expr: Expr) -> Result<BasicValueEnum<'ctx>> {
        // 优先检查是否为编译时常量
        let range = expr.text_range();
        if self.analyzer.is_compile_time_constant(range) {
            // 获取表达式类型以正确转换 Array/Struct
            let ty = self
                .analyzer
                .get_expr_type(range)
                .and_then(|nty| self.convert_ntype_to_type(nty).ok());
            return self.get_const_var_value_by_range(range, ty);
        }

        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::IndexVal(e) => self.compile_index_val(e),
            Expr::Literal(e) => self.compile_literal(e),
            Expr::PostfixExpr(e) => self.compile_postfix_expr(e),
        }
    }

    fn compile_deref_expr(&mut self, expr: &UnaryExpr) -> Result<BasicValueEnum<'ctx>> {
        // 获取整个解引用表达式的类型（即解引用后的结果类型）
        let range = expr.text_range();
        let operand = expr.expr().ok_or(CodegenError::Missing("* operand"))?;
        let ptr = self.compile_expr(operand)?.into_pointer_value();
        let result_ty = self
            .analyzer
            .get_expr_type(range)
            .ok_or(CodegenError::Missing("deref type"))?;
        let llvm_ty = self.convert_ntype_to_type(result_ty)?;
        self.builder
            .build_load(llvm_ty, ptr, "deref")
            .map_err(|_| CodegenError::LlvmBuild("deref load"))
    }

    fn compile_binary_expr(&mut self, expr: BinaryExpr) -> Result<BasicValueEnum<'ctx>> {
        use inkwell::IntPredicate;

        let op_token = expr
            .op()
            .ok_or(CodegenError::Missing("binary operator"))?
            .op();

        if let Some(func) = self.symbols.current_function
            && matches!(op_token.kind(), SyntaxKind::AMPAMP | SyntaxKind::PIPEPIPE)
        {
            let bool_false = self.context.bool_type().const_zero();
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
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs,
                    lhs.get_type().const_zero(),
                    "land.eq_0",
                )
                .map_err(|_| CodegenError::LlvmBuild("int compare failed"))?;
            let short_circuit_val = if op_token.kind() == SyntaxKind::AMPAMP {
                let _ = self
                    .builder
                    .build_conditional_branch(eq_zero, merge_bb, rhs_bb);
                bool_false
            } else {
                let _ = self
                    .builder
                    .build_conditional_branch(eq_zero, rhs_bb, merge_bb);
                self.context.bool_type().const_all_ones()
            };

            self.builder.position_at_end(rhs_bb);
            let rhs =
                self.compile_expr(expr.rhs().ok_or(CodegenError::Missing("right operand"))?)?;
            let rhs_val = self.as_bool(rhs)?;
            let rhs_end_bb = self
                .builder
                .get_insert_block()
                .ok_or(CodegenError::LlvmBuild("no current basic block"))?;
            let _ = self.builder.build_unconditional_branch(merge_bb);

            self.builder.position_at_end(merge_bb);
            let merge = self
                .builder
                .build_phi(self.context.bool_type(), "land.phi")
                .map_err(|_| CodegenError::LlvmBuild("phi build failed"))?;

            merge.add_incoming(&[(&short_circuit_val, lhs_bb), (&rhs_val, rhs_end_bb)]);
            return Ok(merge.as_basic_value());
        }

        let lhs_node = expr.lhs().ok_or(CodegenError::Missing("left operand"))?;
        let rhs_node = expr.rhs().ok_or(CodegenError::Missing("right operand"))?;
        let lhs = self.compile_expr(lhs_node.clone())?;
        let rhs = self.compile_expr(rhs_node.clone())?;

        match (lhs, rhs) {
            // 指针 + 整数
            (BasicValueEnum::PointerValue(p), BasicValueEnum::IntValue(i)) => {
                let lhs_ty = self
                    .analyzer
                    .get_expr_type(lhs_node.text_range())
                    .ok_or(CodegenError::Missing("lhs type"))?;
                let pointee = lhs_ty
                    .pointer_inner()
                    .ok_or_else(|| CodegenError::TypeMismatch("expected pointer".into()))?;
                let llvm_ty = self.convert_ntype_to_type(pointee)?;
                match op_token.kind() {
                    SyntaxKind::PLUS => {
                        let gep = unsafe {
                            self.builder
                                .build_gep(llvm_ty, p, &[i], "ptr.add")
                                .map_err(|_| CodegenError::LlvmBuild("gep"))?
                        };
                        Ok(gep.into())
                    }
                    SyntaxKind::MINUS => {
                        let neg = self
                            .builder
                            .build_int_neg(i, "neg")
                            .map_err(|_| CodegenError::LlvmBuild("neg"))?;
                        let gep = unsafe {
                            self.builder
                                .build_gep(llvm_ty, p, &[neg], "ptr.sub")
                                .map_err(|_| CodegenError::LlvmBuild("gep"))?
                        };
                        Ok(gep.into())
                    }
                    _ => Err(CodegenError::Unsupported("ptr binary op".into())),
                }
            }
            // 整数 + 指针
            (BasicValueEnum::IntValue(i), BasicValueEnum::PointerValue(p)) => {
                let rhs_ty = self
                    .analyzer
                    .get_expr_type(rhs_node.text_range())
                    .ok_or(CodegenError::Missing("rhs type"))?;
                let pointee = rhs_ty
                    .pointer_inner()
                    .ok_or_else(|| CodegenError::TypeMismatch("expected pointer".into()))?;
                let llvm_ty = self.convert_ntype_to_type(pointee)?;
                if op_token.kind() == SyntaxKind::PLUS {
                    let gep = unsafe {
                        self.builder
                            .build_gep(llvm_ty, p, &[i], "ptr.add")
                            .map_err(|_| CodegenError::LlvmBuild("gep"))?
                    };
                    Ok(gep.into())
                } else {
                    Err(CodegenError::Unsupported("int - ptr".into()))
                }
            }
            // 指针 - 指针 / 指针比较
            (BasicValueEnum::PointerValue(p1), BasicValueEnum::PointerValue(p2)) => {
                self.compile_ptr_binary_op(op_token.kind(), p1, p2, lhs_node, rhs_node)
            }
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                // 获取操作数的语义类型
                let lhs_ty = self
                    .analyzer
                    .get_expr_type(lhs_node.text_range())
                    .ok_or(CodegenError::Missing("lhs type"))?;
                let rhs_ty = self
                    .analyzer
                    .get_expr_type(rhs_node.text_range())
                    .ok_or(CodegenError::Missing("rhs type"))?;

                self.compile_int_binary_op(op_token.kind(), l, r, lhs_ty, rhs_ty)
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

        // 取地址需要特殊处理，不能先编译操作数
        let op_kind = op_token.kind();
        if op_kind == SyntaxKind::AMP {
            let operand = expr.expr().ok_or(CodegenError::Missing("& operand"))?;
            return match operand {
                Expr::IndexVal(iv) => {
                    let (_, ptr, _) = self.get_index_val_ptr(&iv)?;
                    Ok(ptr.into())
                }
                Expr::UnaryExpr(de) if de.op().map(|x| x.op().kind()) == Some(SyntaxKind::STAR) => {
                    // &*ptr == ptr
                    self.compile_expr(de.expr().ok_or(CodegenError::Missing("deref operand"))?)
                }
                Expr::PostfixExpr(pe) => {
                    let (ptr, _) = self.get_postfix_expr_ptr(pe)?;
                    Ok(ptr.into())
                }
                _ => Err(CodegenError::Unsupported(format!(
                    "cannot take address {:?}",
                    operand.syntax().text()
                ))),
            };
        } else if op_kind == SyntaxKind::STAR {
            return self.compile_deref_expr(&expr);
        }

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
            _ => Err(CodegenError::Unsupported("operand type".into())),
        }
    }

    fn compile_call_expr(&mut self, expr: CallExpr) -> Result<BasicValueEnum<'ctx>> {
        let name = expr
            .name()
            .and_then(|n| n.var_name())
            .ok_or(CodegenError::Missing("function name"))?;
        let func = self
            .module
            .get_function(&name)
            .or_else(|| self.symbols.functions.get(&name).copied())
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;

        // 获取函数参数类型
        let param_types: Vec<Ty> = if let Some(fid) = self.analyzer.get_function_id_by_name(&name)
            && let Some(func_info) = self.analyzer.get_function_by_id(fid)
        {
            func_info.meta_types.into_iter().map(|(_, ty)| ty).collect()
        } else {
            return Err(CodegenError::Missing("function info"));
        };

        let args: Vec<BasicMetadataValueEnum<'ctx>> = if let Some(rps) = expr.args() {
            rps.args()
                .enumerate()
                .map(|(i, arg_expr)| {
                    let val = self.compile_expr(arg_expr.clone())?;

                    // 如果有参数类型信息，进行类型转换
                    if i < param_types.len() {
                        let arg_ty = self
                            .analyzer
                            .get_expr_type(arg_expr.text_range())
                            .ok_or(CodegenError::Missing("arg type"))?;
                        let casted = self.cast_value(val, arg_ty, &param_types[i])?;
                        Ok(casted.into())
                    } else {
                        Ok(val.into())
                    }
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![]
        };

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

    fn compile_index_val(&mut self, expr: IndexVal) -> Result<BasicValueEnum<'ctx>> {
        let (ty, ptr, name) = self.get_index_val_ptr(&expr)?;

        // 如果是数组类型，执行 decay 并返回指针
        if ty.is_array_type() {
            let (_, decayed_ptr) = self.maybe_decay_array(ty, ptr)?;
            return Ok(decayed_ptr.into());
        }

        // 否则 load 值
        self.builder
            .build_load(ty, ptr, &name)
            .map_err(|_| CodegenError::LlvmBuild("load"))
    }

    fn compile_literal(&mut self, expr: Literal) -> Result<BasicValueEnum<'ctx>> {
        let range = expr.text_range();
        self.get_const_var_value_by_range(range, None)
    }

    /// 获取 struct 字段的指针
    /// 返回 (字段指针, 字段类型)
    fn get_struct_field_ptr(
        &mut self,
        base_ptr: PointerValue<'ctx>,
        base_ty: &Ty,
        member_name: &str,
        is_pointer_access: bool,
    ) -> Result<(PointerValue<'ctx>, BasicTypeEnum<'ctx>, Ty)> {
        // 根据访问方式提取 struct ID
        let struct_id = if is_pointer_access {
            base_ty
                .as_struct_pointer_id()
                .ok_or(CodegenError::NotImplemented("not a struct pointer"))?
        } else {
            base_ty
                .as_struct_id()
                .ok_or(CodegenError::NotImplemented("not a struct"))?
        };

        let struct_def = self
            .analyzer
            .get_struct_by_id(struct_id)
            .ok_or(CodegenError::NotImplemented("undefined struct"))?;

        let field_idx = struct_def
            .field_index(self.analyzer, member_name)
            .ok_or(CodegenError::NotImplemented("field not found"))?;

        let field_id = struct_def.fields[field_idx as usize];
        let field = self.analyzer.fields.get(field_id.index).unwrap();
        let field_ty = field.ty.clone();

        let struct_ntype = Ty::Struct {
            id: struct_id,
            name: struct_def.name.clone(),
        };
        let struct_llvm_ty = self.convert_ntype_to_type(&struct_ntype)?;

        let field_ptr = self
            .builder
            .build_struct_gep(struct_llvm_ty, base_ptr, field_idx, member_name)
            .map_err(|_| CodegenError::LlvmBuild("struct gep failed"))?;
        let filed_llvm_ty = self.convert_ntype_to_type(&field_ty)?;

        Ok((field_ptr, filed_llvm_ty, field_ty))
    }

    fn compile_postfix_expr(&mut self, expr: PostfixExpr) -> Result<BasicValueEnum<'ctx>> {
        let (field_ptr, field_ty) = self.get_postfix_expr_ptr(expr)?;

        // 如果是数组类型，执行 decay 并返回指针
        if field_ty.is_array_type() {
            let (_, decayed_ptr) = self.maybe_decay_array(field_ty, field_ptr)?;
            return Ok(decayed_ptr.into());
        }

        // 否则 load 值
        self.builder
            .build_load(field_ty, field_ptr, "field")
            .map_err(|_| CodegenError::LlvmBuild("load field failed"))
    }

    /// 编译 PostfixExpr 为左值，返回 (指针, 字段类型)
    fn get_postfix_expr_ptr(
        &mut self,
        postfix: PostfixExpr,
    ) -> Result<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        let op = postfix
            .op()
            .ok_or(CodegenError::Missing("postfix operator"))?;
        let op_kind = op.op().kind();

        // 获取字段 FieldAccess（包含字段名和可能的数组索引）
        let field_access = postfix
            .field()
            .ok_or(CodegenError::Missing("field access"))?;
        let member_name = field_access
            .name()
            .and_then(|n| n.var_name())
            .ok_or(CodegenError::Missing("member name"))?;

        let base_expr = postfix
            .expr()
            .ok_or(CodegenError::Missing("base expression"))?;
        let base_range = base_expr.text_range();
        let base_ty = self
            .analyzer
            .get_expr_type(base_range)
            .ok_or(CodegenError::Missing("base type"))?
            .clone();

        // 根据操作符类型获取基础指针
        let (base_ptr, is_pointer_access) = match op_kind {
            SyntaxKind::DOT => (self.get_expr_ptr(base_expr)?, false),
            SyntaxKind::ARROW => (self.compile_expr(base_expr)?.into_pointer_value(), true),
            _ => return Err(CodegenError::NotImplemented("unknown postfix operator")),
        };

        // 获取字段指针和类型
        let (mut field_ptr, mut field_llvm_ty, field_ty) =
            self.get_struct_field_ptr(base_ptr, &base_ty, &member_name, is_pointer_access)?;

        // 处理数组索引（如 arr[0] 或 arr[0][1]）
        let indices: Vec<_> = field_access
            .indices()
            .map(|e| self.compile_expr(e).map(|v| v.into_int_value()))
            .collect::<Result<Vec<_>>>()?;

        (field_llvm_ty, field_ptr) =
            self.calculate_index_op(field_ty, field_llvm_ty, field_ptr, indices)?;

        // eprintln!("name: {member_name}, type: {field_llvm_ty}");

        Ok((field_ptr, field_llvm_ty))
    }

    /// 编译表达式为左值（返回指针）
    pub(crate) fn get_expr_ptr(&mut self, expr: Expr) -> Result<PointerValue<'ctx>> {
        match expr {
            Expr::IndexVal(index_val) => {
                // IndexVal 可以作为左值
                let (_, ptr, _) = self.get_index_val_ptr(&index_val)?;
                Ok(ptr)
            }
            Expr::UnaryExpr(unary) => {
                // 解引用表达式可以作为左值
                let op = unary.op().ok_or(CodegenError::Missing("unary operator"))?;
                if op.op().kind() == SyntaxKind::STAR {
                    let operand = unary.expr().ok_or(CodegenError::Missing("* operand"))?;
                    Ok(self.compile_expr(operand)?.into_pointer_value())
                } else {
                    Err(CodegenError::NotImplemented("not an lvalue"))
                }
            }
            Expr::PostfixExpr(postfix) => {
                let (ptr, _) = self.get_postfix_expr_ptr(postfix)?;
                Ok(ptr)
            }
            _ => Err(CodegenError::NotImplemented("not an lvalue")),
        }
    }

    /// Get (type, ptr) from Index
    pub(crate) fn get_index_val_ptr(
        &mut self,
        index_val: &IndexVal,
    ) -> Result<(BasicTypeEnum<'ctx>, PointerValue<'ctx>, String)> {
        let name = index_val
            .name()
            .and_then(|n| n.var_name())
            .ok_or(CodegenError::Missing("function name"))?;
        let symbol = self
            .symbols
            .lookup_var(&name)
            .ok_or_else(|| CodegenError::UndefinedVar(name.clone()))?;
        let (mut ptr, cur_ntype) = (symbol.ptr, symbol.ty.clone());
        let mut cur_llvm_type = self.convert_ntype_to_type(&cur_ntype)?;

        let indices: Vec<_> = index_val
            .indices()
            .map(|e| self.compile_expr(e).map(|v| v.into_int_value()))
            .collect::<Result<Vec<_>>>()?;

        (cur_llvm_type, ptr) = self.calculate_index_op(cur_ntype, cur_llvm_type, ptr, indices)?;
        Ok((cur_llvm_type, ptr, name))
    }

    /// 编译整数二元运算（算术、比较、逻辑）
    /// 统一处理类型提升和运算逻辑
    fn compile_int_binary_op(
        &mut self,
        op: SyntaxKind,
        l: inkwell::values::IntValue<'ctx>,
        r: inkwell::values::IntValue<'ctx>,
        lhs_ty: &Ty,
        rhs_ty: &Ty,
    ) -> Result<BasicValueEnum<'ctx>> {
        let lhs_unwrapped = lhs_ty.unwrap_const();
        let rhs_unwrapped = rhs_ty.unwrap_const();

        // 算术运算：根据类型提升规则
        if matches!(
            op,
            SyntaxKind::PLUS
                | SyntaxKind::MINUS
                | SyntaxKind::STAR
                | SyntaxKind::SLASH
                | SyntaxKind::PERCENT
        ) {
            return self.compile_int_arithmetic(op, l, r, &lhs_unwrapped, &rhs_unwrapped);
        }

        // 比较运算：返回 bool (i1)
        if matches!(
            op,
            SyntaxKind::LT
                | SyntaxKind::GT
                | SyntaxKind::LTEQ
                | SyntaxKind::GTEQ
                | SyntaxKind::EQEQ
                | SyntaxKind::NEQ
        ) {
            return self.compile_int_comparison(op, l, r, &lhs_unwrapped, &rhs_unwrapped);
        }

        // 逻辑运算：返回 bool (i1)
        if matches!(op, SyntaxKind::AMPAMP | SyntaxKind::PIPEPIPE) {
            let lb = self.cast_int_to_bool(l, lhs_ty)?;
            let rb = self.cast_int_to_bool(r, rhs_ty)?;
            let res = match op {
                SyntaxKind::AMPAMP => self
                    .builder
                    .build_and(lb, rb, "and")
                    .map_err(|_| CodegenError::LlvmBuild("and"))?,
                SyntaxKind::PIPEPIPE => self
                    .builder
                    .build_or(lb, rb, "or")
                    .map_err(|_| CodegenError::LlvmBuild("or"))?,
                _ => unreachable!(),
            };
            return Ok(res.into());
        }

        Err(CodegenError::Unsupported(format!("int binary op {:?}", op)))
    }

    /// 编译整数算术运算
    fn compile_int_arithmetic(
        &mut self,
        op: SyntaxKind,
        l: inkwell::values::IntValue<'ctx>,
        r: inkwell::values::IntValue<'ctx>,
        lhs_ty_unwrapped: &Ty,
        rhs_ty_unwrapped: &Ty,
    ) -> Result<BasicValueEnum<'ctx>> {
        match (lhs_ty_unwrapped, rhs_ty_unwrapped) {
            // 相同类型保持不变
            (Ty::I32, Ty::I32) | (Ty::I8, Ty::I8) => self.build_int_arithmetic_op(op, l, r),
            // i32 混合类型：提升到 i32
            (Ty::I32, Ty::I8 | Ty::Bool) | (Ty::I8 | Ty::Bool, Ty::I32) => {
                let l_i32 = self.cast_int_to_i32(l, lhs_ty_unwrapped)?;
                let r_i32 = self.cast_int_to_i32(r, rhs_ty_unwrapped)?;
                self.build_int_arithmetic_op(op, l_i32, r_i32)
            }
            // i8 + bool：提升到 i8
            (Ty::I8, Ty::Bool) | (Ty::Bool, Ty::I8) => {
                let l_i8 = self.cast_int_to_i8(l, lhs_ty_unwrapped)?;
                let r_i8 = self.cast_int_to_i8(r, rhs_ty_unwrapped)?;
                self.build_int_arithmetic_op(op, l_i8, r_i8)
            }
            // bool + bool：提升到 i32
            (Ty::Bool, Ty::Bool) => {
                let l_i32 = self.cast_int_to_i32(l, lhs_ty_unwrapped)?;
                let r_i32 = self.cast_int_to_i32(r, rhs_ty_unwrapped)?;
                self.build_int_arithmetic_op(op, l_i32, r_i32)
            }
            _ => Err(CodegenError::TypeMismatch(
                "invalid arithmetic types".into(),
            )),
        }
    }

    /// 编译整数比较运算
    fn compile_int_comparison(
        &mut self,
        op: SyntaxKind,
        l: inkwell::values::IntValue<'ctx>,
        r: inkwell::values::IntValue<'ctx>,
        lhs_ty_unwrapped: &Ty,
        rhs_ty_unwrapped: &Ty,
    ) -> Result<BasicValueEnum<'ctx>> {
        match (lhs_ty_unwrapped, rhs_ty_unwrapped) {
            // 相同类型直接比较
            (Ty::I32, Ty::I32) | (Ty::I8, Ty::I8) | (Ty::Bool, Ty::Bool) => {
                let cmp = self.build_int_comparison_op(op, l, r)?;
                Ok(cmp.into())
            }
            // i32 混合类型：提升到 i32
            (Ty::I32, Ty::I8 | Ty::Bool) | (Ty::I8 | Ty::Bool, Ty::I32) => {
                let l_i32 = self.cast_int_to_i32(l, lhs_ty_unwrapped)?;
                let r_i32 = self.cast_int_to_i32(r, rhs_ty_unwrapped)?;
                let cmp = self.build_int_comparison_op(op, l_i32, r_i32)?;
                Ok(cmp.into())
            }
            // i8 + bool：提升到 i8
            (Ty::I8, Ty::Bool) | (Ty::Bool, Ty::I8) => {
                let l_i8 = self.cast_int_to_i8(l, lhs_ty_unwrapped)?;
                let r_i8 = self.cast_int_to_i8(r, rhs_ty_unwrapped)?;
                let cmp = self.build_int_comparison_op(op, l_i8, r_i8)?;
                Ok(cmp.into())
            }
            _ => Err(CodegenError::TypeMismatch(
                "invalid comparison types".into(),
            )),
        }
    }

    /// 构建整数算术运算指令
    fn build_int_arithmetic_op(
        &self,
        op: SyntaxKind,
        l: inkwell::values::IntValue<'ctx>,
        r: inkwell::values::IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let res = match op {
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
            _ => unreachable!(),
        };
        Ok(res.into())
    }

    /// 构建整数比较运算指令
    fn build_int_comparison_op(
        &self,
        op: SyntaxKind,
        l: inkwell::values::IntValue<'ctx>,
        r: inkwell::values::IntValue<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>> {
        use inkwell::IntPredicate;
        match op {
            SyntaxKind::LT => self.build_int_cmp(IntPredicate::SLT, l, r, "lt"),
            SyntaxKind::GT => self.build_int_cmp(IntPredicate::SGT, l, r, "gt"),
            SyntaxKind::LTEQ => self.build_int_cmp(IntPredicate::SLE, l, r, "le"),
            SyntaxKind::GTEQ => self.build_int_cmp(IntPredicate::SGE, l, r, "ge"),
            SyntaxKind::EQEQ => self.build_int_cmp(IntPredicate::EQ, l, r, "eq"),
            SyntaxKind::NEQ => self.build_int_cmp(IntPredicate::NE, l, r, "ne"),
            _ => unreachable!(),
        }
    }

    /// 将两个指针转换为 i64 整数
    /// 返回 (i1, i2)
    fn ptr_to_int_pair(
        &self,
        p1: PointerValue<'ctx>,
        p2: PointerValue<'ctx>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, inkwell::values::IntValue<'ctx>)> {
        let i64_ty = self.context.i64_type();
        let i1 = self
            .builder
            .build_ptr_to_int(p1, i64_ty, "p1")
            .map_err(|_| CodegenError::LlvmBuild("ptr_to_int"))?;
        let i2 = self
            .builder
            .build_ptr_to_int(p2, i64_ty, "p2")
            .map_err(|_| CodegenError::LlvmBuild("ptr_to_int"))?;
        Ok((i1, i2))
    }

    /// 编译指针二元运算（减法和比较）
    fn compile_ptr_binary_op(
        &mut self,
        op: SyntaxKind,
        p1: PointerValue<'ctx>,
        p2: PointerValue<'ctx>,
        lhs_node: Expr,
        _rhs_node: Expr,
    ) -> Result<BasicValueEnum<'ctx>> {
        use inkwell::IntPredicate;

        match op {
            // 指针减法：(p1 - p2) / sizeof(pointee)
            SyntaxKind::MINUS => {
                let lhs_ty = self
                    .analyzer
                    .get_expr_type(lhs_node.text_range())
                    .ok_or(CodegenError::Missing("lhs type"))?;
                let pointee = lhs_ty
                    .pointer_inner()
                    .ok_or_else(|| CodegenError::TypeMismatch("expected pointer".into()))?;

                // 获取元素大小
                let llvm_ty = self.convert_ntype_to_type(pointee)?;
                let size_val = llvm_ty
                    .size_of()
                    .ok_or(CodegenError::LlvmBuild("failed to get type size"))?;

                let (i1, i2) = self.ptr_to_int_pair(p1, p2)?;
                let diff = self
                    .builder
                    .build_int_sub(i1, i2, "diff")
                    .map_err(|_| CodegenError::LlvmBuild("sub"))?;

                let result = self
                    .builder
                    .build_int_signed_div(diff, size_val, "ptr.diff")
                    .map_err(|_| CodegenError::LlvmBuild("div"))?;
                let i32_ty = self.context.i32_type();
                let truncated = self
                    .builder
                    .build_int_truncate(result, i32_ty, "diff.i32")
                    .map_err(|_| CodegenError::LlvmBuild("trunc"))?;
                Ok(truncated.into())
            }
            // 指针比较运算
            SyntaxKind::EQEQ | SyntaxKind::NEQ | SyntaxKind::LT | SyntaxKind::GT
            | SyntaxKind::LTEQ | SyntaxKind::GTEQ => {
                let (i1, i2) = self.ptr_to_int_pair(p1, p2)?;
                let predicate = match op {
                    SyntaxKind::EQEQ => IntPredicate::EQ,
                    SyntaxKind::NEQ => IntPredicate::NE,
                    SyntaxKind::LT => IntPredicate::ULT,
                    SyntaxKind::GT => IntPredicate::UGT,
                    SyntaxKind::LTEQ => IntPredicate::ULE,
                    SyntaxKind::GTEQ => IntPredicate::UGE,
                    _ => unreachable!(),
                };
                let cmp = self
                    .builder
                    .build_int_compare(predicate, i1, i2, "ptr.cmp")
                    .map_err(|_| CodegenError::LlvmBuild("ptr compare"))?;
                Ok(cmp.into())
            }
            _ => Err(CodegenError::Unsupported(format!(
                "unsupported pointer operation: {:?}",
                op
            ))),
        }
    }
}
