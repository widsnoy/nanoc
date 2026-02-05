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
        if !self.expect_or_else_recovery(SyntaxKind::COLON, SyntaxKind::is_decl_recovery) {
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
        let success = self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_decl_recovery);
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
                if !is_first
                    && !self
                        .expect_or_else_recovery(SyntaxKind::COMMA, SyntaxKind::is_decl_recovery)
                {
                    continue;
                }

                if !self.parse_init_val() {
                    return false;
                }
                is_first = false;
            }
            let success =
                self.expect_or_else_recovery(SyntaxKind::R_BRACE, SyntaxKind::is_decl_recovery);
            self.finish_node();
            success
        } else {
            let success = self.parse_exp();
            self.finish_node();
            success
        }
    }
}
