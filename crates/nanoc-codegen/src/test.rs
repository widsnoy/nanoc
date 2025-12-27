use inkwell::context::Context;
use nanoc_parser::ast::{AstNode, CompUnit};
use nanoc_parser::visitor::Visitor as _;

use crate::llvm_ir;

fn try_it(code: &str) -> String {
    let parser = nanoc_parser::parser::Parser::new(code);
    let (green_node, errors) = parser.parse();
    assert!(errors.is_empty(), "Parser errors: {:?}", errors);

    let root = nanoc_parser::parser::Parser::new_root(green_node);

    let mut analyzer = nanoc_ir::module::Module::default();
    analyzer.walk(&root);

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
        current_function: None,
        scopes: Vec::new(),
        functions: Default::default(),
        globals: Default::default(),
        loop_stack: Vec::new(),
    };

    program.compile_comp_unit(comp_unit);

    program.module.print_to_string().to_string()
}

#[test]
fn test_const_init() {
    let code = r#"
    const int x = 233;
    const int y = x + 1;
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_func_call() {
    let code = r#"
    int func(int p, int y) {
        int x = 233;
        return x;
    }
    int main() {
        int res = func(1, 2);
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_expr_stmt() {
    let code = r#"
    int main() {
        int x = 233;
        int y = 1 + 2 * 3;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_if_stmt() {
    let code = r#"
    int main() {
        int x;
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
    void solve(int n, int a, int b, int c) {
        if (n == 1) {
            // print("{a}->{c}\n");
            return;
        }
        solve(n - 1, a, c, b);
        // print("{a}->{c}\n");
        solve(n - 1, b, a, c);
    }

    int main() {
        int n = 3;
        solve(n, 1, 2, 3);
        return 0;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}

#[test]
fn test_return_stmt() {
    let code = r#"
    int main() {
        int x = 233;
        int y = 1 + 2 * 3;
        return x + y;
        return 0;
    }
    "#;
    insta::assert_snapshot!(try_it(code));
}
