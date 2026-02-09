use core::default::Default;
use std::path::PathBuf;

use parser::parse::Parser;

use crate::error::SemanticError;
use crate::header::HeaderAnalyzer;
use crate::module::Module;
use crate::project::Project;

fn analyze(source: &str) -> Module {
    let parser = Parser::new(source);
    let (tree, errors) = parser.parse();

    if !errors.is_empty() {
        panic!("Parser errors: {:?}", errors);
    }

    let project = Project::default();

    let file_id = project
        .vfs
        .new_file(PathBuf::from("test.airy"), source.to_string());

    let mut module = Module::new(tree);
    module.file_id = file_id;

    Project::allocate_module_symbols(&mut module);

    let module_imports =
        HeaderAnalyzer::collect_module_imports(&module, file_id, &project.vfs, &project.modules);

    HeaderAnalyzer::apply_module_imports(&mut module, module_imports);

    Project::fill_definitions(&mut module);

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
        SemanticError::VariableDefined { name, .. } => {
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
        let a: const i32 = 5 > 3;
        let b: const i32 = 10 == 10;
        let c: const i32 = 2 < 8;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

#[test]
fn test_const_logical_operations() {
    let source = r#"
    fn main() -> i32 {
        let a: const i32 = 1 && 1;
        let b: const i32 = 0 || 1;
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
        let b: const i32 = !0;
    }
    "#;
    let module = analyze(source);
    dbg!(&module.semantic_errors);
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
    fn sum(a: i32, b: f32, c: i32) -> i32 {
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
fn test_const_float_arithmetic() {
    let source = r#"
    fn main() -> i32 {
        let a: const f32 = 1.5 + 2.5;
        let b: const f32 = 10.0 - 3.5;
        let c: const f32 = 2.0 * 3.5;
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

    fn func3(a: f32, b: i32) -> f32 {
        let c: f32;
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
        SemanticError::BreakOutsideLoop { .. } => {}
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
        SemanticError::ContinueOutsideLoop { .. } => {}
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
        SemanticError::FunctionUndefined { name, .. } => {
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
fn test_builtin_function_call() {
    let source = r#"
    fn main() -> i32 {
        let x: i32 = getint();
        putint(x);
        return 0;
    }
    "#;
    let module = analyze(source);
    assert!(module.semantic_errors.is_empty());
}

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
        SemanticError::AssignToConst { name, .. } => {
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
        SemanticError::VariableUndefined { name, .. } => {
            assert_eq!(name, "undefined_var");
        }
        _ => panic!("Expected VariableUndefined error"),
    }
}
