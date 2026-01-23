use std::collections::HashMap;

use airyc_analyzer::array::ArrayTree;
use airyc_analyzer::r#type::NType;
use airyc_analyzer::value::Value;
use airyc_parser::ast::{AstNode, ConstIndexVal, IndexVal, Name, SyntaxToken};
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

use crate::error::{CodegenError, Result};
use crate::llvm_ir::{LoopContext, Program, Symbol, SymbolTable};

/// Extract ident token from variable definition
pub(crate) fn get_ident_node(name: &ConstIndexVal) -> Option<SyntaxToken> {
    name.name().and_then(|n| n.ident())
}

/// 提取名称文本
pub(crate) fn name_text(name: &Name) -> Option<String> {
    name.ident().map(|t| t.text().to_string())
}

impl<'a, 'ctx> SymbolTable<'a, 'ctx> {
    /// Push new scope
    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop scope
    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn push_loop(&mut self, cond_bb: BasicBlock<'ctx>, end_bb: BasicBlock<'ctx>) {
        self.loop_stack.push(LoopContext { cond_bb, end_bb });
    }

    pub(crate) fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// 插入局部变量
    pub(crate) fn insert_var(&mut self, name: String, ptr: PointerValue<'ctx>, ty: &'a NType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Symbol::new(ptr, ty));
        }
    }

    /// Lookup variable (inner to outer)
    pub(crate) fn lookup_var(&self, name: &str) -> Option<Symbol<'a, 'ctx>> {
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
}

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// Allocate local variable in entry block
    pub(crate) fn create_entry_alloca(
        &self,
        function: FunctionValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>> {
        let entry = function
            .get_first_basic_block()
            .ok_or(CodegenError::LlvmBuild("function has no basic block"))?;
        let builder = self.context.create_builder();
        if let Some(instr) = entry.get_first_instruction() {
            builder.position_before(&instr);
        } else {
            builder.position_at_end(entry);
        }
        builder
            .build_alloca(ty, name)
            .map_err(|_| CodegenError::LlvmBuild("alloca failed"))
    }

    pub(crate) fn declare_sysy_runtime(&self) {
        let i32_type = self.context.i32_type();
        let void_type = self.context.void_type();
        let i32_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let fn_type = i32_type.fn_type(&[], false);
        self.module.add_function("getint", fn_type, None);
        self.module.add_function("getch", fn_type, None);

        let fn_type = i32_type.fn_type(&[i32_ptr_type.into()], false);
        self.module.add_function("getarray", fn_type, None);

        let fn_type = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("putint", fn_type, None);
        self.module.add_function("putch", fn_type, None);

        let fn_type = void_type.fn_type(&[i32_type.into(), i32_ptr_type.into()], false);
        self.module.add_function("putarray", fn_type, None);

        let fn_type = void_type.fn_type(&[], false);
        self.module.add_function("starttime", fn_type, None);
        self.module.add_function("stoptime", fn_type, None);
    }

    /// Convert `NType` to `BasicTypeEnum`
    pub(crate) fn convert_ntype_to_type(&self, ntype: &NType) -> Result<BasicTypeEnum<'ctx>> {
        match ntype {
            NType::Int => Ok(self.context.i32_type().into()),
            NType::Float => Ok(self.context.f32_type().into()),
            NType::Void => Ok(self.context.i8_type().into()),
            NType::Array(ntype, count) => {
                let inner = self.convert_ntype_to_type(ntype)?;
                Ok(inner.array_type(*count as u32).into())
            }
            NType::Pointer(_) => Ok(self.context.ptr_type(AddressSpace::default()).into()),
            NType::Struct(_) => Err(CodegenError::NotImplemented("struct type")),
            NType::Const(ntype) => self.convert_ntype_to_type(ntype),
        }
    }

    /// Convert `ArrayTree` to LLVM constant value for global variable initialization.
    /// This function only handles compile-time constants.
    pub(crate) fn convert_array_tree_to_global_init(
        &self,
        tree: &ArrayTree,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match tree {
            ArrayTree::Children(array_trees) => {
                let len = ty.into_array_type().len() as usize;
                let mut value_vec = Vec::with_capacity(len);
                let child_ty = ty.into_array_type().get_element_type();
                for child in array_trees {
                    value_vec.push(self.convert_array_tree_to_global_init(child, child_ty)?);
                }
                let count = len.saturating_sub(array_trees.len());
                value_vec.extend(std::iter::repeat_with(|| child_ty.const_zero()).take(count));

                Ok(match child_ty {
                    BasicTypeEnum::ArrayType(array_type) => {
                        let values = value_vec
                            .into_iter()
                            .map(|x| x.into_array_value())
                            .collect::<Vec<_>>();
                        array_type.const_array(&values).into()
                    }
                    BasicTypeEnum::FloatType(float_type) => {
                        let values = value_vec
                            .into_iter()
                            .map(|x| x.into_float_value())
                            .collect::<Vec<_>>();
                        float_type.const_array(&values).into()
                    }
                    BasicTypeEnum::IntType(int_type) => {
                        let values = value_vec
                            .into_iter()
                            .map(|x| x.into_int_value())
                            .collect::<Vec<_>>();
                        int_type.const_array(&values).into()
                    }
                    _ => {
                        return Err(CodegenError::Unsupported(
                            "unsupported array element type".into(),
                        ));
                    }
                })
            }
            ArrayTree::Val(array_tree_value) => match array_tree_value {
                airyc_analyzer::array::ArrayTreeValue::ConstExpr(const_expr) => {
                    self.get_const_var_value(const_expr)
                }
                airyc_analyzer::array::ArrayTreeValue::Expr(expr) => self.get_const_var_value(expr),
                airyc_analyzer::array::ArrayTreeValue::Empty => Ok(ty.const_zero()),
            },
        }
    }

    /// Get (type, ptr) from IndexVal
    pub(crate) fn get_element_ptr_by_index_val(
        &mut self,
        index_val: &IndexVal,
    ) -> Result<(BasicTypeEnum<'ctx>, PointerValue<'ctx>, String)> {
        let name = name_text(
            &index_val
                .name()
                .ok_or(CodegenError::Missing("variable name"))?,
        )
        .ok_or(CodegenError::Missing("identifier"))?;
        let symbol = self
            .symbols
            .lookup_var(&name)
            .ok_or_else(|| CodegenError::UndefinedVar(name.clone()))?;
        let (mut ptr, mut cur_ntype) = (symbol.ptr, symbol.ty.clone());
        let mut cur_llvm_type = self.convert_ntype_to_type(&cur_ntype)?;

        let user_indices: Vec<_> = index_val
            .indices()
            .map(|e| self.compile_expr(e).map(|v| v.into_int_value()))
            .collect::<Result<Vec<_>>>()?;

        if user_indices.is_empty() {
            return Ok((cur_llvm_type, ptr, name));
        }

        let mut idx_iter = user_indices.into_iter().peekable();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());

        while idx_iter.peek().is_some() {
            match &cur_ntype {
                NType::Array(_, _) => {
                    // 收集连续的数组维度索引
                    let zero = self.context.i32_type().const_zero();
                    let mut indices = vec![zero];
                    let mut depth = 0;
                    let mut inner = &cur_ntype;

                    while let NType::Array(next_inner, _) = inner {
                        if let Some(idx) = idx_iter.next() {
                            indices.push(idx);
                            depth += 1;
                            inner = next_inner;
                        } else {
                            break;
                        }
                    }

                    ptr = unsafe {
                        self.builder
                            .build_gep(cur_llvm_type, ptr, &indices, "arr.gep")
                            .map_err(|_| CodegenError::LlvmBuild("gep failed"))?
                    };

                    // 更新类型：剥掉 depth 层数组
                    for _ in 0..depth {
                        cur_llvm_type = cur_llvm_type.into_array_type().get_element_type();
                        if let NType::Array(inner, _) = cur_ntype {
                            cur_ntype = *inner;
                        }
                    }
                }
                NType::Pointer(inner) => {
                    // 指针：load 后 GEP 一个索引
                    let pointee_ty = self.convert_ntype_to_type(inner)?;
                    let loaded_ptr = self
                        .builder
                        .build_load(ptr_ty, ptr, "ptr.load")
                        .map_err(|_| CodegenError::LlvmBuild("load ptr"))?
                        .into_pointer_value();

                    let idx = idx_iter.next().unwrap();
                    ptr = unsafe {
                        self.builder
                            .build_gep(pointee_ty, loaded_ptr, &[idx], "ptr.gep")
                            .map_err(|_| CodegenError::LlvmBuild("gep failed"))?
                    };

                    cur_ntype = *inner.clone();
                    cur_llvm_type = pointee_ty;
                }
                NType::Const(inner) => {
                    cur_ntype = *inner.clone();
                }
                _ => {
                    return Err(CodegenError::TypeMismatch(
                        "cannot index non-array/pointer".into(),
                    ));
                }
            }
        }

        Ok((cur_llvm_type, ptr, name))
    }

    /// Get constant value from analyzer
    pub(crate) fn get_const_var_value(&self, expr: &impl AstNode) -> Result<BasicValueEnum<'ctx>> {
        let value = self
            .analyzer
            .get_value(expr.syntax().text_range())
            .ok_or(CodegenError::Missing("constant value"))?;
        self.convert_value(value)
    }

    /// Convert any value to i1 boolean
    pub(crate) fn as_bool(&self, val: BasicValueEnum<'ctx>) -> Result<IntValue<'ctx>> {
        match val {
            BasicValueEnum::IntValue(i) => {
                if i.get_type().get_bit_width() == 1 {
                    Ok(i)
                } else {
                    self.builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            i,
                            i.get_type().const_int(0, false),
                            "inttobool",
                        )
                        .map_err(|_| CodegenError::LlvmBuild("int compare failed"))
                }
            }
            BasicValueEnum::FloatValue(f) => self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::ONE,
                    f,
                    f.get_type().const_float(0.0),
                    "floattoboolf",
                )
                .map_err(|_| CodegenError::LlvmBuild("float compare failed")),
            _ => Err(CodegenError::TypeMismatch("cannot convert to bool".into())),
        }
    }

    /// Convert `Value` to `BasicValueEnum`
    pub(crate) fn convert_value(&self, value: &Value) -> Result<BasicValueEnum<'ctx>> {
        match value {
            Value::Int(x) => Ok(self.context.i32_type().const_int(*x as u64, false).into()),
            Value::Float(x) => Ok(self.context.f32_type().const_float(*x as f64).into()),
            Value::Array(_) => Err(CodegenError::NotImplemented("array constant")),
            Value::Struct(_) => Err(CodegenError::NotImplemented("struct constant")),
            Value::Pointee(_, _) => Err(CodegenError::NotImplemented("pointer constant")),
        }
    }

    /// Zero-extend i1 to i32
    pub(crate) fn bool_to_i32(&self, val: IntValue<'ctx>) -> Result<IntValue<'ctx>> {
        self.builder
            .build_int_z_extend(val, self.context.i32_type(), "bool_ext")
            .map_err(|_| CodegenError::LlvmBuild("bool extend failed"))
    }

    /// Build int compare and convert result to i32
    pub(crate) fn build_int_cmp(
        &self,
        pred: IntPredicate,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>> {
        let cmp = self
            .builder
            .build_int_compare(pred, l, r, name)
            .map_err(|_| CodegenError::LlvmBuild("cmp"))?;
        self.bool_to_i32(cmp)
    }

    /// Build float compare
    pub(crate) fn build_float_cmp(
        &self,
        pred: FloatPredicate,
        l: FloatValue<'ctx>,
        r: FloatValue<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        self.builder
            .build_float_compare(pred, l, r, name)
            .map(|v| v.into())
            .map_err(|_| CodegenError::LlvmBuild("fcmp"))
    }

    /// Build unconditional branch if current block has no terminator
    pub(crate) fn branch_if_no_terminator(&self, target: BasicBlock<'ctx>) -> Result<()> {
        if self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(target)
                .map_err(|_| CodegenError::LlvmBuild("branch failed"))?;
        }
        Ok(())
    }

    /// Get size of type in bytes
    pub(crate) fn get_type_size(&self, ty: &NType) -> Result<u64> {
        match ty {
            NType::Int => Ok(4),
            NType::Float => Ok(4),
            NType::Pointer(_) => Ok(8),
            NType::Array(inner, count) => Ok(self.get_type_size(inner)? * (*count as u64)),
            NType::Const(inner) => self.get_type_size(inner),
            _ => Err(CodegenError::Unsupported("unknown type size".into())),
        }
    }
}
