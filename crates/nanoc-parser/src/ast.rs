// use crate::{
//     ast_enum, ast_methods, ast_node,
//     syntax_kind::{NanocLanguage, SyntaxKind},
// };

// pub type SyntaxNode = rowan::SyntaxNode<NanocLanguage>;
// pub type SyntaxToken = rowan::SyntaxToken<NanocLanguage>;
// pub type SyntaxElement = rowan::SyntaxElement<NanocLanguage>;

// pub trait AstNode {
//     fn cast(syntax: SyntaxNode) -> Option<Self>
//     where
//         Self: Sized;
//     fn syntax(&self) -> &SyntaxNode;
// }

// // 根节点
// ast_node!(CompUnit, ROOT);

// // 声明相关
// ast_node!(ConstDecl, CONST_DECL);
// ast_node!(VarDecl, VAR_DECL);
// ast_node!(ConstDef, CONST_DEF);
// ast_node!(VarDef, VAR_DEF);
// ast_node!(InitVal, INIT_VAL);
// ast_node!(ConstInitVal, CONST_INIT_VAL);

// // 函数相关
// ast_node!(FuncDef, FUNC_DEF);
// ast_node!(FuncType, FUNC_TYPE);
// ast_node!(FuncFParams, FUNC_F_PARAMS);
// ast_node!(FuncFParam, FUNC_F_PARAM);
// ast_node!(FuncRParams, FUNC_R_PARAMS);

// // 语句相关
// ast_node!(Block, BLOCK);
// ast_node!(IfStmt, IF_STMT);
// ast_node!(WhileStmt, WHILE_STMT);
// ast_node!(AssignStmt, ASSIGN_STMT);
// ast_node!(ExprStmt, EXPR_STMT);
// ast_node!(BreakStmt, BREAK_STMT);
// ast_node!(ContinueStmt, CONTINUE_STMT);
// ast_node!(ReturnStmt, RETURN_STMT);

// // 表达式相关
// ast_node!(BinaryExpr, BINARY_EXPR);
// ast_node!(UnaryExpr, UNARY_EXPR);
// ast_node!(CallExpr, CALL_EXPR);
// ast_node!(ParenExpr, PAREN_EXPR);
// ast_node!(IndexExpr, INDEX_EXPR);
// ast_node!(DerefExpr, DEREF_EXPR);
// ast_node!(Literal, LITERAL);
// ast_node!(Name, NAME);
// ast_node!(ConstExpr, CONST_EXPR);
// ast_node!(Exp, EXPR);

// // 类型
// ast_node!(Type, TYPE);

// // enum
// ast_enum!(Decl { ConstDecl, VarDecl });

// ast_enum!(LVal {
//     IndexExpr,
//     DerefExpr
// });

// ast_enum!(Stmt {
//     Block,
//     IfStmt,
//     WhileStmt,
//     AssignStmt,
//     ExprStmt,
//     BreakStmt,
//     ContinueStmt,
//     ReturnStmt,
// });

// ast_enum!(Expr {
//     BinaryExpr,
//     UnaryExpr,
//     CallExpr,
//     ParenExpr,
//     LVal,
//     Literal,
//     ConstExpr,
//     Exp,
// });

// ast_methods!(CompUnit {
//     func_defs(): FuncDef => children,
//     decls(): Decl => children,
// });

// ast_methods!(FuncDef {
//     func_type(): FuncType => child,
//     name(): Name => child,
//     params(): FuncFParams => child,
//     body(): Block => child,
// });

// ast_methods!(Block {
//     stmts(): Stmt => children,
//     decls(): Decl => children,
// });

// ast_methods!(BinaryExpr {
//     lhs(0): Expr => nth,
//     rhs(1): Expr => nth,
//     op_token(): SyntaxToken => first_token,
// });

// ast_methods!(UnaryExpr {
//     expr(): Expr => child,
//     op_token(): SyntaxToken => first_token,
// });

// ast_methods!(ParenExpr {
//     expr(): Expr => child,
// });

// ast_methods!(CallExpr {
//     name(): Name => child,
//     args(): FuncRParams => child,
// });

// ast_methods!(Name {
//     ident_token(IDENT): SyntaxToken => token,
// });

// ast_methods!(IfStmt {
//     condition(): Expr => child,
//     then_branch(0): Stmt => nth,
//     else_branch(1): Stmt => nth,
// });

// ast_methods!(WhileStmt {
//     condition(): Expr => child,
//     body(): Stmt => child,
// });

// ast_methods!(ReturnStmt {
//     expr(): Expr => child,
// });

// ast_methods!(AssignStmt {
//     lval(): LVal => child,
//     expr(): Expr => child,
// });

// ast_methods!(ConstDecl {
//     ty(): Type => child,
//     defs(): ConstDef => children,
// });

// ast_methods!(VarDecl {
//     ty(): Type => child,
//     defs(): VarDef => children,
// });

// ast_methods!(ConstDef {
//     name(): Name => child,
//     dims(): ConstExpr => children,
//     init_val(): ConstInitVal => child,
// });

// ast_methods!(VarDef {
//     name(): Name => child,
//     dims(): ConstExpr => children,
//     init_val(): InitVal => child,
// });

// ast_methods!(InitVal {
//     expr(): Exp => child,
//     values(): InitVal => children,
// });

// ast_methods!(ConstInitVal {
//     expr(): ConstExpr => child,
//     values(): ConstInitVal => children,
// });

// ast_methods!(FuncType {
//     ty(): Type => child,
// });

// ast_methods!(FuncFParams {
//     params(): FuncFParam => children,
// });

// ast_methods!(FuncFParam {
//     ty(): Type => child,
//     name(): Name => child,
//     dims(): ConstExpr => children,
// });

// ast_methods!(FuncRParams {
//     args(): Exp => children,
// });

// ast_methods!(ExprStmt {
//     expr(): Exp => child,
// });

// ast_methods!(IndexExpr {
//     name(): Name => child,
//     indices(): Exp => children,
// });

// ast_methods!(DerefExpr {
//     unary_expr(): UnaryExpr => child,
// });

// ast_methods!(Literal {
//     value(): SyntaxToken => first_token,
// });

// ast_methods!(Type {
//     name(): Name => child,
//     ty(): SyntaxToken => first_token,
// });

// ast_methods!(ConstExpr {
//     expr(): Expr => child,
// });

// ast_methods!(Exp {
//     expr(): Expr => child,
// });
