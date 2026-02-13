use core::default::Default;
use std::path::PathBuf;

use parser::parse::Parser;
use vfs::Vfs;

use crate::error::AnalyzeError;
use crate::module::Module;
use crate::project::Project;

pub(crate) fn analyze(source: &str) -> Module {
    let parser = Parser::new(source);
    let (tree, errors) = parser.parse();

    if !errors.is_empty() {
        panic!("Parser errors: {:?}", errors);
    }

    let vfs = Vfs::default();

    let file_id = vfs.new_file(PathBuf::from("test.airy"), source.to_string());

    let mut module = Module::new(tree);
    module.file_id = file_id;

    Project::allocate_module_symbols(&mut module);

    module.analyze();

    module
}

#[test]
fn test_variable_declaration() {
    let source = r#"
    fn main() -> i32 {
        let x: const i32 = 1;
        let y: const i32 = x + 1;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_duplicate_variable_error() {
    let source = r#"
    fn main() -> i32 {
        let a: i32;
        let a: i32;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    match &module.semantic_errors[0] {
        AnalyzeError::VariableDefined { name, .. } => {
            assert_eq!(name, "a");
        }
        _ => panic!("Expected VariableDefined error"),
    }
}

#[test]
fn test_const_binary_operations() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = 5 + 3;
        let b: const i32 = 10 - 2;
        let c: const i32 = 4 * 3;
        let d: const i32 = 12 / 4;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_comparison_operations() {
    let source = r#"
    fn main() -> i32 {
        let a: const bool = 5 > 3;
        let b: const bool = 10 == 10;
        let c: const bool = 2 < 8;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_logical_operations() {
    let source = r#"
    fn main() -> i32 {
        let a: const bool = true && true;
        let b: const bool = false || true;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_unary_operations() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = -5;
        let b: const bool = !false;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_parenthesized_expression() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = (1 + 2) * 3;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_nested_scope_variables() {
    let source = r#"
    fn main() -> i32 {
        let a: i32;
        {
            let b: i32;
            let c: i32;
        }
        let d: i32;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_variable_shadowing() {
    let source = r#"
    fn main() -> i32 {
        let a: i32;
        {
            let a: i32;
        }
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_function_definition() {
    let source = r#"
    fn add(a: i32, b: i32) -> i32 {
        let result: i32;
    }

    fn main() -> i32 {
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
    assert_eq!(module.functions.len(), 2);
}

#[test]
fn test_function_parameters() {
    let source = r#"
    fn sum(a: i32, b: i32, c: i32) -> i32 {
        let x: i32;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_duplicate_function_parameters_error() {
    let source = r#"
    fn func(a: i32, a: i32) -> i32 {
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::VariableDefined { .. }
    ));
}

// ========== 函数调用参数检查测试 ==========

#[test]
fn test_function_argument_count_mismatch_too_few() {
    let source = r#"
    fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
    
    fn main() -> i32 {
        let x: i32 = add(1);
        return 0;
    }
    "#;
    let module = analyze(source);
    assert_eq!(module.semantic_errors.len(), 1);
    assert!(matches!(
        &module.semantic_errors[0],
        AnalyzeError::ArgumentCountMismatch {
            expected: 2,
            found: 1,
            ..
        }
    ));
}

#[test]
fn test_function_argument_count_mismatch_too_many() {
    let source = r#"
    fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
    
    fn main() -> i32 {
        let x: i32 = add(1, 2, 3);
        return 0;
    }
    "#;
    let module = analyze(source);
    assert_eq!(module.semantic_errors.len(), 1);
    assert!(matches!(
        &module.semantic_errors[0],
        AnalyzeError::ArgumentCountMismatch {
            expected: 2,
            found: 3,
            ..
        }
    ));
}

#[test]
fn test_function_argument_type_mismatch() {
    let source = r#"
    fn process(ptr: *mut i32) -> void {
        return;
    }
    
    fn main() -> i32 {
        let x: i32 = 10;
        process(x);
        return 0;
    }
    "#;
    let module = analyze(source);
    assert_eq!(module.semantic_errors.len(), 1);
    assert!(matches!(
        &module.semantic_errors[0],
        AnalyzeError::ArgumentTypeMismatch(_)
    ));
}

#[test]
fn test_function_call_correct() {
    let source = r#"
    fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
    
    fn main() -> i32 {
        let x: i32 = add(1, 2);
        return x;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_function_call_with_implicit_conversion() {
    let source = r#"
    fn process(x: i32) -> i32 {
        return x;
    }
    
    fn main() -> i32 {
        let b: bool = true;
        let result: i32 = process(b);
        return result;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_external_function_call() {
    let source = r#"
    fn external_func(a: i32, b: *const i8) -> i32;
    
    fn main() -> i32 {
        let result: i32 = external_func(42, null);
        return result;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_external_function_call_wrong_args() {
    let source = r#"
    fn external_func(a: i32, b: *const i8) -> i32;
    
    fn main() -> i32 {
        let result: i32 = external_func(42);
        return result;
    }
    "#;
    let module = analyze(source);
    assert_eq!(module.semantic_errors.len(), 1);
    assert!(matches!(
        &module.semantic_errors[0],
        AnalyzeError::ArgumentCountMismatch {
            expected: 2,
            found: 1,
            ..
        }
    ));
}

#[test]
fn test_const_propagation() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = 1;
        let b: const i32 = a + 2;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_non_const_propagation_error() {
    // 现在允许运行时初始化的 const 变量
    // let b: const i32 = a + 2; 是合法的，b 是运行时初始化的 const
    let source = r#"
    fn main() -> i32 {
        let a: i32 = 1;
        let b: const i32 = a + 2;
    }
    "#;
    let module = analyze(source);
    // 不再报错，因为允许运行时初始化
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_int_arithmetic() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = 1 + 2;
        let b: const i32 = 10 - 3;
        let c: const i32 = 2 * 3;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_expression_with_multiple_operators() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = 1 + 2 * 3;
        let b: const i32 = (1 + 2) * 3;
        let c: const i32 = 10 - 5 - 2;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_nested_scopes_with_blocks() {
    let source = r#"
    fn main() -> i32 {
        let a: i32;
        {
            let b: i32;
            {
                let c: i32;
            }
        }
        {
            let d: i32;
        }
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_multiple_functions() {
    let source = r#"
    fn func1() {
        let a: i32;
    }

    fn func2(x: i32) -> i32 {
        let b: i32;
    }

    fn func3(a: i32, b: i32) -> i32 {
        let c: i32;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
    assert_eq!(module.functions.len(), 3);
}

#[test]
fn test_const_modulo_operation() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = 10 % 3;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_expression_expected_error() {
    let source = r#"
    let x: i32 = 5;
    fn foo(arr: *mut [i32; x]) {}
    fn main() -> i32 {
        return 0;
    }
    "#;
    let module = analyze(source);
    // Should error: non-constant expression in array size
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_global_const_propagation() {
    let source = r#"
    let x: const i32 = 233;
    let y: const i32 = x + 1;
    "#;
    let module = analyze(source);
    dbg!(&module.semantic_errors);
    dbg!(&module.value_table);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_break_inside_loop() {
    let source = r#"
    fn main() -> i32 {
        while (1) {
            break;
        }
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_break_outside_loop_error() {
    let source = r#"
    fn main() -> i32 {
        break;
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    match &module.semantic_errors[0] {
        AnalyzeError::BreakOutsideLoop { .. } => {}
        _ => panic!("Expected BreakOutsideLoop error"),
    }
}

#[test]
fn test_continue_outside_loop_error() {
    let source = r#"
    fn main() -> i32 {
        continue;
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    match &module.semantic_errors[0] {
        AnalyzeError::ContinueOutsideLoop { .. } => {}
        _ => panic!("Expected ContinueOutsideLoop error"),
    }
}

#[test]
fn test_nested_loop_break() {
    let source = r#"
    fn main() -> i32 {
        while (1) {
            while (1) {
                break;
            }
            continue;
        }
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_function_call_valid() {
    let source = r#"
    fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
    fn main() -> i32 {
        let x: i32 = add(1, 2);
        return x;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_function_undefined_error() {
    let source = r#"
    fn main() -> i32 {
        let x: i32 = undefined_func(1, 2);
        return x;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    match &module.semantic_errors[0] {
        AnalyzeError::FunctionUndefined { name, .. } => {
            assert_eq!(name, "undefined_func");
        }
        _ => panic!("Expected FunctionUndefined error"),
    }
}

// #[test]
// fn test_function_argument_count_mismatch() {
//     let source = r#"
//     fn add(a: i32, b: i32) -> i32 {
//         return a + b;
//     }
//     fn main() -> i32 {
//         let x: i32 = add(1);
//         return x;
//     }
//     "#;
//     let module = analyze(source);
//     assert!(!module.semantic_errors.is_empty());
//     match &module.semantic_errors[0] {
//         SemanticError::ArgumentCountMismatch {
//             function_name,
//             expected,
//             found,
//             ..
//         } => {
//             assert_eq!(function_name, "add");
//             assert_eq!(*expected, 2);
//             assert_eq!(*found, 1);
//         }
//         _ => panic!("Expected ArgumentCountMismatch error"),
//     }
// }
#[test]
fn test_assign_to_const_error() {
    let source = r#"
    fn main() -> i32 {
        let x: const i32 = 1;
        x = 2;
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    match &module.semantic_errors[0] {
        AnalyzeError::AssignToConst { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!("Expected AssignToConst error"),
    }
}

#[test]
fn test_assign_to_mutable_variable() {
    let source = r#"
    fn main() -> i32 {
        let x: i32 = 1;
        x = 2;
        return x;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_return_type_match() {
    let source = r#"
    fn foo() -> i32 {
        return 42;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_return_void_from_void_function() {
    let source = r#"
    fn foo() {
        return;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_variable_read_reference() {
    let source = r#"
    fn main() -> i32 {
        let x: i32 = 1;
        let y: i32 = x + 1;
        return y;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
    // 检查 variable_map 中有多个条目（定义 + 引用）
    assert!(module.variable_map.len() >= 2);
}

// FIXME:
// #[test]
// fn test_variable_write_reference() {
//     let source = r#"
//     fn main() -> i32 {
//         let x: i32 = 1;
//         x = 2;
//         return x;
//     }
//     "#;
//     let module = analyze(source);
//     assert!(module.semantic_errors.is_empty());
//     // 检查有 Write 引用被记录
//     let has_write = module
//         .variables
//         .iter()
//         .any(|(_, v)| v.tag == crate::module::VariableTag::Write);
//     assert!(has_write, "Should have Write reference recorded");
// }

#[test]
fn test_undefined_variable_error() {
    let source = r#"
    fn main() -> i32 {
        return undefined_var;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    match &module.semantic_errors[0] {
        AnalyzeError::VariableUndefined { name, .. } => {
            assert_eq!(name, "undefined_var");
        }
        _ => panic!("Expected VariableUndefined error"),
    }
}

// ========== void 类型限制测试 ==========

#[test]
fn test_void_variable_error() {
    let source = r#"
    fn main() -> i32 {
        let x: void;
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::InvalidVoidUsage { .. }
    ));
}

#[test]
fn test_void_parameter_error() {
    let source = r#"
    fn foo(x: void) -> i32 {
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::InvalidVoidUsage { .. }
    ));
}

#[test]
fn test_void_struct_field_error() {
    let source = r#"
    struct Foo {
        x: void,
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::InvalidVoidUsage { .. }
    ));
}

#[test]
fn test_void_array_element_error() {
    let source = r#"
    fn main() -> i32 {
        let arr: [void; 10];
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::InvalidVoidUsage { .. }
    ));
}

#[test]
fn test_void_pointer_deref_error() {
    let source = r#"
    fn main() -> i32 {
        let p: *mut void;
        let x: i32 = *p;
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    // 应该有 VoidPointerDeref 错误
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::VoidPointerDeref { .. }
    ));
}

#[test]
fn test_void_return_type_ok() {
    let source = r#"
    fn foo() -> void {
        return;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_void_pointer_ok() {
    let source = r#"
    let p: *mut void = null;
    let q: *const void = null;
    
    fn main() -> i32 {
        return 0;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_void_variable_error() {
    let source = r#"
    fn main() -> i32 {
        let x: const void;
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
    assert!(matches!(
        module.semantic_errors[0],
        AnalyzeError::InvalidVoidUsage { .. }
    ));
}

// ========== 可变参数测试 ==========

#[test]
fn test_variadic_function_call() {
    let source = r#"
    fn printf(format: *const i8, ...) -> i32;
    
    fn main() -> i32 {
        printf(null, 1, 2, 3);
        return 0;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_variadic_function_too_few_args() {
    let source = r#"
    fn printf(format: *const i8, ...) -> i32;
    
    fn main() -> i32 {
        printf();
        return 0;
    }
    "#;
    let module = analyze(source);
    assert_eq!(module.semantic_errors.len(), 1);
    assert!(matches!(
        &module.semantic_errors[0],
        AnalyzeError::ArgumentCountMismatch {
            expected: 1,
            found: 0,
            ..
        }
    ));
}

#[test]
fn test_variadic_function_with_impl() {
    let source = r#"
    fn my_func(x: i32, ...) -> i32 {
        return x;
    }
    
    fn main() -> i32 {
        let result: i32 = my_func(42, 1, 2, 3);
        return result;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_variadic_function_no_fixed_params() {
    let source = r#"
    fn varargs(...) -> i32;
    
    fn main() -> i32 {
        varargs(1, 2, 3);
        return 0;
    }
    "#;
    let module = analyze(source);
    if !module.semantic_errors.is_empty() {
        eprintln!("Unexpected errors: {:?}", module.semantic_errors);
    }
    // 允许没有固定参数的可变参数函数
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_variadic_function_type_check() {
    let source = r#"
    fn process(x: i32, ...) -> i32;
    
    fn main() -> i32 {
        let ptr: *mut i32 = null;
        process(ptr);
        return 0;
    }
    "#;
    let module = analyze(source);
    assert_eq!(module.semantic_errors.len(), 1);
    assert!(matches!(
        &module.semantic_errors[0],
        AnalyzeError::ArgumentTypeMismatch(_)
    ));
}

// 新整数类型和字符字面量测试

#[test]
fn test_u8_basic() {
    let source = r#"
        fn main() -> i32 {
            let a: u8 = 0u8;
            let b: u8 = 255u8;
            let c: u8 = 100u8;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_u8_overflow() {
    let source = r#"
        fn main() -> i32 {
            let a: u8 = 256u8;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_u32_basic() {
    let source = r#"
        fn main() -> i32 {
            let a: u32 = 0u32;
            let b: u32 = 4294967295u32;
            let c: u32 = 1000000u32;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_i64_basic() {
    let source = r#"
        fn main() -> i32 {
            let a: i64 = -9223372036854775808i64;
            let b: i64 = 9223372036854775807i64;
            let c: i64 = 0i64;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_u64_basic() {
    let source = r#"
        fn main() -> i32 {
            let a: u64 = 0u64;
            let b: u64 = 18446744073709551615u64;
            let c: u64 = 1000000000000u64;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_type_conversion_signed() {
    let source = r#"
        fn test_i8_to_i32(x: i8) -> i32 {
            return x;
        }
        fn test_i8_to_i64(x: i8) -> i64 {
            return x;
        }
        fn test_i32_to_i64(x: i32) -> i64 {
            return x;
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_type_conversion_unsigned() {
    let source = r#"
        fn test_u8_to_u32(x: u8) -> u32 {
            return x;
        }
        fn test_u8_to_u64(x: u8) -> u64 {
            return x;
        }
        fn test_u32_to_u64(x: u32) -> u64 {
            return x;
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_type_conversion_invalid_signed_to_unsigned() {
    let source = r#"
        fn test(x: i32) -> u32 {
            return x;
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_type_conversion_invalid_unsigned_to_signed() {
    let source = r#"
        fn test(x: u32) -> i32 {
            return x;
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_arithmetic_u8() {
    let source = r#"
        fn main() -> i32 {
            let a: u8 = 10u8;
            let b: u8 = 20u8;
            let c: u8 = a + b;
            let d: u8 = b - a;
            let e: u8 = a * b;
            let f: u8 = b / a;
            let g: u8 = b % a;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_arithmetic_u32() {
    let source = r#"
        fn main() -> i32 {
            let a: u32 = 1000u32;
            let b: u32 = 2000u32;
            let c: u32 = a + b;
            let d: u32 = b - a;
            let e: u32 = a * b;
            let f: u32 = b / a;
            let g: u32 = b % a;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_arithmetic_i64() {
    let source = r#"
        fn main() -> i32 {
            let a: i64 = 1000i64;
            let b: i64 = 2000i64;
            let c: i64 = a + b;
            let d: i64 = b - a;
            let e: i64 = a * b;
            let f: i64 = b / a;
            let g: i64 = b % a;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_arithmetic_u64() {
    let source = r#"
        fn main() -> i32 {
            let a: u64 = 1000u64;
            let b: u64 = 2000u64;
            let c: u64 = a + b;
            let d: u64 = b - a;
            let e: u64 = a * b;
            let f: u64 = b / a;
            let g: u64 = b % a;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_comparison_u8() {
    let source = r#"
        fn main() -> i32 {
            let a: u8 = 10u8;
            let b: u8 = 20u8;
            let c: bool = a < b;
            let d: bool = a <= b;
            let e: bool = a > b;
            let f: bool = a >= b;
            let g: bool = a == b;
            let h: bool = a != b;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_comparison_mixed_types_error() {
    let source = r#"
        fn main() -> i32 {
            let a: i32 = 10;
            let b: u32 = 20u32;
            let c: bool = a < b;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_char_literal_basic() {
    let source = r#"
        fn main() -> i32 {
            let a: u8 = 'a';
            let b: u8 = 'Z';
            let c: u8 = '0';
            let d: u8 = ' ';
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_bool_to_signed_conversion() {
    let source = r#"
        fn test_bool_to_i8(x: bool) -> i8 {
            return x;
        }
        fn test_bool_to_i32(x: bool) -> i32 {
            return x;
        }
        fn test_bool_to_i64(x: bool) -> i64 {
            return x;
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_bool_to_unsigned_error() {
    let source = r#"
        fn test(x: bool) -> u32 {
            return x;
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_const_arithmetic_overflow_u8() {
    let source = r#"
        fn main() -> i32 {
            let a: const u8 = 200u8 + 100u8;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_const_arithmetic_overflow_i64() {
    let source = r#"
        fn main() -> i32 {
            let a: const i64 = 9223372036854775807i64 + 1i64;
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(!module.semantic_errors.is_empty());
}

#[test]
fn test_array_with_new_types() {
    let source = r#"
        fn main() -> i32 {
            let arr1: [u8; 3] = {1u8, 2u8, 3u8};
            let arr2: [u32; 2] = {100u32, 200u32};
            let arr3: [i64; 2] = {100i64, 200i64};
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_function_params_new_types() {
    let source = r#"
        fn test_u8(a: u8, b: u8) -> u8 {
            return a + b;
        }
        fn test_u32(a: u32, b: u32) -> u32 {
            return a + b;
        }
        fn test_i64(a: i64, b: i64) -> i64 {
            return a + b;
        }
        fn test_u64(a: u64, b: u64) -> u64 {
            return a + b;
        }
        fn main() -> i32 {
            let r1: u8 = test_u8(10u8, 20u8);
            let r2: u32 = test_u32(100u32, 200u32);
            let r3: i64 = test_i64(100i64, 200i64);
            let r4: u64 = test_u64(100u64, 200u64);
            return 0;
        }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}
