use tools::TextRange;

use crate::syntax_kind::{
    AirycLanguage,
    SyntaxKind::{self, *},
};

pub type SyntaxNode = rowan::SyntaxNode<AirycLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<AirycLanguage>;

pub trait AstNode {
    type Language: rowan::Language;
    fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool;
    fn cast(syntax: rowan::SyntaxNode<Self::Language>) -> Option<Self>
    where
        Self: Sized;
    fn syntax(&self) -> &rowan::SyntaxNode<Self::Language>;

    fn text_range(&self) -> TextRange {
        TextRange(self.syntax().text_range())
    }
}

macro_rules! ast_node {
    (
        $Name:ident~$Kind:path {
            $( $method:ident : $handler:ident $( ( $($arg:tt)* ) )? ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $Name { syntax: SyntaxNode }
        impl AstNode for $Name {
            type Language = AirycLanguage;
            fn can_cast(kind: SyntaxKind) -> bool { matches!(kind, $Kind) }
            fn cast(syntax: SyntaxNode) -> Option<Self> {
                if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }
            }
            fn syntax(&self) -> &SyntaxNode { &self.syntax }
        }
        impl $Name {
            $(
                #[allow(unused)]
                pub fn $method(&self) -> ast_node!(@ret $handler $( ( $($arg)* ) )? ) {
                    ast_node!(@impl self, $handler $( ( $($arg)* ) )? )
                }
            )*
        }
    };

    (@ret node ($Type:ty)) => { Option<$Type> };
    (@ret nodes ($Type:ty)) => { impl Iterator<Item = $Type> };
    (@ret token ($Kind:expr)) => { Option<SyntaxToken> };
    (@ret nth ($Type:ty, $Index:expr)) => { Option<$Type> };

    (@impl $self:ident, node ($Type:ty)) => {
        $self.syntax().children().find_map(<$Type as AstNode>::cast)
    };
    (@impl $self:ident, nodes ($Type:ty)) => {
        $self.syntax().children().filter_map(<$Type as AstNode>::cast)
    };
    (@impl $self:ident, token ($Kind:expr)) => {
        $self.syntax().children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == $Kind)
    };
    (@impl $self:ident, nth ($Type:ty, $Index:expr)) => {
        $self.syntax().children().filter_map(<$Type as AstNode>::cast).nth($Index)
    };
}

macro_rules! ast_enum {
    (
        $Name:ident {
            $($Variant:ident),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $Name {
            $($Variant($Variant),)*
        }
        impl AstNode for $Name {
            type Language = AirycLanguage;
            fn can_cast(kind: SyntaxKind) -> bool {
                $( $Variant::can_cast(kind) )||*
            }
            fn cast(syntax: SyntaxNode) -> Option<Self> {
                let kind = syntax.kind();
                match kind {
                    $(
                        _ if $Variant::can_cast(kind) => Some($Name::$Variant($Variant::cast(syntax).unwrap())),
                    )*
                    _ => None,
                }
            }
            fn syntax(&self) -> &SyntaxNode {
                match self {
                    $($Name::$Variant(it) => it.syntax(),)*
                }
            }
        }
    }
}

// 编译单元
ast_node!(
    CompUnit ~ COMP_UNIT {
        headers: nodes(Header),
        global_decls: nodes(GlobalDecl),
    }
);

ast_node!(
    Header ~ HEADER {
        path: node(Path),
    }
);

ast_node!(
    Path ~ PATH {
        string_literal: token(STRING_LITERAL),
        symbol: token(IDENT),
    }
);

ast_enum!(GlobalDecl {
    VarDef,
    FuncDef,
    StructDef,
    FuncAttach
});

// 声明

ast_node!(
    VarDef ~ VAR_DEF {
        name: node(Name),
        ty: node(Type),
        init: node(InitVal),
    }
);

ast_node!(
    InitVal ~ INIT_VAL {
        expr: node(Expr),
        inits: nodes(InitVal),
    }
);
impl InitVal {
    pub fn is_list(&self) -> bool {
        self.inits().next().is_some()
    }
}

// Struct 定义
ast_node!(
    StructDef ~ STRUCT_DEF {
        name: node(Name),
        fields: nodes(StructField),
    }
);

ast_node!(
    StructField ~ STRUCT_FIELD {
        name: node(Name),
        ty: node(Type),
    }
);

// 函数
ast_node!(
    FuncDef ~ FUNC_DEF {
        sign: node(FuncSign),
        block: node(Block),
    }
);

ast_node!(
    FuncSign ~ FUNC_SIGN {
        name: node(Name),
        params: node(FuncFParams),
        ret_type: node(Type),
    }
);

ast_node!(
    FuncFParams ~ FUNC_F_PARAMS {
        params: nodes(FuncFParam),
    }
);

ast_node!(
    FuncFParam ~ FUNC_F_PARAM {
        name: node(Name),
        ty: node(Type),
    }
);

ast_node!(
    FuncAttach ~ FUNC_ATTACH {
        name: node(Name),
        block: node(Block),
    }
);

// 块和语句
ast_node!(
    Block ~ BLOCK {
        items: nodes(BlockItem),
    }
);

ast_enum!(BlockItem { VarDef, Stmt });

ast_enum!(Stmt {
    AssignStmt,
    ExprStmt,
    Block,
    IfStmt,
    WhileStmt,
    BreakStmt,
    ContinueStmt,
    ReturnStmt,
});

ast_node!(
    AssignStmt ~ ASSIGN_STMT {
        lhs: nth(Expr, 0),
        rhs: nth(Expr, 1),
    }
);

ast_node!(
    ExprStmt ~ EXPR_STMT {
        expr: node(Expr),
    }
);

ast_node!(
    IfStmt ~ IF_STMT {
        condition: node(Expr),
        then_branch: nth(Stmt, 0),
        else_branch: nth(Stmt, 1),
    }
);

ast_node!(
    WhileStmt ~ WHILE_STMT {
        condition: node(Expr),
        body: node(Stmt),
    }
);

ast_node!(BreakStmt ~ BREAK_STMT {});
ast_node!(ContinueStmt ~ CONTINUE_STMT {});

ast_node!(
    ReturnStmt ~ RETURN_STMT {
        expr: node(Expr),
    }
);

// 表达式
ast_enum!(Expr {
    BinaryExpr,
    UnaryExpr,
    CallExpr,
    ParenExpr,
    PostfixExpr,
    IndexVal,
    Literal,
});

ast_node!(
    BinaryExpr ~ BINARY_EXPR {
        lhs: nth(Expr, 0),
        rhs: nth(Expr, 1),
        op: node(BinaryOp),
    }
);

ast_node!(
    UnaryExpr ~ UNARY_EXPR {
        op: node(UnaryOp),
        expr: node(Expr),
    }
);

ast_node!(
    PostfixExpr ~ POSTFIX_EXPR {
        expr: node(Expr),
        op: node(PostfixOp),
        field: node(FieldAccess),
    }
);

ast_node!(BinaryOp ~ BINARY_OP {});
ast_node!(UnaryOp ~ UNARY_OP {});
ast_node!(PostfixOp ~ POSTFIX_OP {});

pub trait OpNode: AstNode<Language = AirycLanguage> {
    fn op(&self) -> SyntaxToken {
        self.syntax()
            .children_with_tokens()
            .find_map(|t| t.into_token().filter(|t| !t.kind().is_trivia()))
            .expect("impossible")
    }
    fn op_str(&self) -> String {
        self.op().text().to_string()
    }
}

impl OpNode for BinaryOp {}
impl OpNode for UnaryOp {}
impl OpNode for PostfixOp {}

ast_node!(
    CallExpr ~ CALL_EXPR {
        name: node(Name),
        args: node(FuncRParams),
    }
);

ast_node!(
    FuncRParams ~ FUNC_R_PARAMS {
        args: nodes(Expr),
    }
);

ast_node!(
    ParenExpr ~ PAREN_EXPR {
        expr: node(Expr),
    }
);

// 表达式中的变量访问
ast_node!(
    IndexVal ~ INDEX_VAL {
        name: node(Name),
        indices: nodes(Expr),
    }
);

// PostfixExpr 中的字段访问
ast_node!(
    FieldAccess ~ FIELD_ACCESS {
        name: node(Name),
        indices: nodes(Expr),
    }
);

ast_node!(
    Literal ~ LITERAL {
        int_token: token(INT_LITERAL),
        float_token: token(FLOAT_LITERAL),
        null_token: token(NULL_KW),
        true_token: token(TRUE_KW),
        false_token: token(FALSE_KW),
    }
);

// 基本元素
ast_node!(
    PrimitType ~ PRIMIT_TYPE {
        i32_token: token(I32_KW),
        i8_token: token(I8_KW),
        f32_token: token(F32_KW),
        bool_token: token(BOOL_KW),
        void_token: token(VOID_KW),
        struct_token: token(STRUCT_KW),
        name: node(Name),
    }
);

ast_node!(
    Type ~ TYPE {
        const_token: token(CONST_KW),
        primit_type: node(PrimitType),
        pointer: node(Pointer),
        inner_type: node(Type),
        size_expr: node(Expr),
        l_brack_token: token(L_BRACK),
    }
);

ast_node!(
    Name ~ NAME {
        ident: token(IDENT),
    }
);

impl Name {
    pub fn var_name(&self) -> Option<String> {
        self.ident().map(|i| i.text().to_string())
    }
    pub fn var_range(&self) -> Option<TextRange> {
        self.ident().map(|i| TextRange(i.text_range()))
    }
}

ast_node!(Pointer ~ POINTER {
    mut_token: token(MUT_KW),
    const_token: token(CONST_KW),
});

impl Pointer {
    pub fn is_const(&self) -> bool {
        self.const_token().is_some()
    }

    pub fn is_mut(&self) -> bool {
        self.mut_token().is_some()
    }
}
