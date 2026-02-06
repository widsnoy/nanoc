use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    /// 解析语句
    pub(super) fn parse_statement(&mut self) -> bool {
        match self.peek() {
            SyntaxKind::IF_KW => self.parse_if_statement(),
            SyntaxKind::WHILE_KW => self.parse_while_statement(),
            SyntaxKind::BREAK_KW => self.parse_break_statement(),
            SyntaxKind::CONTINUE_KW => self.parse_continue_statement(),
            SyntaxKind::RETURN_KW => self.parse_return_statement(),
            SyntaxKind::L_BRACE => self.parse_block(),
            SyntaxKind::SEMI => {
                self.bump(); // consume ';'
                true
            }
            _ => {
                // 让语义分析检查赋值语句左值是否为 Lval
                let cp = self.checkpoint();
                if !self.parse_exp() {
                    return false;
                }
                if self.at(SyntaxKind::EQ) {
                    self.start_node_at(cp, SyntaxKind::ASSIGN_STMT);
                    self.bump(); // =
                    if !self.parse_exp() {
                        self.finish_node();
                        return false;
                    }
                    let success = self.expect(SyntaxKind::SEMI);
                    self.finish_node();
                    success
                } else {
                    self.start_node_at(cp, SyntaxKind::EXPR_STMT);
                    let success = self.expect(SyntaxKind::SEMI);
                    self.finish_node();
                    success
                }
            }
        }
    }

    fn parse_if_statement(&mut self) -> bool {
        self.start_node(SyntaxKind::IF_STMT);

        if !self.expect(SyntaxKind::IF_KW) {
            self.finish_node();
            return false;
        }
        if !self.expect(SyntaxKind::L_PAREN) {
            self.finish_node();
            return false;
        }
        if !self.parse_exp() {
            self.finish_node();
            return false;
        }
        if !self.expect(SyntaxKind::R_PAREN) {
            self.finish_node();
            return false;
        }
        if !self.parse_statement() {
            self.finish_node();
            return false;
        }
        if self.at(SyntaxKind::ELSE_KW) {
            self.bump();
            if !self.parse_statement() {
                self.finish_node();
                return false;
            }
        }
        self.finish_node();
        true
    }

    fn parse_while_statement(&mut self) -> bool {
        self.start_node(SyntaxKind::WHILE_STMT);

        if !self.expect(SyntaxKind::WHILE_KW) {
            self.finish_node();
            return false;
        }
        if !self.expect(SyntaxKind::L_PAREN) {
            self.finish_node();
            return false;
        }
        if !self.parse_exp() {
            self.finish_node();
            return false;
        }
        if !self.expect(SyntaxKind::R_PAREN) {
            self.finish_node();
            return false;
        }
        let success = self.parse_statement();
        self.finish_node();
        success
    }

    fn parse_break_statement(&mut self) -> bool {
        self.start_node(SyntaxKind::BREAK_STMT);

        if !self.expect(SyntaxKind::BREAK_KW) {
            self.finish_node();
            return false;
        }
        let success = self.expect(SyntaxKind::SEMI);
        self.finish_node();
        success
    }

    fn parse_continue_statement(&mut self) -> bool {
        self.start_node(SyntaxKind::CONTINUE_STMT);

        if !self.expect(SyntaxKind::CONTINUE_KW) {
            self.finish_node();
            return false;
        }
        let success = self.expect(SyntaxKind::SEMI);
        self.finish_node();
        success
    }

    fn parse_return_statement(&mut self) -> bool {
        self.start_node(SyntaxKind::RETURN_STMT);

        if !self.expect(SyntaxKind::RETURN_KW) {
            self.finish_node();
            return false;
        }
        if !self.at(SyntaxKind::SEMI) && !self.parse_exp() {
            self.finish_node();
            return false;
        }
        let success = self.expect(SyntaxKind::SEMI);
        self.finish_node();
        success
    }
}
