use core::panic;

use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{builder::Builder, context::Context};
use nanoc_parser::ast::{AstNode, ConstIndexVal, Name, Pointer};
use nanoc_parser::syntax_kind::SyntaxKind;

/// 统计指针星号数量
pub fn pointer_depth(ptr: &Pointer) -> usize {
    ptr.syntax()
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .filter(|t| t.kind() == SyntaxKind::STAR)
        .count()
}

/// 给基本类型套上指针层级
#[allow(deprecated)]
pub fn apply_pointer<'ctx>(
    base: BasicTypeEnum<'ctx>,
    pointer: Option<Pointer>,
) -> BasicTypeEnum<'ctx> {
    let ty = base;
    if let Some(_ptr) = pointer {
        panic!("指针未实现");
    }
    ty
}

/// 提取常量/变量名字
pub fn const_name(name: &ConstIndexVal) -> String {
    name.name()
        .and_then(|n| n.ident())
        .map(|t| t.text().to_string())
        .expect("获取名字失败")
}

/// 提取普通名字
pub fn name_text(name: &Name) -> String {
    name.ident()
        .map(|t| t.text().to_string())
        .expect("获取标识符失败")
}

/// 提取数组维度
pub fn const_index_dims(index: &ConstIndexVal) -> Option<Vec<u32>> {
    if index.indices().count() == 0 {
        return None;
    }
    todo!("ir 先实现一个计算器");
    // let mut dims = Vec::new();
    // for c in index.indices() {
    //     let expr = c.expr()?;
    //     //let val = match expr {};
    //     dims.push(val as u32);
    // }
    // Some(dims)
}

/// 根据维度包装数组类型
pub fn wrap_array_dims<'ctx>(base: BasicTypeEnum<'ctx>, dims: &[u32]) -> BasicTypeEnum<'ctx> {
    dims.iter()
        .rev()
        .fold(base, |ty, d| ty.array_type(*d).into())
}

/// 将任意值转为 i1 布尔
pub fn as_bool<'ctx>(
    builder: &Builder<'ctx>,
    _context: &'ctx Context,
    val: BasicValueEnum<'ctx>,
) -> IntValue<'ctx> {
    match val {
        BasicValueEnum::IntValue(i) => {
            if i.get_type().get_bit_width() == 1 {
                i
            } else {
                builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        i,
                        i.get_type().const_int(0, false),
                        "inttobool",
                    )
                    .unwrap()
            }
        }
        BasicValueEnum::FloatValue(f) => builder
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
