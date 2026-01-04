use core::panic;

use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, IntValue};
use nanoc_analyzer::array::ArrayTree;
use nanoc_analyzer::r#type::NType;
use nanoc_analyzer::value::Value;
use nanoc_parser::ast::{AstNode as _, ConstExpr, ConstIndexVal, Name, SyntaxToken};

use crate::llvm_ir::Program;

// /// 统计指针星号数量
// pub fn pointer_depth(ptr: &Pointer) -> usize {
//     ptr.syntax()
//         .children_with_tokens()
//         .filter_map(|it| it.into_token())
//         .filter(|t| t.kind() == SyntaxKind::STAR)
//         .count()
// }

// /// 给基本类型套上指针层级
// #[allow(deprecated)]
// pub fn apply_pointer<'ctx>(
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
pub fn get_ident_node(name: &ConstIndexVal) -> SyntaxToken {
    name.name().and_then(|n| n.ident()).unwrap()
}

/// 提取普通名字
pub fn name_text(name: &Name) -> String {
    name.ident().map(|t| t.text().to_string()).unwrap()
}

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// convert `nanoc_analyzer::r#Type::NType` to `BasicTypeEnum`
    pub fn convert_ntype_to_type(&self, ntype: &NType) -> BasicTypeEnum<'ctx> {
        match ntype {
            NType::Int => self.context.i32_type().into(),
            NType::Float => self.context.f32_type().into(),
            NType::Void => self.context.i8_type().into(),
            NType::Array(ntype, count) => {
                let inner = self.convert_ntype_to_type(ntype);
                inner.array_type(*count as u32).into()
            }
            NType::Pointer(_ntype) => todo!(),
            NType::Struct(_) => todo!(),
            NType::Const(ntype) => self.convert_ntype_to_type(ntype),
        }
    }

    /// convert `array_tree` to `BasicValueEnum`
    pub fn convert_array_tree_to_const_value(
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
                nanoc_analyzer::array::ArrayTreeValue::ConstExpr(const_expr) => {
                    self.get_const_var_value(const_expr)
                }
                nanoc_analyzer::array::ArrayTreeValue::Expr(_expr) => todo!(),
                nanoc_analyzer::array::ArrayTreeValue::Empty => ty.const_zero(),
            },
        }
    }

    /// 从 analyzer 获取常量的值
    pub fn get_const_var_value(&self, expr: &ConstExpr) -> BasicValueEnum<'ctx> {
        let value = self
            .analyzer
            .get_value(expr.syntax().text_range())
            .cloned()
            .unwrap_or_else(|| panic!("{}", expr.syntax().text().to_string()));
        self.convert_value(value)
    }

    /// 将任意值转为 i1 布尔
    pub fn as_bool(&self, val: BasicValueEnum<'ctx>) -> IntValue<'ctx> {
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
    pub fn convert_value(&self, value: Value) -> BasicValueEnum<'ctx> {
        match value {
            Value::Int(x) => self.context.i32_type().const_int(x as u64, false).into(),
            Value::Float(x) => self.context.f32_type().const_float(x.into()).into(),
            Value::Array(_) => todo!(),
            Value::Struct(_) => todo!(),
            Value::Symbol(_, _) => todo!(),
        }
    }

    /// 将 i1 无符号扩展为 i32
    pub fn bool_to_i32(&self, val: IntValue<'ctx>) -> IntValue<'ctx> {
        self.builder
            .build_int_z_extend(val, self.context.i32_type(), "bool_ext")
            .unwrap()
    }
}
