use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn parse_statement(&mut self) {
        match self.peek() {
            SyntaxKind::IF_KW => self.parse_if_statement(),
            SyntaxKind::WHILE_KW => self.parse_while_statement(),
            SyntaxKind::BREAK_KW => self.parse_break_statement(),
            SyntaxKind::CONTINUE_KW => self.parse_continue_statement(),
            SyntaxKind::RETURN_KW => self.parse_return_statement(),
            SyntaxKind::L_BRACE => self.parse_block(),
            SyntaxKind::SEMI => {
                self.bump(); // consume ';'
            }
            _ => {
                // 让语义分析检查赋值语句左值是否为 Lval
                let cp = self.checkpoint();
                self.parse_exp();
                if self.at(SyntaxKind::EQ) {
                    self.start_node_at(cp, SyntaxKind::ASSIGN_STMT);
                    self.bump(); // =
                    self.parse_exp();
                    self.expect(SyntaxKind::SEMI);
                    self.finish_node();
                } else {
                    self.start_node_at(cp, SyntaxKind::EXPR_STMT);
                    self.expect(SyntaxKind::SEMI);
                    self.finish_node();
                }
            }
        }
    }

    fn parse_if_statement(&mut self) {
        self.start_node(SyntaxKind::IF_STMT);
        self.expect(SyntaxKind::IF_KW);
        self.expect(SyntaxKind::L_PAREN);
        self.parse_exp();
        self.expect(SyntaxKind::R_PAREN);
        self.parse_statement();
        if self.at(SyntaxKind::ELSE_KW) {
            self.bump();
            self.parse_statement();
        }
        self.finish_node();
    }

    fn parse_while_statement(&mut self) {
        self.start_node(SyntaxKind::WHILE_STMT);
        self.expect(SyntaxKind::WHILE_KW);
        self.expect(SyntaxKind::L_PAREN);
        self.parse_exp();
        self.expect(SyntaxKind::R_PAREN);
        self.parse_statement();
        self.finish_node();
    }

    fn parse_break_statement(&mut self) {
        self.start_node(SyntaxKind::BREAK_STMT);
        self.expect(SyntaxKind::BREAK_KW);
        self.expect(SyntaxKind::SEMI);
        self.finish_node();
    }

    fn parse_continue_statement(&mut self) {
        self.start_node(SyntaxKind::CONTINUE_STMT);
        self.expect(SyntaxKind::CONTINUE_KW);
        self.expect(SyntaxKind::SEMI);
        self.finish_node();
    }

    fn parse_return_statement(&mut self) {
        self.start_node(SyntaxKind::RETURN_STMT);
        self.expect(SyntaxKind::RETURN_KW);
        if !self.at(SyntaxKind::SEMI) {
            self.parse_exp();
        }
        self.expect(SyntaxKind::SEMI);
        self.finish_node();
    }
}
