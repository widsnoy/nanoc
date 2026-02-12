use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    /// 解析表达式
    pub(super) fn parse_exp(&mut self) -> bool {
        self.parse_l_or_exp()
    }

    fn parse_l_or_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_l_and_exp() {
            return false;
        }
        while self.at(SyntaxKind::PIPEPIPE) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            if !self.parse_l_and_exp() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_l_and_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_eq_exp() {
            return false;
        }
        while self.at(SyntaxKind::AMPAMP) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            if !self.parse_eq_exp() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_eq_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_rel_exp() {
            return false;
        }
        while matches!(self.peek(), SyntaxKind::EQEQ | SyntaxKind::NEQ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            if !self.parse_rel_exp() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_rel_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_add_exp() {
            return false;
        }
        while matches!(
            self.peek(),
            SyntaxKind::LT | SyntaxKind::GT | SyntaxKind::LTEQ | SyntaxKind::GTEQ
        ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            if !self.parse_add_exp() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_add_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_mul_exp() {
            return false;
        }
        while matches!(self.peek(), SyntaxKind::PLUS | SyntaxKind::MINUS) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            if !self.parse_mul_exp() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_mul_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_unary_exp() {
            return false;
        }
        while matches!(
            self.peek(),
            SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT
        ) {
            self.start_node_at(cp, SyntaxKind::BINARY_EXPR);
            self.parse_binary_op();
            if !self.parse_unary_exp() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_unary_exp(&mut self) -> bool {
        if self.peek().is_unary_op() {
            self.start_node(SyntaxKind::UNARY_EXPR);
            self.parse_unary_op();
            let success = self.parse_unary_exp();
            self.finish_node();
            success
        } else {
            self.parse_postfix_exp()
        }
    }

    fn parse_postfix_exp(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_primary_exp() {
            return false;
        }
        while self.peek().is_postfix_op() {
            self.start_node_at(cp, SyntaxKind::POSTFIX_EXPR);
            self.parse_postfix_op();
            // 解析字段名和可能的数组索引，如 arr[0] 或 arr[0][1]
            if !self.parse_field_access() {
                self.finish_node();
                return false;
            }
            self.finish_node();
        }
        true
    }

    fn parse_postfix_op(&mut self) {
        self.start_node(SyntaxKind::POSTFIX_OP);
        self.bump(); // . or ->
        self.finish_node();
    }

    /// 解析 FieldAccess: Name {'[' Expr ']'}（用于 PostfixExpr 字段访问）
    fn parse_field_access(&mut self) -> bool {
        self.start_node(SyntaxKind::FIELD_ACCESS);
        if !self.parse_name() {
            self.finish_node();
            return false;
        }
        while self.at(SyntaxKind::L_BRACK) {
            self.bump();
            if !self.parse_exp() {
                self.finish_node();
                return false;
            }
            if !self.expect(SyntaxKind::R_BRACK) {
                self.finish_node();
                return false;
            }
        }
        self.finish_node();
        true
    }

    fn parse_primary_exp(&mut self) -> bool {
        if self.at(SyntaxKind::L_PAREN) {
            self.start_node(SyntaxKind::PAREN_EXPR);
            if !self.expect(SyntaxKind::L_PAREN) {
                self.finish_node();
                return false;
            }
            if !self.parse_exp() {
                self.finish_node();
                return false;
            }
            let success = self.expect(SyntaxKind::R_PAREN);
            self.finish_node();
            success
        } else if self.peek().is_number()
            || matches!(
                self.peek(),
                SyntaxKind::NULL_KW
                    | SyntaxKind::TRUE_KW
                    | SyntaxKind::FALSE_KW
                    | SyntaxKind::STRING_LITERAL
            )
        {
            self.start_node(SyntaxKind::LITERAL);
            self.bump();
            self.finish_node();
            true
        } else {
            self.parse_lval_or_call_expr()
        }
    }

    /// 解析左值或函数调用表达式
    pub(super) fn parse_lval_or_call_expr(&mut self) -> bool {
        let cp = self.checkpoint();
        if !self.parse_name() {
            return false;
        }
        if self.at(SyntaxKind::L_PAREN) {
            self.start_node_at(cp, SyntaxKind::CALL_EXPR);
            if !self.expect(SyntaxKind::L_PAREN) {
                self.finish_node();
                return false;
            }
            if !self.at(SyntaxKind::R_PAREN) && !self.parse_func_r_params() {
                self.finish_node();
                return false;
            }
            let success = self.expect(SyntaxKind::R_PAREN);
            self.finish_node();
            success
        } else {
            self.start_node_at(cp, SyntaxKind::INDEX_VAL);
            while self.at(SyntaxKind::L_BRACK) {
                self.bump(); // `[`
                if !self.parse_exp() {
                    self.finish_node();
                    return false;
                }
                if !self.expect(SyntaxKind::R_BRACK) {
                    self.finish_node();
                    return false;
                }
            }
            self.finish_node();
            true
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
