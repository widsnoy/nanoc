use rowan::SyntaxNode;

use crate::parse::Parser;
use syntax::AirycLanguage;

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
    let A: const i32 = 1;
    let B: const f32 = 2.0;
    let C: const f32 = 3.0;
    let a: i32;
    let b: f32 = 1.0;
    let s: struct MyStruct;
    let p: *mut i32;
    let arr: [i32; 10];
    let arr2: [[i32; 3]; 2];
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_struct_def_and_decl() {
    // 测试结构体定义
    let source1 = "struct Point { x: i32, y: i32 }";
    insta::assert_debug_snapshot!("struct_def", try_it(source1));

    // 测试结构体变量声明
    let source2 = "let p: struct Point;";
    insta::assert_debug_snapshot!("struct_decl", try_it(source2));

    // 测试多个结构体变量声明
    let source3 = "let q: struct Point; let r: struct Point;";
    insta::assert_debug_snapshot!("struct_decl_multi", try_it(source3));
}

#[test]
fn test_functions() {
    let source = r#"
    fn func1() {}
    fn func2(a: i32) -> i32 {}
    fn func3(a: i32, b: f32) -> i32 {}
    fn func4(p: *mut i32, arr: *mut [i32; 10]) -> *mut i32 {}
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_expressions() {
    let source = r#"
    let x: i32 = a + b * c;
    let y: i32 = (a + b) * c;
    let z: i32 = a || b && c;
    let w: i32 = a == b;
    let rel: i32 = a < b;
    let unary: i32 = -a + !b;
    let ptr: i32 = *p + &x;
    let arr: i32 = a[1][2];
    let call: i32 = foo(a, b);
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_complex_mix() {
    let source = r#"
    let MAX: const i32 = 100;
    let p: struct Point;

    fn main(argc: i32, argv: *mut [*mut i32; 10]) -> i32 {
        let a: i32 = 1;
        let ptr: *mut i32 = &a;
        let b: const i32 = MAX;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_if_statement() {
    let source = r#"
    fn test() {
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
    fn test() {
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
    fn test() {
        return;
        return 1;
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_block_statement() {
    let source = r#"
    fn test() {
        {
            let nested: i32;
        }
    }
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_assign_statement() {
    let source = r#"
    fn test() {
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
    fn test() {
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
    let a: i32 = 1;
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_array_init() {
    let source = r#"
    let a: [i32; 3] = {1, 2, 3};
    let b: [[i32; 3]; 2] = {{1, 2, 3}, {4, 5}};
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_postfix_expressions() {
    let source = r#"
    let x: i32 = s.field;
    let y: i32 = p->member;
    let z: i32 = s.a.b;
    let w: i32 = p->a->b;
    let mixed: i32 = arr[0].field;
    let complex: i32 = func().member;
    "#;
    insta::assert_debug_snapshot!(try_it(source));
}

#[test]
fn test_struct_ast_nodes() {
    use crate::ast::*;

    // 测试 StructDef AST 节点
    let source = "struct Point { x: i32, y: i32 }";
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
    let field1_name = field1.name().expect("字段应该有名称");
    let field1_text = field1_name
        .ident()
        .expect("应该有标识符")
        .text()
        .to_string();
    assert_eq!(field1_text, "x");

    // 验证第二个字段
    let field2 = &fields[1];
    let field2_name = field2.name().expect("字段应该有名称");
    let field2_text = field2_name
        .ident()
        .expect("应该有标识符")
        .text()
        .to_string();
    assert_eq!(field2_text, "y");
}
