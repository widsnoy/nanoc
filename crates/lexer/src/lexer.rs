use logos::Logos;
use syntax::SyntaxKind;
use tools::TextRange;

pub use crate::error::{LexerError, LexerErrorKind};

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
    #[token("import")]
    IMPORT_KW,
    #[token("const")]
    CONST_KW,
    #[token("i32")]
    INT_KW,
    #[token("i8")]
    I8_KW,
    #[token("bool")]
    BOOL_KW,
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
    #[token("let")]
    LET_KW,
    #[token("fn")]
    FN_KW,
    #[token("mut")]
    MUT_KW,
    #[token("attach")]
    ATTACH_KW,
    #[token("null")]
    NULL_KW,
    #[token("true")]
    TRUE_KW,
    #[token("false")]
    FALSE_KW,

    // 运算符和标点符号
    #[token("=")]
    EQ,
    #[token(";")]
    SEMI,
    #[token("::")]
    COLONCOLON,
    #[token(":")]
    COLON,
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
    #[regex(r#""([^"\\]|\\.)*""#)]
    STRING_LITERAL,
    #[regex(r"0[xX][0-9a-fA-F]+", priority = 3)]
    #[regex(r"0[0-7]*", priority = 3)]
    #[regex(r"[1-9][0-9]*", priority = 3)]
    INT_LITERAL,
}

impl From<Token> for SyntaxKind {
    fn from(token: Token) -> Self {
        match token {
            Token::WHITESPACE => SyntaxKind::WHITESPACE,
            Token::NEWLINE => SyntaxKind::NEWLINE,
            Token::COMMENT_LINE => SyntaxKind::COMMENT_LINE,
            Token::COMMENT_BLOCK => SyntaxKind::COMMENT_BLOCK,
            Token::IMPORT_KW => SyntaxKind::IMPORT_KW,
            Token::CONST_KW => SyntaxKind::CONST_KW,
            Token::INT_KW => SyntaxKind::I32_KW,
            Token::I8_KW => SyntaxKind::I8_KW,
            Token::BOOL_KW => SyntaxKind::BOOL_KW,
            Token::VOID_KW => SyntaxKind::VOID_KW,
            Token::IF_KW => SyntaxKind::IF_KW,
            Token::ELSE_KW => SyntaxKind::ELSE_KW,
            Token::WHILE_KW => SyntaxKind::WHILE_KW,
            Token::BREAK_KW => SyntaxKind::BREAK_KW,
            Token::CONTINUE_KW => SyntaxKind::CONTINUE_KW,
            Token::RETURN_KW => SyntaxKind::RETURN_KW,
            Token::STRUCT_KW => SyntaxKind::STRUCT_KW,
            Token::ATTACH_KW => SyntaxKind::ATTACH_KW,
            Token::NULL_KW => SyntaxKind::NULL_KW,
            Token::TRUE_KW => SyntaxKind::TRUE_KW,
            Token::FALSE_KW => SyntaxKind::FALSE_KW,
            Token::FN_KW => SyntaxKind::FN_KW,
            Token::MUT_KW => SyntaxKind::MUT_KW,
            Token::LET_KW => SyntaxKind::LET_KW,
            Token::EQ => SyntaxKind::EQ,
            Token::SEMI => SyntaxKind::SEMI,
            Token::COLONCOLON => SyntaxKind::COLONCOLON,
            Token::COMMA => SyntaxKind::COMMA,
            Token::COLON => SyntaxKind::COLON,
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
            Token::STRING_LITERAL => SyntaxKind::STRING_LITERAL,
            Token::INT_LITERAL => SyntaxKind::INT_LITERAL,
        }
    }
}

/// 词法分析器
pub struct Lexer<'a> {
    tokens: Vec<(SyntaxKind, &'a str, TextRange)>,
    pos: usize,
    pos_skip_trivia: usize,
    pub lexer_errors: Vec<LexerError>,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut tokens = Vec::new();
        let inner = Token::lexer(text).spanned();

        let mut lexer_errors = vec![];
        for (res, span) in inner {
            let kind = match res {
                Ok(token) => token.into(),
                Err(e) => {
                    let err = match e {
                        LexerErrorKind::InvalidInteger => LexerError::InvalidInteger {
                            text: text[span.clone()].to_string(),
                            range: span.clone().into(),
                        },
                        LexerErrorKind::Unknown => LexerError::Unknown {
                            range: span.clone().into(),
                        },
                    };
                    lexer_errors.push(err);
                    SyntaxKind::ERROR
                }
            };
            tokens.push((kind, &text[span.clone()], span.into()));
        }

        let pos_skip_trivia = Self::get_next_non_trivia_pos(&tokens, 0usize);
        Self {
            tokens,
            pos: 0,
            pos_skip_trivia,
            lexer_errors,
        }
    }

    /// 从当前位置获取第一个非空白 token 的位置
    fn get_next_non_trivia_pos(
        tokens: &[(SyntaxKind, &str, TextRange)],
        start_pos: usize,
    ) -> usize {
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
    pub fn current_kind(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.0)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// 返回当前 token 文本
    pub fn current_text(&self) -> &'a str {
        self.tokens.get(self.pos).map(|t| t.1).unwrap_or("")
    }

    /// 返回当前 range
    pub fn current_range(&self) -> TextRange {
        self.tokens
            .get(self.pos_skip_trivia)
            .map(|t| t.2)
            .unwrap_or(
                self.tokens
                    .last()
                    .map(|t| t.2)
                    .unwrap_or(TextRange::new(0, 0)),
            )
    }

    /// 返回当前位置
    pub fn current_pos(&self) -> usize {
        self.pos
    }

    pub fn get_tokens(&self) -> &[(SyntaxKind, &str, TextRange)] {
        &self.tokens
    }

    /// 返回当前非空白 token 类型
    pub fn current_without_trivia(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos_skip_trivia)
            .map(|t| t.0)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// 移动到下一个 token
    pub fn bump(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
            if self.pos > self.pos_skip_trivia {
                self.pos_skip_trivia =
                    Self::get_next_non_trivia_pos(&self.tokens, self.pos_skip_trivia + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SyntaxKind::*;

    fn check(source: &str, expected_tokens: &[(SyntaxKind, &str)]) {
        let mut lexer = Lexer::new(source);
        for (expected_kind, expected_text) in expected_tokens {
            let kind = lexer.current_kind();
            let text = lexer.current_text();

            assert_eq!(kind, *expected_kind);
            assert_eq!(text, *expected_text);

            lexer.bump();
        }
        assert_eq!(lexer.current_kind(), EOF);
    }

    #[test]
    fn test_lexer_basic() {
        check(
            "let x: const i32 = 42;",
            &[
                (LET_KW, "let"),
                (WHITESPACE, " "),
                (IDENT, "x"),
                (COLON, ":"),
                (WHITESPACE, " "),
                (CONST_KW, "const"),
                (WHITESPACE, " "),
                (I32_KW, "i32"),
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
            "let ptr: *i32;",
            &[
                (LET_KW, "let"),
                (WHITESPACE, " "),
                (IDENT, "ptr"),
                (COLON, ":"),
                (WHITESPACE, " "),
                (STAR, "*"),
                (I32_KW, "i32"),
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
