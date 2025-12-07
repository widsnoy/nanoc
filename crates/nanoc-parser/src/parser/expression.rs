use crate::{parser::Parser, syntax_kind::SyntaxKind};

impl Parser<'_> {
    pub(super) fn parse_exp(&mut self) {
        self.start_node(SyntaxKind::EXPR);
        self.parse_l_or_exp();
        self.finish_node();
    }

    pub(super) fn parse_lval_or_exp(&mut self) {
        self.parse_l_or_exp();
    }

    pub(super) fn parse_const_exp(&mut self) {
        self.start_node(SyntaxKind::CONST_EXPR);
        self.parse_l_or_exp();
        self.finish_node();
    }

    fn parse_l_or_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_l_and_exp();
        while self.at(SyntaxKind::PIPEPIPE) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.bump(); // op
            self.parse_l_and_exp();
            self.finish_node();
        }
    }

    fn parse_l_and_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_eq_exp();
        while self.at(SyntaxKind::AMPAMP) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.bump(); // op
            self.parse_eq_exp();
            self.finish_node();
        }
    }

    fn parse_eq_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_rel_exp();
        while matches!(self.peek(), SyntaxKind::EQEQ | SyntaxKind::NEQ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.bump(); // op
            self.parse_rel_exp();
            self.finish_node();
        }
    }

    fn parse_rel_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_add_exp();
        while matches!(
            self.peek(),
            SyntaxKind::LT | SyntaxKind::GT | SyntaxKind::LTEQ | SyntaxKind::GTEQ
        ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.bump(); // op
            self.parse_add_exp();
            self.finish_node();
        }
    }

    fn parse_add_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_mul_exp();
        while matches!(self.peek(), SyntaxKind::PLUS | SyntaxKind::MINUS) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.bump(); // op
            self.parse_mul_exp();
            self.finish_node();
        }
    }

    fn parse_mul_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_unary_exp();
        while matches!(
            self.peek(),
            SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT
        ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.bump(); // op
            self.parse_unary_exp();
            self.finish_node();
        }
    }

    fn parse_unary_exp(&mut self) {
        if self.peek().is_unary_op() {
            self.start_node(SyntaxKind::UNARY_EXPR);
            self.bump(); // op
            self.parse_unary_exp();
            self.finish_node();
        } else {
            self.parse_primary_exp();
        }
    }

    fn parse_primary_exp(&mut self) {
        if self.at(SyntaxKind::L_PAREN) {
            self.start_node(SyntaxKind::PAREN_EXPR);
            self.expect(SyntaxKind::L_PAREN);
            self.parse_exp();
            self.expect(SyntaxKind::R_PAREN);
            self.finish_node();
        } else if self.peek().is_number() {
            self.start_node(SyntaxKind::LITERAL);
            self.bump(); // number
            self.finish_node();
        } else {
            self.parse_lval_or_call_expr();
        }
    }

    pub fn parse_lval_or_call_expr(&mut self) {
        if self.at(SyntaxKind::STAR) {
            self.start_node(SyntaxKind::LVAL);
            self.bump(); // '*'
            self.parse_unary_exp();
            self.finish_node();
            return;
        }
        let cp = self.checkpoint();
        self.parse_name();
        if self.at(SyntaxKind::L_PAREN) {
            self.start_node_at(cp, SyntaxKind::CALL_EXPR);
            self.expect(SyntaxKind::L_PAREN);
            if !self.at(SyntaxKind::R_PAREN) {
                self.parse_func_r_params();
            }
            self.expect(SyntaxKind::R_PAREN);
            self.finish_node();
        } else {
            self.start_node_at(cp, SyntaxKind::LVAL);
            while self.at(SyntaxKind::L_BRACK) {
                self.bump(); // `[`
                self.parse_exp();
                self.expect(SyntaxKind::R_BRACK);
            }
            self.finish_node();
        }
    }
}
