use parser::parse::Parser;
use parser::visitor::Visitor;

use crate::module::{Module, SemanticError};

fn analyze(source: &str) -> Module {
    let parser = Parser::new(source);
    let (tree, errors) = parser.parse();

    if !errors.is_empty() {
        panic!("Parser errors: {:?}", errors);
    }

    let ast = Parser::new_root(tree);
    // dbg!(&ast);
    let mut module = Module::default();
    module.walk(&ast);
    module
}

#[test]
fn test_variable_declaration() {
    let source = r#"
    int main() {
        const int x = 1, y = x + 1;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_duplicate_variable_error() {
    let source = r#"
    int main() {
        int a;
        int a;
    }
    "#;
    let module = analyze(source);
    assert!(!module.analyzing.errors.is_empty());
    match &module.analyzing.errors[0] {
        SemanticError::VariableDefined { name, .. } => {
            assert_eq!(name, "a");
        }
        _ => panic!("Expected VariableDefined error"),
    }
}

#[test]
fn test_const_binary_operations() {
    let source = r#"
    int main() {
        const int a = 5 + 3;
        const int b = 10 - 2;
        const int c = 4 * 3;
        const int d = 12 / 4;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_comparison_operations() {
    let source = r#"
    int main() {
        const int a = 5 > 3;
        const int b = 10 == 10;
        const int c = 2 < 8;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_logical_operations() {
    let source = r#"
    int main() {
        const int a = 1 && 1;
        const int b = 0 || 1;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_unary_operations() {
    let source = r#"
    int main() {
        const int a = -5;
        const int b = !0;
    }
    "#;
    let module = analyze(source);
    dbg!(&module.analyzing.errors);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_parenthesized_expression() {
    let source = r#"
    int main() {
        const int a = (1 + 2) * 3;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_nested_scope_variables() {
    let source = r#"
    int main() {
        int a;
        {
            int b;
            int c;
        }
        int d;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_variable_shadowing() {
    let source = r#"
    int main() {
        int a;
        {
            int a;
        }
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_function_definition() {
    let source = r#"
    int add(int a, int b) {
        int result;
    }

    int main() {
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
    assert_eq!(module.functions.len(), 2);
}

#[test]
fn test_function_parameters() {
    let source = r#"
    int sum(int a, float b, int c) {
        int x;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_duplicate_function_parameters_error() {
    let source = r#"
    int func(int a, int a) {
    }
    "#;
    let module = analyze(source);
    assert!(!module.analyzing.errors.is_empty());
}

#[test]
fn test_const_propagation() {
    let source = r#"
    int main() {
        const int a = 1;
        const int b = a + 2;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_non_const_propagation_error() {
    // 现在允许运行时初始化的 const 变量
    // const int b = a + 2; 是合法的，b 是运行时初始化的 const
    let source = r#"
    int main() {
        int a = 1;
        const int b = a + 2;
    }
    "#;
    let module = analyze(source);
    // 不再报错，因为允许运行时初始化
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_float_arithmetic() {
    let source = r#"
    int main() {
        const float a = 1.5 + 2.5;
        const float b = 10.0 - 3.5;
        const float c = 2.0 * 3.5;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_expression_with_multiple_operators() {
    let source = r#"
    int main() {
        const int a = 1 + 2 * 3;
        const int b = (1 + 2) * 3;
        const int c = 10 - 5 - 2;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_nested_scopes_with_blocks() {
    let source = r#"
    int main() {
        int a;
        {
            int b;
            {
                int c;
            }
        }
        {
            int d;
        }
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_multiple_functions() {
    let source = r#"
    void func1() {
        int a;
    }

    int func2(int x) {
        int b;
    }

    float func3(float a, int b) {
        float c;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
    assert_eq!(module.functions.len(), 3);
}

#[test]
fn test_const_modulo_operation() {
    let source = r#"
    int main() {
        const int a = 10 % 3;
    }
    "#;
    let module = analyze(source);
    assert!(module.analyzing.errors.is_empty());
}

#[test]
fn test_const_expression_expected_error() {
    let source = r#"
    int x = 5;
    void foo(int arr[][x]) {}
    int main() {
        return 0;
    }
    "#;
    let module = analyze(source);
    // Should error: non-constant expression in array size
    assert!(!module.analyzing.errors.is_empty());
}
