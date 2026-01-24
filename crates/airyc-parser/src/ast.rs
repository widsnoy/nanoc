use crate::syntax_kind::{
    NanocLanguage,
    SyntaxKind::{self, *},
};

pub type SyntaxNode = rowan::SyntaxNode<NanocLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<NanocLanguage>;

pub trait AstNode {
    type Language: rowan::Language;
    fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool;
    fn cast(syntax: rowan::SyntaxNode<Self::Language>) -> Option<Self>
    where
        Self: Sized;
    fn syntax(&self) -> &rowan::SyntaxNode<Self::Language>;
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
            type Language = NanocLanguage;
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
            type Language = NanocLanguage;
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

// 1. 编译单元
ast_node!(
    CompUnit ~ COMP_UNIT {
        global_decls: nodes(GlobalDecl),
    }
);

ast_enum!(GlobalDecl { VarDecl, FuncDef });

// 2. 声明
ast_node!(
    VarDecl ~ VAR_DECL {
        const_token: token(CONST_KW),
        ty: node(Type),
        var_defs: nodes(VarDef),
    }
);
impl VarDecl {
    pub fn is_const(&self) -> bool {
        self.const_token().is_some()
    }
}

ast_node!(
    VarDef ~ VAR_DEF {
        pointer: node(Pointer),
        index_val: node(IndexVal),
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

// 3. 函数
ast_node!(
    FuncDef ~ FUNC_DEF {
        func_type: node(FuncType),
        name: node(Name),
        params: node(FuncFParams),
        block: node(Block),
    }
);

ast_node!(
    FuncType ~ FUNC_TYPE {
        ty: node(Type),
        pointer: node(Pointer),
        void_token: token(VOID_KW),
    }
);

ast_node!(
    FuncFParams ~ FUNC_F_PARAMS {
        params: nodes(FuncFParam),
    }
);

ast_node!(
    FuncFParam ~ FUNC_F_PARAM {
        ty: node(Type),
        pointer: node(Pointer),
        name: node(Name),
        l_brack_token: token(L_BRACK),
        indices: nodes(Expr),
    }
);

impl FuncFParam {
    pub fn is_array(&self) -> bool {
        self.l_brack_token().is_some()
    }
}

// 4. 块和语句
ast_node!(
    Block ~ BLOCK {
        items: nodes(BlockItem),
    }
);

ast_enum!(BlockItem { VarDecl, Stmt });

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
        lhs: nth(LVal, 0),
        rhs: nth(Expr, 1),
    }
);

ast_enum!(LVal {
    IndexVal,
    DerefExpr
});

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

// 5. 表达式
ast_enum!(Expr {
    BinaryExpr,
    UnaryExpr,
    CallExpr,
    ParenExpr,
    DerefExpr,
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

ast_node!(BinaryOp ~ BINARY_OP {});
ast_node!(UnaryOp ~ UNARY_OP {});

pub trait OpNode: AstNode<Language = NanocLanguage> {
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

ast_node!(
    DerefExpr ~ DEREF_EXPR {
        expr: node(Expr),
    }
);

// 作为右值或声明
ast_node!(
    IndexVal ~ INDEX_VAL {
        name: node(Name),
        indices: nodes(Expr),
    }
);

ast_node!(
    Literal ~ LITERAL {
        int_token: token(INT_LITERAL),
        float_token: token(FLOAT_LITERAL),
    }
);

// 6. 基本元素
ast_node!(
    Type ~ TYPE { int_token: token(INT_KW),
        float_token: token(FLOAT_KW),
        struct_token: token(STRUCT_KW),
        name: node(Name),
    }
);

ast_node!(
    Name ~ NAME {
        ident: token(IDENT),
    }
);

ast_node!(Pointer ~ POINTER {});

impl Pointer {
    /// 返回指针的可变性列表，true 表示可变指针，false 表示不可变指针
    pub fn stars(&self) -> Vec<bool> {
        let iter = self
            .syntax()
            .children_with_tokens()
            .filter_map(|x| x.into_token())
            .filter(|x| !x.kind().is_trivia());
        let mut iter = iter.peekable();
        let mut vec = Vec::new();
        while let Some(_) = iter.next() {
            if let Some(nxt) = iter.peek()
                && nxt.kind() == SyntaxKind::CONST_KW
            {
                iter.next();
                vec.push(false);
            } else {
                vec.push(true);
            }
        }

        vec
    }
}
