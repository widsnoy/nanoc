use crate::{
    parser::{Parser, ParserError},
    syntax_kind::SyntaxKind,
};

impl SyntaxKind {
    /// 检查是否为表达式恢复点
    pub(crate) fn is_expr_recovery(self) -> bool {
        matches!(
            self,
            SyntaxKind::SEMI
                | SyntaxKind::R_BRACE
                | SyntaxKind::R_PAREN
                | SyntaxKind::R_BRACK
                | SyntaxKind::COMMA
                | SyntaxKind::EOF
        )
    }

    /// 检查是否为语句恢复点
    pub(crate) fn is_stmt_recovery(self) -> bool {
        matches!(
            self,
            SyntaxKind::SEMI
                | SyntaxKind::R_BRACE
                | SyntaxKind::IF_KW
                | SyntaxKind::WHILE_KW
                | SyntaxKind::RETURN_KW
                | SyntaxKind::INT_KW
                | SyntaxKind::FLOAT_KW
                | SyntaxKind::CONST_KW
                | SyntaxKind::EOF
        )
    }

    /// 检查是否为声明恢复点
    pub(crate) fn is_decl_recovery(self) -> bool {
        matches!(
            self,
            SyntaxKind::IF_KW
                | SyntaxKind::WHILE_KW
                | SyntaxKind::RETURN_KW
                | SyntaxKind::INT_KW
                | SyntaxKind::FLOAT_KW
                | SyntaxKind::CONST_KW
                | SyntaxKind::STRUCT_KW
                | SyntaxKind::VOID_KW
                | SyntaxKind::EOF
                | SyntaxKind::SEMI
                | SyntaxKind::R_BRACE
        )
    }
}

impl<'a> Parser<'a> {
    pub(crate) fn expect_or_else_recovery<F>(&mut self, expect_token: SyntaxKind, cond: F)
    where
        F: Fn(SyntaxKind) -> bool,
    {
        if self.at(expect_token) {
            self.bump();
        } else {
            self.errors.push(ParserError::Expected(expect_token));
            self.start_node(SyntaxKind::ERROR);
            while !cond(self.peek()) {
                self.bump();
            }
            self.finish_node();
        }
    }
}
