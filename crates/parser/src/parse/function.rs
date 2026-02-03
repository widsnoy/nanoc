use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    pub(super) fn parse_func_def(&mut self) {
        self.start_node(SyntaxKind::FUNC_DEF);
        self.bump();
        self.parse_name();
        self.expect_or_else_recovery(SyntaxKind::L_PAREN, SyntaxKind::is_decl_recovery);
        if !self.at(SyntaxKind::R_PAREN) {
            self.parse_func_f_params();
        }
        self.expect_or_else_recovery(SyntaxKind::R_PAREN, SyntaxKind::is_decl_recovery);
        if self.at(SyntaxKind::ARROW) {
            self.bump();
            self.parse_type();
        }
        self.parse_block();
        self.finish_node();
    }

    fn parse_func_f_params(&mut self) {
        self.start_node(SyntaxKind::FUNC_F_PARAMS);
        self.parse_func_f_param();
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.parse_func_f_param();
        }
        self.finish_node();
    }

    fn parse_func_f_param(&mut self) {
        self.start_node(SyntaxKind::FUNC_F_PARAM);
        self.parse_name();
        self.expect_or_else_recovery(SyntaxKind::COLON, SyntaxKind::is_decl_recovery);
        self.parse_type();
        self.finish_node();
    }

    pub(super) fn parse_func_r_params(&mut self) {
        self.start_node(SyntaxKind::FUNC_R_PARAMS);
        self.parse_exp();
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.parse_exp();
        }
        self.finish_node();
    }
}
