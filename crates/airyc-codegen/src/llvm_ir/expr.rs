use airyc_analyzer::r#type::NType;
use airyc_parser::ast::*;
use airyc_parser::syntax_kind::SyntaxKind;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum};

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;
use crate::utils::*;

impl<'a, 'ctx> Program<'a, 'ctx> {
    pub(crate) fn compile_expr(&mut self, expr: Expr) -> Result<BasicValueEnum<'ctx>> {
        self.compile_expr_inner(expr, false)
    }

    fn compile_expr_inner(
        &mut self,
        expr: Expr,
        is_func_arg: bool,
    ) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::BinaryExpr(e) => self.compile_binary_expr(e),
            Expr::UnaryExpr(e) => self.compile_unary_expr(e),
            Expr::CallExpr(e) => self.compile_call_expr(e),
            Expr::ParenExpr(e) => self.compile_paren_expr(e),
            Expr::DerefExpr(e) => self.compile_deref_expr(e),
            Expr::IndexVal(e) => self.compile_index_val(e, is_func_arg),
            Expr::Literal(e) => self.compile_literal(e),
        }
    }

    fn compile_deref_expr(&mut self, expr: DerefExpr) -> Result<BasicValueEnum<'ctx>> {
        let inner = expr.expr().ok_or(CodegenError::Missing("deref operand"))?;
        let ptr = self.compile_expr(inner)?.into_pointer_value();
        let result_ty = self
            .analyzer
            .get_expr_type(expr.syntax().text_range())
            .ok_or(CodegenError::Missing("deref type"))?;
        let llvm_ty = self.convert_ntype_to_type(result_ty)?;
        self.builder
            .build_load(llvm_ty, ptr, "deref")
            .map_err(|_| CodegenError::LlvmBuild("deref load"))
    }

    fn compile_binary_expr(&mut self, expr: BinaryExpr) -> Result<BasicValueEnum<'ctx>> {
        use inkwell::FloatPredicate;
        use inkwell::IntPredicate;

        let op_token = expr
            .op()
            .ok_or(CodegenError::Missing("binary operator"))?
            .op();

        if let Some(func) = self.symbols.current_function
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

        let lhs_node = expr.lhs().ok_or(CodegenError::Missing("left operand"))?;
        let rhs_node = expr.rhs().ok_or(CodegenError::Missing("right operand"))?;
        let lhs = self.compile_expr(lhs_node.clone())?;
        let rhs = self.compile_expr(rhs_node.clone())?;

        match (lhs, rhs) {
            // 指针 + 整数
            (BasicValueEnum::PointerValue(p), BasicValueEnum::IntValue(i)) => {
                let lhs_ty = self
                    .analyzer
                    .get_expr_type(lhs_node.syntax().text_range())
                    .ok_or(CodegenError::Missing("lhs type"))?;
                let NType::Pointer(pointee) = lhs_ty else {
                    return Err(CodegenError::TypeMismatch("expected pointer".into()));
                };
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
                    .get_expr_type(rhs_node.syntax().text_range())
                    .ok_or(CodegenError::Missing("rhs type"))?;
                let NType::Pointer(pointee) = rhs_ty else {
                    return Err(CodegenError::TypeMismatch("expected pointer".into()));
                };
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
            // 指针 - 指针
            (BasicValueEnum::PointerValue(p1), BasicValueEnum::PointerValue(p2)) => {
                if op_token.kind() != SyntaxKind::MINUS {
                    return Err(CodegenError::Unsupported("ptr + ptr".into()));
                }
                let lhs_ty = self
                    .analyzer
                    .get_expr_type(lhs_node.syntax().text_range())
                    .ok_or(CodegenError::Missing("lhs type"))?;
                let NType::Pointer(pointee) = lhs_ty else {
                    return Err(CodegenError::TypeMismatch("expected pointer".into()));
                };
                let elem_size = self.get_type_size(pointee)?;
                let i64_ty = self.context.i64_type();
                let i1 = self
                    .builder
                    .build_ptr_to_int(p1, i64_ty, "p1")
                    .map_err(|_| CodegenError::LlvmBuild("ptr_to_int"))?;
                let i2 = self
                    .builder
                    .build_ptr_to_int(p2, i64_ty, "p2")
                    .map_err(|_| CodegenError::LlvmBuild("ptr_to_int"))?;
                let diff = self
                    .builder
                    .build_int_sub(i1, i2, "diff")
                    .map_err(|_| CodegenError::LlvmBuild("sub"))?;
                let size_val = i64_ty.const_int(elem_size, false);
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
                    SyntaxKind::LT => self.build_int_cmp(IntPredicate::SLT, l, r, "lt")?,
                    SyntaxKind::GT => self.build_int_cmp(IntPredicate::SGT, l, r, "gt")?,
                    SyntaxKind::LTEQ => self.build_int_cmp(IntPredicate::SLE, l, r, "le")?,
                    SyntaxKind::GTEQ => self.build_int_cmp(IntPredicate::SGE, l, r, "ge")?,
                    SyntaxKind::EQEQ => self.build_int_cmp(IntPredicate::EQ, l, r, "eq")?,
                    SyntaxKind::NEQ => self.build_int_cmp(IntPredicate::NE, l, r, "ne")?,
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
                    SyntaxKind::LT => self.build_float_cmp(FloatPredicate::OLT, l, r, "flt")?,
                    SyntaxKind::GT => self.build_float_cmp(FloatPredicate::OGT, l, r, "fgt")?,
                    SyntaxKind::LTEQ => self.build_float_cmp(FloatPredicate::OLE, l, r, "fle")?,
                    SyntaxKind::GTEQ => self.build_float_cmp(FloatPredicate::OGE, l, r, "fge")?,
                    SyntaxKind::EQEQ => self.build_float_cmp(FloatPredicate::OEQ, l, r, "feq")?,
                    SyntaxKind::NEQ => self.build_float_cmp(FloatPredicate::ONE, l, r, "fne")?,
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

        // 取地址需要特殊处理，不能先编译操作数
        if op_token.kind() == SyntaxKind::AMP {
            let operand = expr.expr().ok_or(CodegenError::Missing("& operand"))?;
            return match operand {
                Expr::IndexVal(iv) => {
                    let (_, ptr, _) = self.get_element_ptr_by_index_val(&iv)?;
                    Ok(ptr.into())
                }
                Expr::DerefExpr(de) => {
                    // &*ptr == ptr
                    self.compile_expr(de.expr().ok_or(CodegenError::Missing("deref operand"))?)
                }
                _ => Err(CodegenError::Unsupported("cannot take address".into())),
            };
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
            .or_else(|| self.symbols.functions.get(&name).copied())
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;

        let args: Vec<BasicMetadataValueEnum<'ctx>> = expr
            .args()
            .map(|rps| {
                rps.args()
                    .map(|a| self.compile_expr_inner(a, true).map(|v| v.into()))
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
        _func_call_r_param: bool,
    ) -> Result<BasicValueEnum<'ctx>> {
        let (ty, ptr, name) = self.get_element_ptr_by_index_val(&expr)?;
        if ty.is_array_type() {
            // 数组 decay 成指向第一个元素的指针
            let zero = self.context.i32_type().const_zero();
            let gep = unsafe {
                self.builder
                    .build_gep(ty, ptr, &[zero, zero], "arr.decay")
                    .map_err(|_| CodegenError::LlvmBuild("gep"))?
            };
            Ok(gep.into())
        } else {
            self.builder
                .build_load(ty, ptr, &name)
                .map_err(|_| CodegenError::LlvmBuild("load"))
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
}
