#![allow(unused_assignments)] // FIXME: https://github.com/zkat/miette/pull/459

use miette::Diagnostic;
use parser::parse::ParserError;
use thiserror::Error;
use tools::TextRange;

use crate::{array::ArrayInitError, r#type::Ty};

#[derive(Debug, Clone)]
pub struct ArgumentTypeMismatchData {
    pub function_name: String,
    pub param_name: String,
    pub arg_index: usize,
    pub expected: Ty,
    pub found: Ty,
    pub range: TextRange,
}

impl std::fmt::Display for ArgumentTypeMismatchData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "function '{}' argument {} ('{}') type mismatch: expected {}, found {}",
            self.function_name, self.arg_index, self.param_name, self.expected, self.found
        )
    }
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum AnalyzeError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    ParserError(Box<ParserError>),

    #[error("type mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::type_mismatch))]
    TypeMismatch {
        expected: Ty,
        found: Ty,
        #[label("here")]
        range: TextRange,
    },

    #[error("constant expression expected")]
    #[diagnostic(code(semantic::constant_expr_expected))]
    ConstantExprExpected {
        #[label("here")]
        range: TextRange,
    },

    #[error("variable '{name}' is already defined")]
    #[diagnostic(code(semantic::variable_defined))]
    VariableDefined {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("function '{name}' is already defined")]
    #[diagnostic(code(semantic::function_defined))]
    FunctionDefined {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("function '{name}' have been implemented")]
    #[diagnostic(code(semantic::function_implemented))]
    FunctionImplemented {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("function '{name}' is not implemented")]
    #[diagnostic(code(semantic::function_unimplemented))]
    FunctionUnImplemented {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("variable '{name}' is not defined")]
    #[diagnostic(code(semantic::variable_undefined))]
    VariableUndefined {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("variable '{name}' must be initialized")]
    #[diagnostic(code(semantic::expect_initial_val))]
    ExpectInitialVal {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("array initialization error: {message}")]
    #[diagnostic(code(semantic::array_error))]
    ArrayError {
        message: Box<ArrayInitError>,
        #[label("here")]
        range: TextRange,
    },

    #[error("struct '{name}' is already defined")]
    #[diagnostic(code(semantic::struct_defined))]
    StructDefined {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("struct '{name}' undefined")]
    #[diagnostic(code(semantic::struct_undefined))]
    StructUndefined {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("field '{field_name}' not found in struct '{struct_name}'")]
    #[diagnostic(code(semantic::field_not_found))]
    FieldNotFound {
        struct_name: String,
        field_name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("type {ty} is not a struct")]
    #[diagnostic(code(semantic::not_a_struct))]
    NotAStruct {
        ty: Ty,
        #[label("here")]
        range: TextRange,
    },

    #[error("type {ty} is not a struct pointer")]
    #[diagnostic(code(semantic::not_a_struct_pointer))]
    NotAStructPointer {
        ty: Ty,
        #[label("here")]
        range: TextRange,
    },

    #[error("struct initialization field count mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::struct_init_field_count_mismatch))]
    StructInitFieldCountMismatch {
        expected: usize,
        found: usize,
        #[label("here")]
        range: TextRange,
    },

    #[error("can't apply operator '{op}' to type {ty}")]
    #[diagnostic(code(semantic::apply_op_on_type))]
    ApplyOpOnType {
        ty: Ty,
        op: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("invalid_void_usage")]
    #[diagnostic(
        code(semantic::invalid_void_usage),
        help("void type can only be used as function return type or pointer type")
    )]
    InvalidVoidUsage {
        #[label("here")]
        range: TextRange,
    },

    #[error("cannot dereference void pointer")]
    #[diagnostic(code(semantic::void_pointer_deref))]
    VoidPointerDeref {
        #[label("here")]
        range: TextRange,
    },

    #[error("function '{name}' is not defined")]
    #[diagnostic(code(semantic::function_undefined))]
    FunctionUndefined {
        name: String,
        #[label("here")]
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
        #[label("here")]
        range: TextRange,
    },

    #[error("{0}")]
    #[diagnostic(code(semantic::argument_type_mismatch))]
    ArgumentTypeMismatch(Box<ArgumentTypeMismatchData>),

    #[error("can't assign to const variable '{name}'")]
    #[diagnostic(code(semantic::assign_to_const))]
    AssignToConst {
        name: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("break statement outside loop")]
    #[diagnostic(code(semantic::break_outside_loop))]
    BreakOutsideLoop {
        #[label("here")]
        range: TextRange,
    },

    #[error("continue statement outside loop")]
    #[diagnostic(code(semantic::continue_outside_loop))]
    ContinueOutsideLoop {
        #[label("here")]
        range: TextRange,
    },

    #[error("return type mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::return_type_mismatch))]
    ReturnTypeMismatch {
        expected: Ty,
        found: Ty,
        #[label("here")]
        range: TextRange,
    },

    #[error("not a left value")]
    #[diagnostic(code(semantic::invalid_lvalue))]
    NotALValue {
        #[label("here")]
        range: TextRange,
    },

    #[error("can't take address of right value")]
    #[diagnostic(code(semantic::address_of_non_lvalue))]
    AddressOfRight {
        #[label("here")]
        range: TextRange,
    },

    #[error("circular dependency detected in module imports")]
    #[diagnostic(code(semantic::circular_dependency))]
    CircularDependency {
        #[label("this module is part of a circular dependency")]
        range: TextRange,
    },

    #[error("import path not found: {path}")]
    #[diagnostic(code(semantic::import_path_not_found))]
    ImportPathNotFound {
        path: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("symbol '{symbol}' not found in module '{module_path}'")]
    #[diagnostic(code(semantic::import_symbol_not_found))]
    ImportSymbolNotFound {
        symbol: String,
        module_path: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("symbol '{symbol}' conflicts with existing definition")]
    #[diagnostic(code(semantic::import_symbol_conflict))]
    ImportSymbolConflict {
        symbol: String,
        #[label("imported symbol conflicts with this definition")]
        range: TextRange,
    },

    #[error("recursive type `{struct_name}` has infinite size")]
    #[diagnostic(
        code(semantic::recursive_type),
        help("{}", cycle.join("->") )
    )]
    RecursiveType {
        struct_name: String,
        cycle: Vec<String>,
        #[label("here")]
        range: TextRange,
    },

    #[error("initializer type mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(semantic::initializer_mismatch))]
    InitializerMismatch {
        expected: String,
        found: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("cannot apply binary operator '{op}' to types {lhs} and {rhs}")]
    #[diagnostic(code(semantic::binary_op_type_mismatch))]
    BinaryOpTypeMismatch {
        op: String,
        lhs: Ty,
        rhs: Ty,
        #[label("here")]
        range: TextRange,
    },
}

impl AnalyzeError {
    /// 获取错误的位置范围
    pub fn range(&self) -> &TextRange {
        match self {
            Self::ParserError(e) => e.range(),
            Self::ArgumentTypeMismatch(data) => &data.range,
            Self::TypeMismatch { range, .. }
            | Self::ConstantExprExpected { range }
            | Self::VariableDefined { range, .. }
            | Self::FunctionDefined { range, .. }
            | Self::VariableUndefined { range, .. }
            | Self::ExpectInitialVal { range, .. }
            | Self::ArrayError { range, .. }
            | Self::StructDefined { range, .. }
            | Self::StructUndefined { range, .. }
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
            | Self::InvalidVoidUsage { range }
            | Self::VoidPointerDeref { range }
            | Self::AddressOfRight { range }
            | Self::FunctionImplemented { range, .. }
            | Self::FunctionUnImplemented { range, .. }
            | Self::CircularDependency { range }
            | Self::ImportPathNotFound { range, .. }
            | Self::ImportSymbolNotFound { range, .. }
            | Self::ImportSymbolConflict { range, .. }
            | Self::RecursiveType { range, .. }
            | Self::InitializerMismatch { range, .. }
            | Self::BinaryOpTypeMismatch { range, .. } => range,
        }
    }
}
