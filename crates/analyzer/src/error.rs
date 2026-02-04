use rowan::TextRange;

use crate::{array::ArrayInitError, r#type::NType};

#[derive(Debug)]
pub enum SemanticError {
    TypeMismatch {
        expected: NType,
        found: NType,
        range: TextRange,
    },
    ConstantExprExpected {
        range: TextRange,
    },
    VariableDefined {
        name: String,
        range: TextRange,
    },
    FunctionDefined {
        name: String,
        range: TextRange,
    },
    VariableUndefined {
        name: String,
        range: TextRange,
    },
    ExpectInitialVal {
        name: String,
        range: TextRange,
    },
    ArrayError {
        message: Box<ArrayInitError>,
        range: TextRange,
    },
    StructDefined {
        name: String,
        range: TextRange,
    },
    TypeUndefined {
        range: TextRange,
    },
    FieldNotFound {
        struct_name: String,
        field_name: String,
        range: TextRange,
    },
    NotAStruct {
        ty: NType,
        range: TextRange,
    },
    NotAStructPointer {
        ty: NType,
        range: TextRange,
    },
    /// Struct 初始化列表字段数量不匹配
    StructInitFieldCountMismatch {
        expected: usize,
        found: usize,
        range: TextRange,
    },
    /// 不能对 type 应用某种 op
    ApplyOpOnType {
        ty: NType,
        op: String,
    },
    /// 函数未定义
    FunctionUndefined {
        name: String,
        range: TextRange,
    },
    /// 函数参数数量不匹配
    ArgumentCountMismatch {
        function_name: String,
        expected: usize,
        found: usize,
        range: TextRange,
    },
    /// 赋值给 const 变量
    AssignToConst {
        name: String,
        range: TextRange,
    },
    /// break 在循环外使用
    BreakOutsideLoop {
        range: TextRange,
    },
    /// continue 在循环外使用
    ContinueOutsideLoop {
        range: TextRange,
    },
    /// 返回类型不匹配
    ReturnTypeMismatch {
        expected: NType,
        found: NType,
        range: TextRange,
    },
    /// 无效的左值
    InvalidLValue {
        range: TextRange,
    },
    /// 取地址操作的操作数不是左值
    AddressOfNonLvalue {
        range: TextRange,
    },
}
