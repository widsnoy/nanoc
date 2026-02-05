use analyzer::module::Module;
use parser::parse::Parser;
use tools::LineIndex;

/// 文档状态管理
#[derive(Debug)]
pub struct Document {
    #[allow(unused)] // FIXME
    pub text: String,
    pub line_index: LineIndex,
    pub module: Module,
}

impl Document {
    /// 创建新文档
    pub fn new(text: String) -> Self {
        let parser = Parser::new(&text);
        let line_index = LineIndex::new(
            parser
                .lexer
                .get_tokens()
                .iter()
                .filter(|(kind, _, _)| *kind == syntax::SyntaxKind::NEWLINE)
                .map(|(_, _, r)| r.end as u32)
                .collect::<Vec<_>>(),
        );

        let (green_node, _parser_errors) = parser.parse(); //FIXME:
        let mut module = Module::new(green_node);
        module.analyze();
        Self {
            text,
            line_index,
            module,
        }
    }

    /// 更新文档内容
    pub fn update(&mut self, text: String) {
        *self = Self::new(text);
    }
}
