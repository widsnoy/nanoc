use inkwell::context::Context;
use nanoc_codegen::llvm_ir::Program;
use nanoc_parser::ast::{AstNode, CompUnit};

fn main() {
    // let mut input = String::new();
    // io::stdin().read_to_string(&mut input).unwrap();

    let input = r#"
    void print(int x) {
        if (x) {
            233;
        } else {
            666;
         }
    }

    int main() {
        print(233);
        return 0;
    }
    "#;

    let parser = nanoc_parser::parser::Parser::new(input);
    let (green_node, errors) = parser.parse();
    if !errors.is_empty() {
        eprintln!("Parser errors:");
        for error in errors {
            eprintln!("- {}", error);
        }
        std::process::exit(1);
    }

    let root = nanoc_parser::parser::Parser::new_root(green_node);
    let comp_unit = CompUnit::cast(root.clone()).expect("Root node is not CompUnit");

    let context = Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    let mut program = Program {
        context: &context,
        builder: &builder,
        module: &module,
        current_function: None,
        scopes: Vec::new(),
        functions: Default::default(),
        globals: Default::default(),
        loop_stack: Vec::new(),
    };

    program.compile_comp_unit(comp_unit);

    program.module.print_to_stderr();
}
