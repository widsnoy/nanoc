use std::collections::HashMap;

use analyzer::array::ArrayTree;
use analyzer::r#type::Ty;
use analyzer::value::Value;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};
use syntax::ast::AstNode;
use tools::TextRange;

use crate::error::{CodegenError, Result};
use crate::llvm_ir::{LoopContext, Program, Symbol, SymbolTable};

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
    pub(crate) fn insert_var(&mut self, name: String, ptr: PointerValue<'ctx>, ty: &'a Ty) {
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

    /// Convert `NType` to `BasicTypeEnum`
    pub(crate) fn convert_ntype_to_type(&self, ntype: &Ty) -> Result<BasicTypeEnum<'ctx>> {
        match ntype {
            Ty::I32 => Ok(self.context.i32_type().into()),
            Ty::I8 => Ok(self.context.i8_type().into()),
            Ty::Bool => Ok(self.context.bool_type().into()),
            Ty::Void => Ok(self.context.i8_type().into()),
            Ty::Array(ntype, count) => {
                let inner = self.convert_ntype_to_type(ntype)?;
                let size = count.ok_or_else(|| {
                    CodegenError::NotImplemented("array with runtime size not supported")
                })?;
                Ok(inner.array_type(size as u32).into())
            }
            Ty::Pointer { .. } => Ok(self.context.ptr_type(AddressSpace::default()).into()),
            Ty::Struct {
                id: struct_id,
                name,
            } => {
                // 获取 struct 定义
                let struct_def = self
                    .analyzer
                    .get_struct_by_id(*struct_id)
                    .ok_or(CodegenError::NotImplemented("undefined struct"))?;

                // 转换字段类型
                let field_types: Vec<_> = struct_def
                    .fields
                    .iter()
                    .map(|field_id| {
                        let field = self.analyzer.fields.get(field_id.index).unwrap();
                        self.convert_ntype_to_type(&field.ty)
                    })
                    .collect::<Result<Vec<_>>>()?;

                // 如果有名称，创建命名 struct；否则创建匿名 struct
                if !name.is_empty() {
                    // 尝试获取已存在的命名 struct
                    if let Some(existing) = self.context.get_struct_type(name) {
                        return Ok(existing.into());
                    }

                    // 创建新的命名 struct
                    let opaque = self.context.opaque_struct_type(name);
                    opaque.set_body(&field_types, false);
                    Ok(opaque.into())
                } else {
                    // 回退到匿名 struct（向后兼容）
                    let struct_type = self.context.struct_type(&field_types, false);
                    Ok(struct_type.into())
                }
            }
            Ty::Const(ntype) => self.convert_ntype_to_type(ntype),
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
                    BasicTypeEnum::IntType(int_type) => {
                        let values = value_vec
                            .into_iter()
                            .map(|x| x.into_int_value())
                            .collect::<Vec<_>>();
                        int_type.const_array(&values).into()
                    }
                    BasicTypeEnum::StructType(struct_ty) => {
                        let values = value_vec
                            .into_iter()
                            .map(|x| x.into_struct_value())
                            .collect::<Vec<_>>();
                        struct_ty.const_array(&values).into()
                    }
                    _ => {
                        return Err(CodegenError::Unsupported(
                            "unsupported array element type".into(),
                        ));
                    }
                })
            }
            ArrayTree::Val(array_tree_value) => match array_tree_value {
                analyzer::array::ArrayTreeValue::Expr(expr_range) => {
                    self.get_const_var_value_by_range(*expr_range, None)
                }
                analyzer::array::ArrayTreeValue::Struct {
                    init_list: list_range,
                    ..
                } => self.get_const_var_value_by_range(*list_range, None),
                analyzer::array::ArrayTreeValue::Empty => Ok(ty.const_zero()),
            },
        }
    }

    pub(crate) fn calculate_index_op(
        &self,
        mut cur_ntype: Ty,
        mut cur_llvm_type: BasicTypeEnum<'ctx>,
        mut ptr: PointerValue<'ctx>,
        indices: Vec<IntValue<'ctx>>,
    ) -> Result<(BasicTypeEnum<'ctx>, PointerValue<'ctx>)> {
        let mut idx_iter = indices.into_iter().peekable();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());

        while idx_iter.peek().is_some() {
            match &cur_ntype {
                Ty::Array(_, _) => {
                    // 收集连续的数组维度索引
                    let zero = self.context.i32_type().const_zero();
                    let mut indices = vec![zero];
                    let mut depth = 0;
                    let mut inner = &cur_ntype;

                    while let Ty::Array(next_inner, _) = inner {
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
                        if let Ty::Array(inner, _) = cur_ntype {
                            cur_ntype = *inner;
                        }
                    }
                }
                Ty::Pointer { pointee, .. } => {
                    // 指针：load 后 GEP 一个索引
                    let pointee_ty = self.convert_ntype_to_type(pointee)?;
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

                    cur_ntype = *pointee.clone();
                    cur_llvm_type = pointee_ty;
                }
                Ty::Const(inner) => {
                    cur_ntype = *inner.clone();
                }
                _ => {
                    return Err(CodegenError::TypeMismatch(
                        "cannot index non-array/pointer".into(),
                    ));
                }
            }
        }
        Ok((cur_llvm_type, ptr))
    }

    /// Get constant value from analyzer
    /// 如果是 Array，保证 ty.is_some() == true
    pub(crate) fn get_const_var_value(
        &self,
        ast_node: &impl AstNode,
        ty: Option<BasicTypeEnum<'ctx>>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let value = self
            .analyzer
            .get_value_by_range(ast_node.text_range())
            .ok_or(CodegenError::Missing("constant value"))?;
        self.convert_value(value, ty)
    }

    /// Get constant value from analyzer
    /// 如果是 Array，保证 ty.is_some() == true
    pub(crate) fn get_const_var_value_by_range(
        &self,
        range: TextRange,
        ty: Option<BasicTypeEnum<'ctx>>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let value = self
            .analyzer
            .get_value_by_range(range)
            .ok_or(CodegenError::Missing("constant value"))?;
        self.convert_value(value, ty)
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
            BasicValueEnum::PointerValue(p) => {
                let i64_ty = self.context.i64_type();
                let ptr_as_int = self
                    .builder
                    .build_ptr_to_int(p, i64_ty, "ptr_to_int")
                    .map_err(|_| CodegenError::LlvmBuild("ptr_to_int failed"))?;
                self.builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        ptr_as_int,
                        i64_ty.const_zero(),
                        "ptr_to_bool",
                    )
                    .map_err(|_| CodegenError::LlvmBuild("int compare failed"))
            }
            _ => Err(CodegenError::Unsupported(
                "unsupported type for bool conversion".into(),
            )),
        }
    }

    /// Convert `Value` to `BasicValueEnum`
    /// 如果是 Array，保证 ty.is_some() == true
    pub(crate) fn convert_value(
        &self,
        value: &Value,
        ty: Option<BasicTypeEnum<'ctx>>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match value {
            Value::I32(x) => Ok(self.context.i32_type().const_int(*x as u64, false).into()),
            Value::I8(x) => Ok(self.context.i8_type().const_int(*x as u64, false).into()),
            Value::Bool(x) => Ok(self.context.bool_type().const_int(*x as u64, false).into()),
            Value::Array(tree) => self.convert_array_tree_to_global_init(tree, ty.unwrap()),
            Value::Struct(struct_id, fields) => {
                // 生成 struct 常量
                // 获取 struct 的 LLVM 类型
                let struct_name = self
                    .analyzer
                    .get_struct_by_id(*struct_id)
                    .map(|s| s.name)
                    .unwrap_or_default();
                let struct_ntype = Ty::Struct {
                    id: *struct_id,
                    name: struct_name,
                };
                let struct_ty = ty
                    .map(|t| t.into_struct_type())
                    .or_else(|| {
                        self.convert_ntype_to_type(&struct_ntype)
                            .ok()
                            .map(|t| t.into_struct_type())
                    })
                    .ok_or(CodegenError::NotImplemented("struct constant without type"))?;

                // 按字段顺序生成常量值
                let field_values: Vec<_> = fields
                    .iter()
                    .enumerate()
                    .map(|(idx, v)| {
                        let field_ty = struct_ty.get_field_type_at_index(idx as u32).unwrap();
                        self.convert_value(v, Some(field_ty))
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(struct_ty.const_named_struct(&field_values).into())
            }
            Value::StructZero(struct_id) => {
                // 生成 struct 零值
                let struct_def = self
                    .analyzer
                    .get_struct_by_id(*struct_id)
                    .ok_or(CodegenError::NotImplemented("undefined struct"))?;

                // 获取 struct 的 LLVM 类型
                let struct_ntype = Ty::Struct {
                    id: *struct_id,
                    name: struct_def.name.clone(),
                };
                let struct_llvm_ty = self
                    .convert_ntype_to_type(&struct_ntype)?
                    .into_struct_type();

                // 为每个字段生成零值
                let field_values: Vec<_> = struct_def
                    .fields
                    .iter()
                    .map(|field_id| {
                        let field = self.analyzer.fields.get(field_id.index).unwrap();
                        let field_llvm_ty = self.convert_ntype_to_type(&field.ty)?;
                        Ok(field_llvm_ty.const_zero())
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(struct_llvm_ty.const_named_struct(&field_values).into())
            }
            Value::Null => {
                // 生成 LLVM null 指针
                // 如果提供了类型，使用该类型；否则使用 void*
                let ptr_ty = ty
                    .map(|t| t.into_pointer_type())
                    .unwrap_or_else(|| self.context.ptr_type(inkwell::AddressSpace::default()));
                Ok(ptr_ty.const_null().into())
            }
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
        self.builder
            .build_int_compare(pred, l, r, name)
            .map_err(|_| CodegenError::LlvmBuild("cmp"))
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

    /// 将整数值统一转换为 i32（用于二元运算）
    pub(crate) fn cast_int_to_i32(&self, val: IntValue<'ctx>, ty: &Ty) -> Result<IntValue<'ctx>> {
        match ty.unwrap_const() {
            Ty::I32 => Ok(val), // 已经是 i32
            Ty::I8 => {
                // i8 → i32: sext
                let i32_ty = self.context.i32_type();
                self.builder
                    .build_int_s_extend(val, i32_ty, "i8_to_i32")
                    .map_err(|_| CodegenError::LlvmBuild("sext"))
            }
            Ty::Bool => {
                // bool (i1) → i32: zext
                let i32_ty = self.context.i32_type();
                self.builder
                    .build_int_z_extend(val, i32_ty, "bool_to_i32")
                    .map_err(|_| CodegenError::LlvmBuild("zext"))
            }
            _ => Ok(val),
        }
    }

    /// 将整数值转换为 i8（用于二元运算）
    pub(crate) fn cast_int_to_i8(&self, val: IntValue<'ctx>, ty: &Ty) -> Result<IntValue<'ctx>> {
        match ty.unwrap_const() {
            Ty::I8 => Ok(val), // 已经是 i8
            Ty::I32 => {
                // i32 → i8: trunc
                let i8_ty = self.context.i8_type();
                self.builder
                    .build_int_truncate(val, i8_ty, "i32_to_i8")
                    .map_err(|_| CodegenError::LlvmBuild("trunc"))
            }
            Ty::Bool => {
                // bool (i1) → i8: zext
                let i8_ty = self.context.i8_type();
                self.builder
                    .build_int_z_extend(val, i8_ty, "bool_to_i8")
                    .map_err(|_| CodegenError::LlvmBuild("zext"))
            }
            _ => Ok(val),
        }
    }

    /// 将整数值转换为 bool（用于逻辑运算和条件）
    pub(crate) fn cast_int_to_bool(&self, val: IntValue<'ctx>, ty: &Ty) -> Result<IntValue<'ctx>> {
        match ty.unwrap_const() {
            Ty::Bool => Ok(val), // 已经是 bool
            Ty::I32 => {
                // i32 → bool: icmp ne 0
                let zero = self.context.i32_type().const_zero();
                self.builder
                    .build_int_compare(IntPredicate::NE, val, zero, "i32_to_bool")
                    .map_err(|_| CodegenError::LlvmBuild("icmp"))
            }
            Ty::I8 => {
                // i8 → bool: icmp ne 0
                let zero = self.context.i8_type().const_zero();
                self.builder
                    .build_int_compare(IntPredicate::NE, val, zero, "i8_to_bool")
                    .map_err(|_| CodegenError::LlvmBuild("icmp"))
            }
            _ => Ok(val),
        }
    }

    /// 通用类型转换
    /// 整数向上转换
    pub(crate) fn cast_value(
        &self,
        val: BasicValueEnum<'ctx>,
        from_ty: &Ty,
        to_ty: &Ty,
    ) -> Result<BasicValueEnum<'ctx>> {
        let from = from_ty.unwrap_const();
        let to = to_ty.unwrap_const();

        // 相同类型，无需转换
        if from == to {
            return Ok(val);
        }

        let int_val = val.into_int_value();

        match (from, to) {
            // i8 → i32: sext
            (Ty::I8, Ty::I32) => Ok(self
                .builder
                .build_int_s_extend(int_val, self.context.i32_type(), "cast")
                .map_err(|_| CodegenError::LlvmBuild("sext"))?
                .into()),
            // bool → i32: zext
            (Ty::Bool, Ty::I32) => Ok(self
                .builder
                .build_int_z_extend(int_val, self.context.i32_type(), "cast")
                .map_err(|_| CodegenError::LlvmBuild("zext"))?
                .into()),
            // bool → i8: zext
            (Ty::Bool, Ty::I8) => Ok(self
                .builder
                .build_int_z_extend(int_val, self.context.i8_type(), "cast")
                .map_err(|_| CodegenError::LlvmBuild("zext"))?
                .into()),
            _ => Ok(val),
        }
    }
}
