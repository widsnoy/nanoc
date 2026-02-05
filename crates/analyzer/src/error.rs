use miette::Diagnostic;
use thiserror::Error;
use tools::TextRange;

use crate::{array::ArrayInitError, r#type::NType};

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum SemanticError {
    #[error("type mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::type_mismatch))]
    TypeMismatch {
        expected: NType,
        found: NType,
        #[label("type mismatch: expected {expected}, found {found}")]
        range: TextRange,
    },

    #[error("constant expression expected")]
    #[diagnostic(code(semantic::constant_expr_expected))]
    ConstantExprExpected {
        #[label("constant expression expected here")]
        range: TextRange,
    },

    #[error("variable '{name}' is already defined")]
    #[diagnostic(code(semantic::variable_defined))]
    VariableDefined {
        name: String,
        #[label("variable '{name}' is already defined")]
        range: TextRange,
    },

    #[error("function '{name}' is already defined")]
    #[diagnostic(code(semantic::function_defined))]
    FunctionDefined {
        name: String,
        #[label("function '{name}' is already defined")]
        range: TextRange,
    },

    #[error("variable '{name}' is not defined")]
    #[diagnostic(code(semantic::variable_undefined))]
    VariableUndefined {
        name: String,
        #[label("variable '{name}' is not defined")]
        range: TextRange,
    },

    #[error("variable '{name}' must be initialized")]
    #[diagnostic(code(semantic::expect_initial_val))]
    ExpectInitialVal {
        name: String,
        #[label("variable '{name}' must be initialized")]
        range: TextRange,
    },

    #[error("array initialization error: {message}")]
    #[diagnostic(code(semantic::array_error))]
    ArrayError {
        message: Box<ArrayInitError>,
        #[label("array initialization error: {message}")]
        range: TextRange,
    },

    #[error("struct '{name}' is already defined")]
    #[diagnostic(code(semantic::struct_defined))]
    StructDefined {
        name: String,
        #[label("struct '{name}' is already defined")]
        range: TextRange,
    },

    #[error("type is not defined")]
    #[diagnostic(code(semantic::type_undefined))]
    TypeUndefined {
        #[label("type is not defined")]
        range: TextRange,
    },

    #[error("field '{field_name}' not found in struct '{struct_name}'")]
    #[diagnostic(code(semantic::field_not_found))]
    FieldNotFound {
        struct_name: String,
        field_name: String,
        #[label("field '{field_name}' not found in struct '{struct_name}")]
        range: TextRange,
    },

    #[error("type {ty} is not a struct")]
    #[diagnostic(code(semantic::not_a_struct))]
    NotAStruct {
        ty: NType,
        #[label("type {ty} is not a struct")]
        range: TextRange,
    },

    #[error("type {ty} is not a struct pointer")]
    #[diagnostic(code(semantic::not_a_struct_pointer))]
    NotAStructPointer {
        ty: NType,
        #[label("type {ty} is not a struct pointer")]
        range: TextRange,
    },

    #[error("struct initialization field count mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::struct_init_field_count_mismatch))]
    StructInitFieldCountMismatch {
        expected: usize,
        found: usize,
        #[label("struct initialization field count mismatch: expected {expected}, found {found}")]
        range: TextRange,
    },

    #[error("can't apply operator '{op}' to type {ty}")]
    #[diagnostic(code(semantic::apply_op_on_type))]
    ApplyOpOnType {
        ty: NType,
        op: String,
        #[label("can't apply operator '{op}' to type {ty}")]
        range: TextRange,
    },

    #[error("function '{name}' is not defined")]
    #[diagnostic(code(semantic::function_undefined))]
    FunctionUndefined {
        name: String,
        #[label("function '{name}' is not defined")]
        range: TextRange,
    },

    #[error(
        "function '{function_name}' argument count mismatch: expected {expected}, found {found}"
    )]
    #[diagnostic(code(semantic::argument_count_mismatch))]
    ArgumentCountMismatch {
        function_name: String,
        expected: usize,
        found: usize,
        #[label(
            "function '{function_name}' argument count mismatch: expected {expected}, found {found}"
        )]
        range: TextRange,
    },

    #[error("can't assign to const variable '{name}'")]
    #[diagnostic(code(semantic::assign_to_const))]
    AssignToConst {
        name: String,
        #[label("'{name}' is const")]
        range: TextRange,
    },

    #[error("break statement outside loop")]
    #[diagnostic(code(semantic::break_outside_loop))]
    BreakOutsideLoop {
        #[label("break outside loop")]
        range: TextRange,
    },

    #[error("continue statement outside loop")]
    #[diagnostic(code(semantic::continue_outside_loop))]
    ContinueOutsideLoop {
        #[label("continue outside loop")]
        range: TextRange,
    },

    #[error("return type mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::return_type_mismatch))]
    ReturnTypeMismatch {
        expected: NType,
        found: NType,
        #[label("expected {expected}, found {found}")]
        range: TextRange,
    },

    #[error("not a left value")]
    #[diagnostic(code(semantic::invalid_lvalue))]
    NotALValue {
        #[label("not a left value")]
        range: TextRange,
    },

    #[error("can't take address of right value")]
    #[diagnostic(code(semantic::address_of_non_lvalue))]
    AddressOfRight {
        #[label("can't take address of right value")]
        range: TextRange,
    },
}

impl SemanticError {
    /// 获取错误的位置范围
    pub fn range(&self) -> &TextRange {
        match self {
            Self::TypeMismatch { range, .. }
            | Self::ConstantExprExpected { range }
            | Self::VariableDefined { range, .. }
            | Self::FunctionDefined { range, .. }
            | Self::VariableUndefined { range, .. }
            | Self::ExpectInitialVal { range, .. }
            | Self::ArrayError { range, .. }
            | Self::StructDefined { range, .. }
            | Self::TypeUndefined { range }
            | Self::FieldNotFound { range, .. }
            | Self::NotAStruct { range, .. }
            | Self::NotAStructPointer { range, .. }
            | Self::StructInitFieldCountMismatch { range, .. }
            | Self::FunctionUndefined { range, .. }
            | Self::ArgumentCountMismatch { range, .. }
            | Self::AssignToConst { range, .. }
            | Self::BreakOutsideLoop { range }
            | Self::ContinueOutsideLoop { range }
            | Self::ReturnTypeMismatch { range, .. }
            | Self::NotALValue { range }
            | Self::ApplyOpOnType { range, .. }
            | Self::AddressOfRight { range } => range,
        }
    }
}
