use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    /// 解析函数定义
    pub(super) fn parse_func_def(&mut self) -> bool {
        self.start_node(SyntaxKind::FUNC_DEF);
        self.bump(); // FN_KW

        if !self.parse_name() {
            self.finish_node();
            return false;
        }
        if !self.expect_or_else_recovery(SyntaxKind::L_PAREN, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }
        if !self.at(SyntaxKind::R_PAREN) && !self.parse_func_f_params() {
            self.finish_node();
            return false;
        }
        if !self.expect_or_else_recovery(SyntaxKind::R_PAREN, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }
        if self.at(SyntaxKind::ARROW) {
            self.bump();
            if !self.parse_type() {
                self.finish_node();
                return false;
            }
        }
        let success = self.parse_block();
        self.finish_node();
        success
    }

    /// 解析函数形参列表
    fn parse_func_f_params(&mut self) -> bool {
        self.start_node(SyntaxKind::FUNC_F_PARAMS);

        let mut is_first = true;

        while !matches!(self.peek(), SyntaxKind::R_PAREN | SyntaxKind::EOF) {
            if !is_first
                && !self.expect_or_else_recovery(SyntaxKind::COMMA, SyntaxKind::is_decl_recovery)
            {
                continue;
            }
            if !self.parse_func_f_param() {
                self.finish_node();
                return false;
            }
            is_first = false
        }

        self.finish_node();
        true
    }

    /// 解析单个函数形参
    fn parse_func_f_param(&mut self) -> bool {
        self.start_node(SyntaxKind::FUNC_F_PARAM);
        if !self.parse_name() {
            self.finish_node();
            return false;
        }
        if !self.expect_or_else_recovery(SyntaxKind::COLON, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }
        let success = self.parse_type();
        self.finish_node();
        success
    }

    /// 解析函数实参列表
    pub(super) fn parse_func_r_params(&mut self) -> bool {
        self.start_node(SyntaxKind::FUNC_R_PARAMS);

        let mut is_first = true;

        while !matches!(self.peek(), SyntaxKind::R_PAREN | SyntaxKind::EOF) {
            if !is_first
                && !self.expect_or_else_recovery(SyntaxKind::COMMA, SyntaxKind::is_decl_recovery)
            {
                continue;
            }
            if !self.parse_exp() {
                self.finish_node();
                return false;
            }
            is_first = false;
        }

        self.finish_node();
        true
    }
}
