use crate::{parser::Parser, syntax_kind::SyntaxKind};

impl Parser<'_> {
    pub(super) fn parse_exp(&mut self) {
        self.parse_l_or_exp();
    }

    fn parse_l_or_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_l_and_exp();
        while self.at(SyntaxKind::PIPEPIPE) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            self.parse_l_and_exp();
            self.finish_node();
        }
    }

    fn parse_l_and_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_eq_exp();
        while self.at(SyntaxKind::AMPAMP) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            self.parse_eq_exp();
            self.finish_node();
        }
    }

    fn parse_eq_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_rel_exp();
        while matches!(self.peek(), SyntaxKind::EQEQ | SyntaxKind::NEQ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
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
            self.parse_binary_op();
            self.parse_add_exp();
            self.finish_node();
        }
    }

    fn parse_add_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_mul_exp();
        while matches!(self.peek(), SyntaxKind::PLUS | SyntaxKind::MINUS) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
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
            self.parse_binary_op();
            self.parse_unary_exp();
            self.finish_node();
        }
    }

    fn parse_unary_exp(&mut self) {
        if self.peek().is_unary_op() {
            self.start_node(SyntaxKind::UNARY_EXPR);
            self.parse_unary_op();
            self.parse_unary_exp();
            self.finish_node();
        } else {
            self.parse_postfix_exp();
        }
    }

    fn parse_postfix_exp(&mut self) {
        let cp = self.checkpoint();
        self.parse_primary_exp();
        while self.peek().is_postfix_op() {
            self.start_node_at(cp, SyntaxKind::POSTFIX_EXPR);
            self.parse_postfix_op();
            // 解析字段名和可能的数组索引，如 arr[0] 或 arr[0][1]
            self.parse_field_access();
            self.finish_node();
        }
    }

    fn parse_postfix_op(&mut self) {
        self.start_node(SyntaxKind::POSTFIX_OP);
        self.bump(); // . or ->
        self.finish_node();
    }

    /// 解析 FieldAccess: Name {'[' Expr ']'}（用于 PostfixExpr 字段访问）
    fn parse_field_access(&mut self) {
        self.start_node(SyntaxKind::FIELD_ACCESS);
        self.parse_name();
        while self.at(SyntaxKind::L_BRACK) {
            self.bump();
            self.parse_exp();
            self.expect_or_else_recovery(SyntaxKind::R_BRACK, SyntaxKind::is_expr_recovery);
        }
        self.finish_node();
    }

    fn parse_primary_exp(&mut self) {
        if self.at(SyntaxKind::L_PAREN) {
            self.start_node(SyntaxKind::PAREN_EXPR);
            self.expect_or_else_recovery(SyntaxKind::L_PAREN, SyntaxKind::is_expr_recovery);
            self.parse_exp();
            self.expect_or_else_recovery(SyntaxKind::R_PAREN, SyntaxKind::is_expr_recovery);
            self.finish_node();
        } else if self.peek().is_number() {
            self.start_node(SyntaxKind::LITERAL);
            self.bump(); // number
            self.finish_node();
        } else {
            self.parse_lval_or_call_expr();
        }
    }

    pub(super) fn parse_lval_or_call_expr(&mut self) {
        let cp = self.checkpoint();
        self.parse_name();
        if self.at(SyntaxKind::L_PAREN) {
            self.start_node_at(cp, SyntaxKind::CALL_EXPR);
            self.expect_or_else_recovery(SyntaxKind::L_PAREN, SyntaxKind::is_expr_recovery);
            if !self.at(SyntaxKind::R_PAREN) {
                self.parse_func_r_params();
            }
            self.expect_or_else_recovery(SyntaxKind::R_PAREN, SyntaxKind::is_expr_recovery);
            self.finish_node();
        } else {
            self.start_node_at(cp, SyntaxKind::INDEX_VAL);
            while self.at(SyntaxKind::L_BRACK) {
                self.bump(); // `[`
                self.parse_exp();
                self.expect_or_else_recovery(SyntaxKind::R_BRACK, SyntaxKind::is_expr_recovery);
            }
            self.finish_node();
        }
    }

    /// 仅在确认是二元运算符后调用
    fn parse_binary_op(&mut self) {
        self.start_node(SyntaxKind::BINARY_OP);
        self.bump(); // op
        self.finish_node();
    }

    /// 仅在确认是一元运算符后调用
    fn parse_unary_op(&mut self) {
        self.start_node(SyntaxKind::UNARY_OP);
        self.bump(); // op
        self.finish_node();
    }
}
