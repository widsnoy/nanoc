mod expression;
mod parsing;
mod recovery;
mod statement;

use crate::syntax_kind::SyntaxKind;
use crate::{lexer::Lexer, syntax_kind::AirycLanguage};
use rowan::{Checkpoint, GreenNode, GreenNodeBuilder};

#[derive(Debug)]
pub enum ParserError {
    Expected(SyntaxKind),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::Expected(kind) => write!(f, "expected token: {:?}", kind),
        }
    }
}

/// 语法解析器
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    builder: GreenNodeBuilder<'static>,
    pub errors: Vec<ParserError>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            lexer: Lexer::new(text),
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> (GreenNode, Vec<ParserError>) {
        self.parse_root();
        self.finish()
    }

    pub fn new_root(green_node: GreenNode) -> rowan::SyntaxNode<AirycLanguage> {
        rowan::SyntaxNode::new_root(green_node)
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        self.builder.checkpoint()
    }

    pub(crate) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind.into());
    }

    pub(crate) fn start_node_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(checkpoint, kind.into());
    }

    pub(crate) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// 消费当前 token 并添加到语法树
    pub(crate) fn bump(&mut self) {
        if self.lexer.current() == SyntaxKind::EOF {
            return;
        }
        self.bump_trivia();
        let kind = self.lexer.current();
        let text = self.lexer.current_text();

        self.builder.token(rowan::SyntaxKind(kind as u16), text);
        self.lexer.bump();
    }

    /// 消费 token 直到遇到非空白字符
    pub(crate) fn bump_trivia(&mut self) {
        while self.lexer.current().is_trivia() {
            self.builder.token(
                rowan::SyntaxKind(self.lexer.current() as u16),
                self.lexer.current_text(),
            );
            self.lexer.bump();
        }
    }

    /// 完成解析并返回 GreenNode
    pub(crate) fn finish(self) -> (GreenNode, Vec<ParserError>) {
        (self.builder.finish(), self.errors)
    }

    /// 检查当前 token 是否匹配 `kind`（跳过空白）
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.peek() == kind
    }

    /// 检查下一个 token 是否匹配 `kind`（跳过空白）
    pub(crate) fn at_1(&self, kind: SyntaxKind) -> bool {
        self.peek_1() == kind
    }

    /// 检查向前看两步的 token 是否匹配 `kind`（跳过空白）
    pub(crate) fn at_2(&self, kind: SyntaxKind) -> bool {
        self.peek_2() == kind
    }

    /// 获取当前 token 类型（跳过空白）
    pub(crate) fn peek(&self) -> SyntaxKind {
        self.lexer.current_without_trivia()
    }

    /// 获取下一个 token 类型（跳过空白）
    pub(crate) fn peek_1(&self) -> SyntaxKind {
        self.lexer.current_without_trivia_1()
    }

    /// 获取向前看两步的 token 类型（跳过空白）
    pub(crate) fn peek_2(&self) -> SyntaxKind {
        self.lexer.current_without_trivia_2()
    }
}
