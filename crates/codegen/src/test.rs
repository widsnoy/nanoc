use inkwell::context::Context;
use syntax::{
    ast::{AstNode, CompUnit},
    SyntaxNode,
};

use crate::llvm_ir;

fn try_it(code: &str) -> String {
    let parser = parser::parse::Parser::new(code);
    let (green_node, errors, _) = parser.parse();
    assert!(errors.is_empty(), "Parser errors: {:?}", errors);

    let root = SyntaxNode::new_root(green_node.clone());
    let mut analyzer = analyzer::module::Module::new(green_node);
    analyzer.analyze();

    assert!(
        analyzer.semantic_errors.is_empty(),
        "Analyzer erros: {:?}",
        analyzer.semantic_errors
    );

    // dbg!(&root);
    let comp_unit = CompUnit::cast(root).unwrap();

    let context = Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let mut program = llvm_ir::Program {
        context: &context,
        builder: &builder,
        module: &module,
        analyzer: &analyzer,
        project: None,
        symbols: Default::default(),
    };

    program.compile_comp_unit(comp_unit).unwrap();

    program.module.print_to_string().to_string()
}

#[test]
fn test_const_init() {
    let code = r#"
    let x: const i32 = 233;
    let y: const i32 = x + 1;
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_func_call() {
    let code = r#"
    fn func(p: i32, y: i32) -> i32 {
        let x: i32 = 233;
        return x;
    }
    fn main() -> i32 {
        let res: i32 = func(1, 2);
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_expr_stmt() {
    let code = r#"
    fn main() -> i32 {
        let x: i32 = 233;
        let y: i32 = 1 + 2 * 3;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_if_stmt() {
    let code = r#"
    fn main() -> i32 {
        let x: i32;
        if (x > 1) {
            if ( x > 2) {
                x = 3;
            } else if (x > 3) {
                x = 4;
            } else {
                x = 5;
            }
        } else {
            x = 6;
        }
        return 0;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_complex_program() {
    let code = r#"
    fn solve(n: i32, a: i32, b: i32, c: i32) {
        if (n == 1) {
            // print("{a}->{c}\n");
            return;
        }
        solve(n - 1, a, c, b);
        // print("{a}->{c}\n");
        solve(n - 1, b, a, c);
    }

    fn main() -> i32 {
        let n: i32 = 3;
        solve(n, 1, 2, 3);
        return 0;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_return_stmt() {
    let code = r#"
    fn main() -> i32 {
        let x: i32 = 233;
        let y: i32 = 1 + 2 * 3;
        return x + y;
        return 0;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_array_initialize() {
    let code = r#"
    let a: [i32; 3] = {};
    let b: [[[i32; 4]; 3]; 2] = {1, 2, 3, 4, {5}, {6}, {7, 8}};
    
    let d: [f32; 2] = {1.11};

    let g: [i32; 2] = {1, 2};

    fn main() -> i32 {
        let c: i32 = b[1][0][1];
        let e: f32 = d[0];
        let a: [i32; 3] = {1, 2, 3};
        let b: [i32; 2] = {1, a[1]};
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}
