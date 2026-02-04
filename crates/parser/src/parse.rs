mod block;
mod common;
mod expression;
mod function;
mod recovery;
mod statement;
mod r#struct;
mod variable;

use lexer::Lexer;
use rowan::{Checkpoint, GreenNode, GreenNodeBuilder};
use syntax::SyntaxKind;

#[derive(Debug)]
pub enum ParserError {
    Expected(Vec<SyntaxKind>),
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
    pub lexer: Lexer<'a>,
    builder: GreenNodeBuilder<'static>,
    pub parse_errors: Vec<ParserError>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            lexer: Lexer::new(text),
            builder: GreenNodeBuilder::new(),
            parse_errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> (GreenNode, Vec<ParserError>) {
        self.parse_root();
        self.finish()
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
        (self.builder.finish(), self.parse_errors)
    }

    /// 检查当前 token 是否匹配 `kind`（跳过空白）
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.peek() == kind
    }

    /// 获取当前 token 类型（跳过空白）
    pub(crate) fn peek(&self) -> SyntaxKind {
        self.lexer.current_without_trivia()
    }

    pub fn parse_root(&mut self) {
        self.start_node(SyntaxKind::COMP_UNIT);
        self.bump_trivia();

        loop {
            match self.peek() {
                SyntaxKind::LET_KW => self.parse_var_def(),
                SyntaxKind::FN_KW => self.parse_func_def(),
                SyntaxKind::STRUCT_KW => self.parse_struct_def(),
                SyntaxKind::EOF => break,
                _ => {
                    self.skip_until(&[
                        SyntaxKind::LET_KW,
                        SyntaxKind::FN_KW,
                        SyntaxKind::STRUCT_KW,
                        SyntaxKind::EOF,
                    ]);
                }
            }
            self.bump_trivia();
        }

        self.finish_node();
    }
}
