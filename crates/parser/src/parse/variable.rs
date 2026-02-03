use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    pub(super) fn parse_var_def(&mut self) {
        self.start_node(SyntaxKind::VAR_DEF);
        self.bump();
        self.parse_name();
        self.expect_or_else_recovery(SyntaxKind::COLON, SyntaxKind::is_decl_recovery);
        self.parse_type();
        if self.at(SyntaxKind::EQ) {
            self.bump();
            self.parse_init_val();
        }
        self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_decl_recovery);
        self.finish_node();
    }

    pub(super) fn parse_init_val(&mut self) {
        self.start_node(SyntaxKind::INIT_VAL);
        if self.at(SyntaxKind::L_BRACE) {
            self.bump();
            while !matches!(self.peek(), SyntaxKind::R_BRACE | SyntaxKind::EOF) {
                self.parse_init_val();
                if self.at(SyntaxKind::COMMA) {
                    self.bump();
                }
            }
            self.expect_or_else_recovery(SyntaxKind::R_BRACE, SyntaxKind::is_decl_recovery);
        } else {
            self.parse_exp();
        }
        self.finish_node();
    }
}
