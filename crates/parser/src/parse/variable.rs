use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    /// 解析变量定义
    pub(super) fn parse_var_def(&mut self) -> bool {
        self.start_node(SyntaxKind::VAR_DEF);
        self.bump(); // LET_KW

        if !self.parse_name() {
            self.finish_node();
            return false;
        }
        if !self.expect(SyntaxKind::COLON) {
            self.finish_node();
            return false;
        }
        if !self.parse_type() {
            self.finish_node();
            return false;
        }
        if self.at(SyntaxKind::EQ) {
            self.bump();
            if !self.parse_init_val() {
                self.finish_node();
                return false;
            }
        }
        let success = self.expect(SyntaxKind::SEMI);
        self.finish_node();
        success
    }

    /// 解析初始化值
    pub(super) fn parse_init_val(&mut self) -> bool {
        self.start_node(SyntaxKind::INIT_VAL);

        if self.at(SyntaxKind::L_BRACE) {
            self.bump(); // {

            let mut is_first = true;
            while !matches!(self.peek(), SyntaxKind::R_BRACE | SyntaxKind::EOF) {
                let start_pos = self.current_range().start();

                if !is_first && !self.expect(SyntaxKind::COMMA) {
                    // 检查是否有进展，防止死循环
                    if self.current_range().start() == start_pos {
                        break;
                    }
                    continue;
                }

                if !self.parse_init_val() {
                    return false;
                }
                is_first = false;
            }
            let success = self.expect(SyntaxKind::R_BRACE);
            self.finish_node();
            success
        } else {
            self.bump_trivia();
            let success = self.parse_exp();
            self.finish_node();
            success
        }
    }
}
