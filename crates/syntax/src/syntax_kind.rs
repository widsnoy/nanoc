/// 语法树节点类型
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
    LET_KW,      // "let"
    FN_KW,       // "fn"
    MUT_KW,      // "mut"

    IDENT,         // my_var
    INT_LITERAL,   // 123, 0xFF
    FLOAT_LITERAL, // 3.14
    PLUS,          // +
    MINUS,         // -
    STAR,          // *
    SLASH,         // /
    PERCENT,       // %
    EQ,            // =
    EQEQ,          // ==
    NEQ,           // !=
    LT,            // <
    GT,            // >
    LTEQ,          // <=
    GTEQ,          // >=
    AMP,           // &
    AMPAMP,        // &&
    PIPEPIPE,      // ||
    BANG,          // !
    DOT,           // .
    ARROW,         // ->
    COMMA,         // ,
    SEMI,          // ;
    COLON,         // :
    L_PAREN,       // (
    R_PAREN,       // )
    L_BRACE,       // {
    R_BRACE,       // }
    L_BRACK,       // [
    R_BRACK,       // ]

    // 语法树节点
    COMP_UNIT,

    FUNC_DEF,
    VAR_DEF,
    INIT_VAL,
    STRUCT_DEF,
    STRUCT_FIELD,

    PRIMIT_TYPE,
    TYPE,

    POINTER,
    NAME,
    FUNC_F_PARAMS,
    FUNC_F_PARAM,
    FUNC_R_PARAMS,
    LITERAL,

    UNARY_OP,
    BINARY_OP,
    POSTFIX_OP,

    BINARY_EXPR,
    UNARY_EXPR,
    POSTFIX_EXPR,
    CALL_EXPR,
    PAREN_EXPR,

    INDEX_VAL,
    FIELD_ACCESS,

    BLOCK,
    IF_STMT,
    WHILE_STMT,
    ASSIGN_STMT,
    EXPR_STMT,
    BREAK_STMT,
    CONTINUE_STMT,
    RETURN_STMT,

    __LAST,
}

impl SyntaxKind {
    /// 检查是否为空白字符（空格和注释）
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::WHITESPACE
                | SyntaxKind::NEWLINE
                | SyntaxKind::COMMENT_LINE
                | SyntaxKind::COMMENT_BLOCK
        )
    }

    /// 检查是否为关键字
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
                | SyntaxKind::LET_KW
                | SyntaxKind::FN_KW
                | SyntaxKind::MUT_KW
        )
    }

    /// 检查是否为一元运算符：`+`, `-`, `!`, `&`
    pub fn is_unary_op(self) -> bool {
        matches!(
            self,
            SyntaxKind::PLUS
                | SyntaxKind::MINUS
                | SyntaxKind::BANG
                | SyntaxKind::AMP
                | SyntaxKind::STAR
        )
    }

    /// 检查是否为二元运算符
    pub fn is_binary_op(self) -> bool {
        matches!(
            self,
            SyntaxKind::PLUS
                | SyntaxKind::MINUS
                | SyntaxKind::STAR
                | SyntaxKind::SLASH
                | SyntaxKind::PERCENT
                | SyntaxKind::EQEQ
                | SyntaxKind::NEQ
                | SyntaxKind::LT
                | SyntaxKind::GT
                | SyntaxKind::LTEQ
                | SyntaxKind::GTEQ
                | SyntaxKind::AMPAMP
                | SyntaxKind::PIPEPIPE
        )
    }

    /// 检查是否是后缀运算符
    pub fn is_postfix_op(self) -> bool {
        matches!(self, SyntaxKind::ARROW | SyntaxKind::DOT)
    }

    /// 检查是否为数字字面量
    pub fn is_number(self) -> bool {
        matches!(self, SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL)
    }
}

impl SyntaxKind {
    /// 检查是否为表达式恢复点
    pub fn is_expr_recovery(self) -> bool {
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
    pub fn is_stmt_recovery(self) -> bool {
        matches!(
            self,
            SyntaxKind::SEMI
                | SyntaxKind::R_BRACE
                | SyntaxKind::IF_KW
                | SyntaxKind::WHILE_KW
                | SyntaxKind::RETURN_KW
                | SyntaxKind::LET_KW
                | SyntaxKind::EOF
        )
    }

    /// 检查是否为声明恢复点
    pub fn is_decl_recovery(self) -> bool {
        matches!(
            self,
            SyntaxKind::IF_KW
                | SyntaxKind::WHILE_KW
                | SyntaxKind::RETURN_KW
                | SyntaxKind::STRUCT_KW
                | SyntaxKind::LET_KW
                | SyntaxKind::FN_KW
                | SyntaxKind::EOF
                | SyntaxKind::SEMI
        )
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

/// Airyc 语言定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AirycLanguage {}

impl rowan::Language for AirycLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}
