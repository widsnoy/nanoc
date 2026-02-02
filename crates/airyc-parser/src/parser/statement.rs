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
                    self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_stmt_recovery);
                    self.finish_node();
                } else {
                    self.start_node_at(cp, SyntaxKind::EXPR_STMT);

                    self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_stmt_recovery);

                    self.finish_node();
                }
            }
        }
    }

    fn parse_if_statement(&mut self) {
        self.start_node(SyntaxKind::IF_STMT);
        self.expect_or_else_recovery(SyntaxKind::IF_KW, SyntaxKind::is_stmt_recovery);
        self.expect_or_else_recovery(SyntaxKind::L_PAREN, SyntaxKind::is_stmt_recovery);
        self.parse_exp();
        self.expect_or_else_recovery(SyntaxKind::R_PAREN, SyntaxKind::is_stmt_recovery);
        self.parse_statement();
        if self.at(SyntaxKind::ELSE_KW) {
            self.bump();
            self.parse_statement();
        }
        self.finish_node();
    }

    fn parse_while_statement(&mut self) {
        self.start_node(SyntaxKind::WHILE_STMT);
        self.expect_or_else_recovery(SyntaxKind::WHILE_KW, SyntaxKind::is_stmt_recovery);
        self.expect_or_else_recovery(SyntaxKind::L_PAREN, SyntaxKind::is_stmt_recovery);
        self.parse_exp();
        self.expect_or_else_recovery(SyntaxKind::R_PAREN, SyntaxKind::is_stmt_recovery);
        self.parse_statement();
        self.finish_node();
    }

    fn parse_break_statement(&mut self) {
        self.start_node(SyntaxKind::BREAK_STMT);
        self.expect_or_else_recovery(SyntaxKind::BREAK_KW, SyntaxKind::is_stmt_recovery);
        self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_stmt_recovery);
        self.finish_node();
    }

    fn parse_continue_statement(&mut self) {
        self.start_node(SyntaxKind::CONTINUE_STMT);
        self.expect_or_else_recovery(SyntaxKind::CONTINUE_KW, SyntaxKind::is_stmt_recovery);
        self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_stmt_recovery);
        self.finish_node();
    }

    fn parse_return_statement(&mut self) {
        self.start_node(SyntaxKind::RETURN_STMT);
        self.expect_or_else_recovery(SyntaxKind::RETURN_KW, SyntaxKind::is_stmt_recovery);
        if !self.at(SyntaxKind::SEMI) {
            self.parse_exp();
        }
        self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_stmt_recovery);
        self.finish_node();
    }
}
