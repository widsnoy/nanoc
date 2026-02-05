use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    pub(super) fn parse_block(&mut self) -> bool {
        self.start_node(SyntaxKind::BLOCK);

        if !self.expect_or_else_recovery(SyntaxKind::L_BRACE, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }

        while !matches!(self.peek(), SyntaxKind::R_BRACE | SyntaxKind::EOF) {
            if !self.parse_block_item() {
                self.finish_node();
                return false;
            }
        }

        let success =
            self.expect_or_else_recovery(SyntaxKind::R_BRACE, SyntaxKind::is_decl_recovery);
        self.finish_node();
        success
    }

    fn parse_block_item(&mut self) -> bool {
        match self.peek() {
            SyntaxKind::LET_KW => self.parse_var_def(),
            _ => self.parse_statement(),
        }
    }
}
