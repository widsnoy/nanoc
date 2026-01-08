use core::panic;
use std::collections::HashMap;

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use airyc_analyzer::array::ArrayTree;
use airyc_analyzer::r#type::NType;
use airyc_analyzer::value::Value;
use airyc_parser::ast::{AstNode, ConstIndexVal, IndexVal, Name, SyntaxToken};

use crate::llvm_ir::{LoopContext, Program, Symbol};

// /// 统计指针星号数量
// pub(crate) fn pointer_depth(ptr: &Pointer) -> usize {
//     ptr.syntax()
//         .children_with_tokens()
//         .filter_map(|it| it.into_token())
//         .filter(|t| t.kind() == SyntaxKind::STAR)
//         .count()
// }

// /// 给基本类型套上指针层级
// #[allow(deprecated)]
// pub(crate) fn apply_pointer<'ctx>(
//     base: BasicTypeEnum<'ctx>,
//     pointer: Option<Pointer>,
// ) -> BasicTypeEnum<'ctx> {
//     let ty = base;
//     if let Some(_ptr) = pointer {
//         panic!("指针未实现");
//     }
//     ty
// }

/// 提取变量定义中的 ident token  
pub(crate) fn get_ident_node(name: &ConstIndexVal) -> SyntaxToken {
    name.name().and_then(|n| n.ident()).unwrap()
}

/// 提取普通名字
pub(crate) fn name_text(name: &Name) -> String {
    name.ident().map(|t| t.text().to_string()).unwrap()
}

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// 新作用域
    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 离开作用域
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

    /// 查找变量（从内到外）
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

    /// 在基本块分配局部变量
    pub(crate) fn create_entry_alloca(
        &self,
        function: FunctionValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> PointerValue<'ctx> {
        let entry = function.get_first_basic_block().unwrap();
        let builder = self.context.create_builder();
        if let Some(instr) = entry.get_first_instruction() {
            builder.position_before(&instr);
        } else {
            builder.position_at_end(entry);
        }
        builder.build_alloca(ty, name).unwrap()
    }

    pub(crate) fn declare_sysy_runtime(&self) {
        let i32_type = self.context.i32_type();
        let void_type = self.context.void_type();
        let i32_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        // int getint()
        let fn_type = i32_type.fn_type(&[], false);
        self.module.add_function("getint", fn_type, None);

        // int getch()
        self.module.add_function("getch", fn_type, None);

        // int getarray(int a[])
        let fn_type = i32_type.fn_type(&[i32_ptr_type.into()], false);
        self.module.add_function("getarray", fn_type, None);

        // void putint(int a)
        let fn_type = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("putint", fn_type, None);

        // void putch(int a)
        self.module.add_function("putch", fn_type, None);

        // void putarray(int n, int a[])
        let fn_type = void_type.fn_type(&[i32_type.into(), i32_ptr_type.into()], false);
        self.module.add_function("putarray", fn_type, None);

        // void starttime()
        let fn_type = void_type.fn_type(&[], false);
        self.module.add_function("starttime", fn_type, None);

        // void stoptime()
        self.module.add_function("stoptime", fn_type, None);
    }

    /// convert `airyc_analyzer::r#Type::NType` to `BasicTypeEnum`
    pub(crate) fn convert_ntype_to_type(&self, ntype: &NType) -> BasicTypeEnum<'ctx> {
        match ntype {
            NType::Int => self.context.i32_type().into(),
            NType::Float => self.context.f32_type().into(),
            NType::Void => self.context.i8_type().into(),
            NType::Array(ntype, count) => {
                let inner = self.convert_ntype_to_type(ntype);
                inner.array_type(*count as u32).into()
            }
            NType::Pointer(_) => self.context.ptr_type(AddressSpace::default()).into(),
            NType::Struct(_) => todo!(),
            NType::Const(ntype) => self.convert_ntype_to_type(ntype),
        }
    }

    /// convert `array_tree` to `BasicValueEnum`, 用于全局变量初始化
    pub(crate) fn convert_array_tree_to_const_value(
        &self,
        tree: &ArrayTree,
        ty: BasicTypeEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        match tree {
            ArrayTree::Children(array_trees) => {
                let len = ty.into_array_type().len() as usize;
                let mut value_vec = Vec::with_capacity(len);
                let child_ty = ty.into_array_type().get_element_type();
                for child in array_trees {
                    value_vec.push(self.convert_array_tree_to_const_value(child, child_ty));
                }
                let count = len.saturating_sub(array_trees.len());
                value_vec.extend(std::iter::repeat_with(|| child_ty.const_zero()).take(count));

                match child_ty {
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
                    // BasicTypeEnum::PointerType(pointer_type) => {
                    //     let values = value_vec.into_iter().map(|x| x.into_pointer_value()).collect::<Vec<_>>();
                    //     pointer_type.const_array(&values).into()
                    // },
                    // BasicTypeEnum::StructType(struct_type) => {
                    //     let values = value_vec.into_iter().map(|x| x.into_struct_value()).collect::<Vec<_>>();
                    //     struct_type.const_array(&values).into()
                    // },
                    _ => unreachable!(),
                }
            }
            ArrayTree::Val(array_tree_value) => match array_tree_value {
                airyc_analyzer::array::ArrayTreeValue::ConstExpr(const_expr) => {
                    self.get_const_var_value(const_expr)
                }
                airyc_analyzer::array::ArrayTreeValue::Expr(expr) => self.get_const_var_value(expr),
                airyc_analyzer::array::ArrayTreeValue::Empty => ty.const_zero(),
            },
        }
    }

    /// 从 IndexVal 获取变量的 (type, ptr)
    pub(crate) fn get_element_ptr_by_index_val(
        &mut self,
        index_val: &IndexVal, // fixme: 可能有解引用
    ) -> (BasicTypeEnum<'ctx>, PointerValue<'ctx>, String) {
        let name = name_text(&index_val.name().expect("变量缺名"));
        let symbol = self.lookup_var(&name).expect("变量未定义");
        let (ptr, elem_ty) = (symbol.ptr, symbol.ty);
        let basic_type = self.convert_ntype_to_type(elem_ty);
        if !elem_ty.is_array() && !elem_ty.is_pointer() {
            return (basic_type, ptr, name);
        }

        // todo：后续多级指针需要多次 load, 先只处理函数形参的数组
        let (basic_type, zero) = if elem_ty.is_array() {
            (basic_type, Some(self.context.i32_type().const_zero()))
        } else {
            let NType::Pointer(inner) = elem_ty else {
                panic!();
            };
            (self.convert_ntype_to_type(inner), None)
        };

        let indices = zero
            .into_iter()
            .chain(
                index_val
                    .indices()
                    .map(|e| self.compile_expr(e).into_int_value()),
            )
            .collect::<Vec<_>>();

        let gep = unsafe {
            self.builder
                .build_gep(basic_type, ptr, &indices, "idx.gep")
                .expect("gep failed")
        };

        let mut final_ty = basic_type;

        // 比如调用函数 func(int a[]), 保存的是指针类型，前面已经拆了一层
        if indices.is_empty() {
            final_ty = self.context.ptr_type(AddressSpace::default()).into();
        } else {
            for _ in 0..indices.len() - 1 {
                final_ty = final_ty.into_array_type().get_element_type();
            }
        }
        (final_ty, gep, name)
    }

    /// 从 analyzer 获取常量的值
    pub(crate) fn get_const_var_value(&self, expr: &impl AstNode) -> BasicValueEnum<'ctx> {
        let value = self
            .analyzer
            .get_value(expr.syntax().text_range())
            .unwrap_or_else(|| panic!("{}", expr.syntax().text().to_string()));
        self.convert_value(value)
    }

    /// 将任意值转为 i1 布尔
    pub(crate) fn as_bool(&self, val: BasicValueEnum<'ctx>) -> IntValue<'ctx> {
        match val {
            BasicValueEnum::IntValue(i) => {
                if i.get_type().get_bit_width() == 1 {
                    i
                } else {
                    self.builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            i,
                            i.get_type().const_int(0, false),
                            "inttobool",
                        )
                        .unwrap()
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
                .unwrap(),
            _ => panic!("无法转换为布尔"),
        }
    }

    /// convert `Value` to `BasicValueEnum`
    pub(crate) fn convert_value(&self, value: &Value) -> BasicValueEnum<'ctx> {
        match value {
            Value::Int(x) => self.context.i32_type().const_int(*x as u64, false).into(),
            Value::Float(x) => self.context.f32_type().const_float(*x as f64).into(),
            Value::Array(_) => todo!(),
            Value::Struct(_) => todo!(),
            Value::Pointee(_, _) => todo!(),
        }
    }

    /// 将 i1 无符号扩展为 i32
    pub(crate) fn bool_to_i32(&self, val: IntValue<'ctx>) -> IntValue<'ctx> {
        self.builder
            .build_int_z_extend(val, self.context.i32_type(), "bool_ext")
            .unwrap()
    }
}
