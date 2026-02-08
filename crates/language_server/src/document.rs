use analyzer::module::Module;
use parser::parse::Parser;
use tools::LineIndex;

use crate::error::LspError;

/// 文档状态管理
#[derive(Debug)]
pub struct Document {
    #[allow(unused)] // FIXME
    pub text: String,
    pub line_index: LineIndex,
    pub module: Module<'static>,
    pub errors: Vec<LspError>,
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
                .map(|(_, _, r)| r.end().into())
                .collect::<Vec<u32>>(),
        );

        let (green_node, errors) = parser.parse();
        let mut module = Module::new(green_node);
        module.analyze();

        // 收集所有错误
        let mut all_errors = Vec::with_capacity(errors.len() + module.semantic_errors.len());
        for e in errors {
            all_errors.push(e.into());
        }
        // 将 semantic_errors 移动到 errors 中
        for e in module.semantic_errors.drain(..) {
            all_errors.push(e.into());
        }

        Self {
            text,
            line_index,
            module,
            errors: all_errors,
        }
    }

    /// 更新文档内容
    pub fn update(&mut self, text: String) {
        *self = Self::new(text);
    }
}
