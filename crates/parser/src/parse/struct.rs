use crate::{parse::Parser, syntax_kind::SyntaxKind};

impl Parser<'_> {
    pub(super) fn parse_struct_def(&mut self) {
        self.start_node(SyntaxKind::STRUCT_DEF);
        self.expect_or_else_recovery(SyntaxKind::STRUCT_KW, SyntaxKind::is_decl_recovery);
        self.parse_name();
        self.expect_or_else_recovery(SyntaxKind::L_BRACE, SyntaxKind::is_decl_recovery);

        if !self.at(SyntaxKind::R_BRACE) {
            self.parse_struct_field();
        }
        while !self.at(SyntaxKind::R_BRACE) {
            self.expect_or_else_recovery(SyntaxKind::COMMA, SyntaxKind::is_decl_recovery);
            self.parse_struct_field();
        }

        self.expect_or_else_recovery(SyntaxKind::R_BRACE, SyntaxKind::is_decl_recovery);
        self.finish_node();
    }

    fn parse_struct_field(&mut self) {
        self.start_node(SyntaxKind::STRUCT_FIELD);
        self.parse_name();
        self.expect_or_else_recovery(SyntaxKind::COLON, SyntaxKind::is_decl_recovery);
        self.parse_type();
        self.finish_node();
    }
}
