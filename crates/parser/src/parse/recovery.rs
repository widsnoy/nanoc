use crate::parse::{Parser, ParserError};
use syntax::SyntaxKind;

impl<'a> Parser<'a> {
    pub(crate) fn expect_or_else_recovery<F>(&mut self, expect_token: SyntaxKind, cond: F)
    where
        F: Fn(SyntaxKind) -> bool,
    {
        if self.at(expect_token) {
            self.bump();
        } else {
            self.parse_errors
                .push(ParserError::Expected(vec![expect_token]));
            if cond(self.peek()) {
                return;
            }
            self.start_node(SyntaxKind::ERROR);
            while !cond(self.peek()) {
                self.bump();
            }
            self.finish_node();
        }
    }

    /// 找到新的可以开始的关键词
    /// 在确定错误的时候使用
    /// 要保证有 SyntaxKind::EOF
    pub(crate) fn skip_until(&mut self, next_start_token: &[SyntaxKind]) {
        assert!(next_start_token.contains(&SyntaxKind::EOF));
        self.parse_errors
            .push(ParserError::Expected(next_start_token.to_vec()));
        if next_start_token.iter().any(|x| self.at(*x)) {
            return;
        }
        self.start_node(SyntaxKind::ERROR);
        while !next_start_token.iter().any(|x| self.at(*x)) {
            self.bump();
        }
        self.finish_node();
    }
}
