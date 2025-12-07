/// 语法树节点的类型
#[derive(Debug, Clone, Hash, Copy, Ord, Eq, PartialEq, PartialOrd)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    WHITESPACE,    // 空格, \t
    NEWLINE,       // \n, \r, \r\n
    COMMENT_LINE,  // // ...
    COMMENT_BLOCK, // /* ... */

    ERROR,
    EOF,
    CONST_KW,    // "const"
    INT_KW,      // "int"
    FLOAT_KW,    // "float"
    VOID_KW,     // "void"
    IF_KW,       // "if"
    ELSE_KW,     // "else"
    WHILE_KW,    // "while"
    BREAK_KW,    // "break"
    CONTINUE_KW, // "continue"
    RETURN_KW,   // "return"
    STRUCT_KW,   // "struct"
    IMPL_KW,     // "impl"

    IDENT,         // my_var
    INT_LITERAL,   // 123, 0xFF
    FLOAT_LITERAL, // 3.14
    // STRING_LITERAL, // "hello"
    PLUS,     // +
    MINUS,    // -
    STAR,     // *
    SLASH,    // /
    PERCENT,  // %
    EQ,       // =
    EQEQ,     // ==
    NEQ,      // !=
    LT,       // <
    GT,       // >
    LTEQ,     // <=
    GTEQ,     // >=
    AMP,      // &
    AMPAMP,   // &&
    PIPEPIPE, // ||
    BANG,     // !
    DOT,      // .
    ARROW,    // ->
    COMMA,    // ,
    SEMI,     // ;
    L_PAREN,  // (
    R_PAREN,  // )
    L_BRACE,  // {
    R_BRACE,  // }
    L_BRACK,  // [
    R_BRACK,  // ]

    ROOT,

    FUNC_DEF,
    CONST_DECL,
    VAR_DECL,

    CONST_DEF,
    VAR_DEF,

    TYPE,
    FUNC_TYPE,
    POINTER,

    LITERAL,
    EXPR,
    CONST_EXPR,
    BINARY_EXPR,
    UNARY_EXPR,
    CALL_EXPR,
    PAREN_EXPR,
    LVAL,

    BLOCK,
    STMT,
    IF_STMT,
    WHILE_STMT,
    ASSIGN_STMT,
    EXPR_STMT,
    BREAK_STMT,
    CONTINUE_STMT,
    RETURN_STMT,
    FUNC_F_PARAMS,
    FUNC_F_PARAM,
    FUNC_R_PARAMS,
    INIT_VAL,
    CONST_INIT_VAL,
    NAME,

    __LAST,
}

impl SyntaxKind {
    /// 判断是否为 Trivia（空白与注释）
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::WHITESPACE
                | SyntaxKind::NEWLINE
                | SyntaxKind::COMMENT_LINE
                | SyntaxKind::COMMENT_BLOCK
        )
    }

    /// 判断是否是关键字
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            SyntaxKind::CONST_KW
                | SyntaxKind::INT_KW
                | SyntaxKind::FLOAT_KW
                | SyntaxKind::VOID_KW
                | SyntaxKind::IF_KW
                | SyntaxKind::ELSE_KW
                | SyntaxKind::WHILE_KW
                | SyntaxKind::BREAK_KW
                | SyntaxKind::CONTINUE_KW
                | SyntaxKind::RETURN_KW
                | SyntaxKind::STRUCT_KW
                | SyntaxKind::IMPL_KW
        )
    }

    /// 判断是否为 ``+``, ``-``, ``!``, ``&``
    pub fn is_unary_op(self) -> bool {
        matches!(
            self,
            SyntaxKind::PLUS | SyntaxKind::MINUS | SyntaxKind::BANG | SyntaxKind::AMP
        )
    }

    /// 判断是否为数字
    pub fn is_number(self) -> bool {
        matches!(self, SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL)
    }
}

impl From<u16> for SyntaxKind {
    fn from(value: u16) -> Self {
        assert!(value < SyntaxKind::__LAST as u16);
        unsafe { std::mem::transmute::<u16, SyntaxKind>(value) }
    }
}

impl From<SyntaxKind> for u16 {
    fn from(kind: SyntaxKind) -> Self {
        kind as u16
    }
}

impl From<rowan::SyntaxKind> for SyntaxKind {
    fn from(raw: rowan::SyntaxKind) -> Self {
        SyntaxKind::from(raw.0)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        rowan::SyntaxKind(kind as u16)
    }
}

/// nanoc 定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NanocLanguage {}

impl rowan::Language for NanocLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}
