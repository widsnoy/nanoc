use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    pub(super) fn parse_block(&mut self) {
        self.start_node(SyntaxKind::BLOCK);
        self.expect_or_else_recovery(SyntaxKind::L_BRACE, SyntaxKind::is_decl_recovery);
        while !matches!(self.peek(), SyntaxKind::R_BRACE | SyntaxKind::EOF) {
            self.parse_block_item();
        }
        self.expect_or_else_recovery(SyntaxKind::R_BRACE, SyntaxKind::is_decl_recovery);
        self.finish_node();
    }

    fn parse_block_item(&mut self) {
        match self.peek() {
            SyntaxKind::LET_KW => self.parse_var_def(),
            _ => self.parse_statement(),
        }
    }
}
