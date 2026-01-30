use rowan::SyntaxNode;

use crate::{parser::Parser, syntax_kind::NanocLanguage};

fn try_it(source: &str) -> SyntaxNode<NanocLanguage> {
    let parser = Parser::new(source);
    let (tree, errors) = parser.parse();

    if !errors.is_empty() {
        panic!("Parser errors: {:?}", errors);
    }

    Parser::new_root(tree)
}

#[test]
fn test_declarations() {
    let source = r#"
    const int A = 1;
    const float B = 2.0, C = 3.0;
    int a;
    float b = 1.0;
    struct MyStruct s;
    int *p;
    int arr[10];
    int arr2[2][3];
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_initializers() {
    let source = r#"
    int a = 1;
    int b = {1, 2};
    int c = {{1}, {2}};
    const int cb = {1, 2};
    const int cb[2][3] = {{1,2,3}, {4,5,6}};
    const int d = 1 + 2 * 3;
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_functions() {
    let source = r#"
    void func1() {}
    int func2(int a) {}
    int func3(int a, float b) {}
    int *func4(int *p, int arr[]) {}
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_expressions() {
    // Wrapped in a var decl because we can't parse bare expressions at root
    let source = r#"
    int x = a + b * c;
    int y = (a + b) * c;
    int z = a || b && c;
    int w = a == b;
    int rel = a < b;
    int unary = -a + !b;
    int ptr = *p + &x;
    int arr = a[1][2];
    int call = foo(a, b);
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_complex_mix() {
    let source = r#"
    const int MAX = 100;
    struct Point p;

    int main(int argc, int *argv[]) {
        int a = 1;
        int *ptr = &a;
        const int b = MAX;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_if_statement() {
    let source = r#"
    void test() {
        if (a) {
            return;
        }
        if (a) return; else return 1;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_while_statement() {
    let source = r#"
    void test() {
        while (1) {
            break;
            continue;
        }
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_return_statement() {
    let source = r#"
    void test() {
        return;
        return 1;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_block_statement() {
    let source = r#"
    void test() {
        {
            int nested;
        }
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_assign_statement() {
    let source = r#"
    void test() {
        a = 1;
        *p = 2;
        arr[0] = 3;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_expr_statement() {
    let source = r#"
    void test() {
        func();
        a + b;
        ;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_comments() {
    let source = r#"
    /* normal comment */
    /* comment with * inside */
    /* comment with / inside */
    /* comment with ** inside */
    /* **/
    /****/
    /**/
    int a = 1;
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_array_init() {
    let source = r#"
    const int a[3] = {1, 2, 3};
    const int a[2][3] = {{1, 2, 3}, {4, 5}};
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_postfix_expressions() {
    let source = r#"
    int x = s.field;
    int y = p->member;
    int z = s.a.b;
    int w = p->a->b;
    int mixed = arr[0].field;
    int complex = func().member;
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}
