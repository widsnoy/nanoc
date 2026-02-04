use analyzer::module::Module;
use parser::parse::Parser;

use crate::utils::line_index::LineIndex;

/// 文档状态管理
#[derive(Debug)]
pub struct Document {
    text: String,
    line_index: LineIndex,
    module: Module,
}

impl Document {
    /// 创建新文档
    pub fn new(text: String) -> Self {
        let parser = Parser::new(&text);
        let line_index = LineIndex::new(parser.get_newline_end_postions());

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
        self.text = text;
    }
}
