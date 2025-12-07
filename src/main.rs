use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let parser = nanoc_parser::parser::Parser::new(&input);
    let (green_node, errors) = parser.parse();
    if !errors.is_empty() {
        eprintln!("Parser errors:");
        for error in errors {
            eprintln!("- {}", error);
        }
        std::process::exit(1);
    }
    dbg!(nanoc_parser::parser::Parser::new_root(green_node));
}
