use crate::syntax_kind::SyntaxKind;
use logos::Logos;
use std::ops::Range;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum LexerErrorKind {
    InvalidInteger,
    InvalidFloat,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("Lexer error: {kind:?} at {span:?}: {text}")]
pub struct LexerError {
    pub kind: LexerErrorKind,
    pub text: String,
    pub span: Range<usize>,
}

/// 词法单元
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(error = LexerErrorKind)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_camel_case_types)]
pub enum Token {
    // 空白字符（空格和注释）
    #[regex(r"[ \t]+")]
    WHITESPACE,
    #[regex(r"(\r\n|\n|\r)")]
    NEWLINE,
    #[regex(r"//[^\n]*")]
    COMMENT_LINE,
    #[regex(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
    COMMENT_BLOCK,

    // 关键字
    #[token("const")]
    CONST_KW,
    #[token("int")]
    INT_KW,
    #[token("float")]
    FLOAT_KW,
    #[token("void")]
    VOID_KW,
    #[token("if")]
    IF_KW,
    #[token("else")]
    ELSE_KW,
    #[token("while")]
    WHILE_KW,
    #[token("break")]
    BREAK_KW,
    #[token("continue")]
    CONTINUE_KW,
    #[token("return")]
    RETURN_KW,
    #[token("struct")]
    STRUCT_KW,
    #[token("impl")]
    IMPL_KW,

    // 运算符和标点符号
    #[token("=")]
    EQ,
    #[token(";")]
    SEMI,
    #[token(",")]
    COMMA,
    #[token("{")]
    L_BRACE,
    #[token("}")]
    R_BRACE,
    #[token("(")]
    L_PAREN,
    #[token(")")]
    R_PAREN,
    #[token("[")]
    L_BRACK,
    #[token("]")]
    R_BRACK,
    #[token("*")]
    STAR,
    #[token(".")]
    DOT,
    #[token("->")]
    ARROW,

    // 算术运算符
    #[token("+")]
    PLUS,
    #[token("-")]
    MINUS,
    #[token("/")]
    SLASH,
    #[token("%")]
    PERCENT,

    // 比较运算符
    #[token("==")]
    EQEQ,
    #[token("!=")]
    NEQ,
    #[token("<")]
    LT,
    #[token(">")]
    GT,
    #[token("<=")]
    LTEQ,
    #[token(">=")]
    GTEQ,

    // 逻辑运算符
    #[token("&&")]
    AMPAMP,
    #[token("||")]
    PIPEPIPE,
    #[token("!")]
    BANG,

    /// 取地址运算符
    #[token("&")]
    AMP,

    // 字面量
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    IDENT,
    #[regex(r"0[xX][0-9a-fA-F]+", priority = 3)]
    #[regex(r"0[0-7]*", priority = 3)]
    #[regex(r"[1-9][0-9]*", priority = 3)]
    INT_LITERAL,

    #[regex(r"(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?", priority = 2)]
    FLOAT_LITERAL,
}

impl From<Token> for SyntaxKind {
    fn from(token: Token) -> Self {
        match token {
            Token::WHITESPACE => SyntaxKind::WHITESPACE,
            Token::NEWLINE => SyntaxKind::NEWLINE,
            Token::COMMENT_LINE => SyntaxKind::COMMENT_LINE,
            Token::COMMENT_BLOCK => SyntaxKind::COMMENT_BLOCK,
            Token::CONST_KW => SyntaxKind::CONST_KW,
            Token::INT_KW => SyntaxKind::INT_KW,
            Token::FLOAT_KW => SyntaxKind::FLOAT_KW,
            Token::VOID_KW => SyntaxKind::VOID_KW,
            Token::IF_KW => SyntaxKind::IF_KW,
            Token::ELSE_KW => SyntaxKind::ELSE_KW,
            Token::WHILE_KW => SyntaxKind::WHILE_KW,
            Token::BREAK_KW => SyntaxKind::BREAK_KW,
            Token::CONTINUE_KW => SyntaxKind::CONTINUE_KW,
            Token::RETURN_KW => SyntaxKind::RETURN_KW,
            Token::STRUCT_KW => SyntaxKind::STRUCT_KW,
            Token::IMPL_KW => SyntaxKind::IMPL_KW,
            Token::EQ => SyntaxKind::EQ,
            Token::SEMI => SyntaxKind::SEMI,
            Token::COMMA => SyntaxKind::COMMA,
            Token::L_BRACE => SyntaxKind::L_BRACE,
            Token::R_BRACE => SyntaxKind::R_BRACE,
            Token::L_PAREN => SyntaxKind::L_PAREN,
            Token::R_PAREN => SyntaxKind::R_PAREN,
            Token::L_BRACK => SyntaxKind::L_BRACK,
            Token::R_BRACK => SyntaxKind::R_BRACK,
            Token::STAR => SyntaxKind::STAR,
            Token::DOT => SyntaxKind::DOT,
            Token::ARROW => SyntaxKind::ARROW,
            Token::PLUS => SyntaxKind::PLUS,
            Token::MINUS => SyntaxKind::MINUS,
            Token::SLASH => SyntaxKind::SLASH,
            Token::PERCENT => SyntaxKind::PERCENT,
            Token::EQEQ => SyntaxKind::EQEQ,
            Token::NEQ => SyntaxKind::NEQ,
            Token::LT => SyntaxKind::LT,
            Token::GT => SyntaxKind::GT,
            Token::LTEQ => SyntaxKind::LTEQ,
            Token::GTEQ => SyntaxKind::GTEQ,
            Token::AMPAMP => SyntaxKind::AMPAMP,
            Token::PIPEPIPE => SyntaxKind::PIPEPIPE,
            Token::BANG => SyntaxKind::BANG,
            Token::AMP => SyntaxKind::AMP,
            Token::IDENT => SyntaxKind::IDENT,
            Token::INT_LITERAL => SyntaxKind::INT_LITERAL,
            Token::FLOAT_LITERAL => SyntaxKind::FLOAT_LITERAL,
        }
    }
}

/// 词法分析器
pub struct Lexer<'a> {
    tokens: Vec<(SyntaxKind, &'a str)>,
    pos: usize,
    pos_skip_trivia: usize,
    pos_skip_trivia_1: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut tokens = Vec::new();
        let inner = Token::lexer(text).spanned();

        for (res, span) in inner {
            let kind = match res {
                Ok(token) => token.into(),
                Err(_) => SyntaxKind::ERROR,
            };
            tokens.push((kind, &text[span]));
        }

        let pos_skip_trivia = Self::get_next_non_trivia_pos(&tokens, 0usize);
        let pos_skip_trivia_1 = Self::get_next_non_trivia_pos(&tokens, pos_skip_trivia + 1);

        Self {
            tokens,
            pos: 0,
            pos_skip_trivia,
            pos_skip_trivia_1,
        }
    }

    /// 从当前位置获取第一个非空白 token 的位置
    fn get_next_non_trivia_pos(tokens: &[(SyntaxKind, &str)], start_pos: usize) -> usize {
        let mut pos = start_pos;
        while pos < tokens.len() {
            let kind = tokens[pos].0;
            if !kind.is_trivia() {
                break;
            }
            pos += 1;
        }
        pos
    }

    /// 返回当前 token 类型
    pub fn current(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.0)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// 返回当前 token 文本
    pub fn current_text(&self) -> &'a str {
        self.tokens.get(self.pos).map(|t| t.1).unwrap_or("")
    }

    /// 返回当前非空白 token 类型
    pub fn current_without_trivia(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos_skip_trivia)
            .map(|t| t.0)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// 返回下一个非空白 token 类型
    pub fn current_without_trivia_1(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos_skip_trivia_1)
            .map(|t| t.0)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// 移动到下一个 token
    pub fn bump(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
            if self.pos > self.pos_skip_trivia {
                self.pos_skip_trivia = self.pos_skip_trivia_1;
                self.pos_skip_trivia_1 =
                    Self::get_next_non_trivia_pos(&self.tokens, self.pos_skip_trivia_1 + 1);
            }
        }
    }

    /// 检查当前 token 是否匹配 `kind`
    pub fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SyntaxKind::*;

    fn check(source: &str, expected_tokens: &[(SyntaxKind, &str)]) {
        let mut lexer = Lexer::new(source);
        for (expected_kind, expected_text) in expected_tokens {
            let kind = lexer.current();
            let text = lexer.current_text();

            assert_eq!(kind, *expected_kind);
            assert_eq!(text, *expected_text);

            lexer.bump();
        }
        assert_eq!(lexer.current(), EOF);
    }

    #[test]
    fn test_lexer_basic() {
        check(
            "const int x = 42;",
            &[
                (CONST_KW, "const"),
                (WHITESPACE, " "),
                (INT_KW, "int"),
                (WHITESPACE, " "),
                (IDENT, "x"),
                (WHITESPACE, " "),
                (EQ, "="),
                (WHITESPACE, " "),
                (INT_LITERAL, "42"),
                (SEMI, ";"),
            ],
        );
    }

    #[test]
    fn test_star_token() {
        check(
            "int* ptr;",
            &[
                (INT_KW, "int"),
                (STAR, "*"),
                (WHITESPACE, " "),
                (IDENT, "ptr"),
                (SEMI, ";"),
            ],
        );
    }

    #[test]
    fn test_comments_and_whitespace() {
        check(
            "// comment\n/* block */ ",
            &[
                (COMMENT_LINE, "// comment"),
                (NEWLINE, "\n"),
                (COMMENT_BLOCK, "/* block */"),
                (WHITESPACE, " "),
            ],
        );
    }
}
