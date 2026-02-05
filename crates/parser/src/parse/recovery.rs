use crate::parse::{Parser, ParserError};
use syntax::SyntaxKind;

impl<'a> Parser<'a> {
    /// 期望特定 token，如果不匹配则尝试恢复
    /// 返回 true 表示成功找到期望的 token
    /// 返回 false 表示遇到恢复词或恢复后仍未找到，需要调用者退出
    pub(crate) fn expect_or_else_recovery<F>(&mut self, expect_token: SyntaxKind, cond: F) -> bool
    where
        F: Fn(SyntaxKind) -> bool,
    {
        if self.at(expect_token) {
            self.bump();
            return true;
        }

        // 记录错误
        let range = self.current_range();
        self.parse_errors.push(ParserError::Expected {
            expected: vec![expect_token],
            range,
        });

        // 如果当前 token 是恢复词，不 bump，返回 false
        if cond(self.peek()) {
            return false;
        }

        // 尝试恢复：跳过 token 直到遇到恢复词或 EOF
        self.start_node(SyntaxKind::ERROR);
        while !cond(self.peek()) && !self.at(SyntaxKind::EOF) {
            self.bump();
        }
        self.finish_node();

        // 恢复后返回 false，让调用者检查是否继续
        false
    }

    /// 找到新的可以开始的关键词
    /// 在确定错误的时候使用
    /// 要保证有 SyntaxKind::EOF
    pub(crate) fn skip_until(&mut self, next_start_token: &[SyntaxKind]) {
        assert!(next_start_token.contains(&SyntaxKind::EOF));
        let range = self.current_range();
        self.parse_errors.push(ParserError::Expected {
            expected: next_start_token.to_vec(),
            range,
        });
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
