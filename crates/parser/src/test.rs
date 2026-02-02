use rowan::SyntaxNode;

use crate::{parse::Parser, syntax_kind::AirycLanguage};

fn try_it(source: &str) -> SyntaxNode<AirycLanguage> {
    let parser = Parser::new(source);
    let (tree, errors) = parser.parse();

    if !errors.is_empty() {
        eprintln!("Source: {}", source);
        eprintln!("Parser errors: {:?}", errors);
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
fn test_struct_def_and_decl() {
    // 测试结构体定义
    let source1 = "struct Point { int x, int y }";
    insta::assert_debug_snapshot!("struct_def", try_it(source1));

    // 测试结构体变量声明
    let source2 = "struct Point p;";
    insta::assert_debug_snapshot!("struct_decl", try_it(source2));

    // 测试多个结构体变量声明
    let source3 = "struct Point q, r;";
    insta::assert_debug_snapshot!("struct_decl_multi", try_it(source3));
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

#[test]
fn test_struct_ast_nodes() {
    use crate::ast::*;

    // 测试 StructDef AST 节点
    let source = "struct Point { int x, int y }";
    let syntax = try_it(source);
    let root = CompUnit::cast(syntax).unwrap();

    let struct_def = root
        .global_decls()
        .find_map(|decl| {
            if let GlobalDecl::StructDef(s) = decl {
                Some(s)
            } else {
                None
            }
        })
        .expect("应该找到 StructDef");

    // 验证 struct 名称
    let name = struct_def.name().expect("应该有名称");
    let name_text = name.ident().expect("应该有标识符").text().to_string();
    assert_eq!(name_text, "Point");

    // 验证字段
    let fields: Vec<_> = struct_def.fields().collect();
    assert_eq!(fields.len(), 2);

    // 验证第一个字段
    let field1 = &fields[0];
    assert!(field1.ty().is_some());
    let field1_array_decl = field1.array_decl().expect("字段应该有 array_decl");
    let field1_name = field1_array_decl.name().expect("array_decl 应该有名称");
    let field1_text = field1_name
        .ident()
        .expect("应该有标识符")
        .text()
        .to_string();
    assert_eq!(field1_text, "x");

    // 验证第二个字段
    let field2 = &fields[1];
    let field2_array_decl = field2.array_decl().expect("字段应该有 array_decl");
    let field2_name = field2_array_decl.name().expect("array_decl 应该有名称");
    let field2_text = field2_name
        .ident()
        .expect("应该有标识符")
        .text()
        .to_string();
    assert_eq!(field2_text, "y");
}
