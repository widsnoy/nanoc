/// 编译单元：整个源代码文件
#[derive(Debug, Clone, PartialEq)]
pub struct CompUnit {
    pub items: Vec<GlobalItem>,
}

/// 顶层条目：声明或函数定义
#[derive(Debug, Clone, PartialEq)]
pub enum GlobalItem {
    Decl(Decl),
    FuncDef(FuncDef),
}

// ========================================================================
// 类型系统
// ========================================================================

/// 基础类型：int, float, 结构体
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Ident(String),
}

/// 函数返回类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuncType {
    Void,
    Type(Type),
}

/// 指针修饰符：例如 * const
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointer {
    pub is_const: bool,
}

// ========================================================================
// 声明 (Declarations)
// ========================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    ConstDecl(ConstDecl),
    VarDecl(VarDecl),
}

/// 常量声明: const int a = 1, b = 2;
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub ty: Type,
    pub defs: Vec<ConstDef>,
}

/// 常量定义: * const a[10] = {1, 2}
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub pointers: Vec<Pointer>, // 指针层级
    pub name: String,
    pub dims: Vec<Expr>,    // 数组维度 (ConstExp)
    pub init: ConstInitVal, // 必须有初值
}

/// 变量声明: int a, b = 2;
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub ty: Type,
    pub defs: Vec<VarDef>,
}

/// 变量定义
#[derive(Debug, Clone, PartialEq)]
pub struct VarDef {
    pub pointers: Vec<Pointer>,
    pub name: String,
    pub dims: Vec<Expr>,       // 数组维度 (ConstExp)
    pub init: Option<InitVal>, // 初值可选
}

/// 常量初始化值
#[derive(Debug, Clone, PartialEq)]
pub enum ConstInitVal {
    Single(Box<Expr>),        // ConstExp
    Array(Vec<ConstInitVal>), // { ... }
}

/// 变量初始化值
#[derive(Debug, Clone, PartialEq)]
pub enum InitVal {
    Single(Box<Expr>),   // Exp
    Array(Vec<InitVal>), // { ... }
}

// ========================================================================
// 函数定义 (Function Definitions)
// ========================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDef {
    pub ret_type: FuncType,
    pub pointers: Vec<Pointer>, // 返回值是指针的情况
    pub name: String,
    pub params: Vec<FuncFParam>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncFParam {
    pub ty: Type,
    pub pointers: Vec<Pointer>,
    pub name: String,
    // 数组维度: 第一维可能是空的 [] (用 None 表示)，后续维度必须是 ConstExp
    // 例如: int a[][10] -> first_dim_is_array=true, rest_dims=[10]
    pub is_array: bool,
    pub dims: Vec<Expr>,
}

// ========================================================================
// 语句 (Statements) & 代码块
// ========================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Lval = Exp;
    Assign(LVal, Expr),
    /// [Exp]; (表达式语句，Exp 可选)
    Exp(Option<Expr>),
    /// Block
    Block(Block),
    /// if (Cond) Stmt [else Stmt]
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    /// while (Cond) Stmt
    While { cond: Expr, body: Box<Stmt> },
    /// break;
    Break,
    /// continue;
    Continue,
    /// return [Exp];
    Return(Option<Expr>),
}

// ========================================================================
// 表达式 (Expressions)
// ========================================================================

/// 左值：可以出现在赋值号左边
#[derive(Debug, Clone, PartialEq)]
pub enum LVal {
    /// Ident {'[' Exp ']'}
    /// 普通变量或数组访问
    Named(String, Vec<Expr>),
    /// * UnaryExp
    /// 指针解引用
    Deref(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// 左值 (作为右值使用时)
    LVal(LVal),
    /// 字面量
    IntConst(i32),
    FloatConst(f64),
    /// 字符串字面量 (虽然语法没明确写，但通常会有)
    StringConst(String),
    /// 函数调用: Ident '(' [FuncRParams] ')'
    Call(String, Vec<Expr>),
    /// 一元运算: + - ! &
    Unary(UnaryOp, Box<Expr>),
    /// 二元运算: + - * / % < > <= >= == != && ||
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Pos, // +
    Neg, // -
    Not, // !
    Addr, // & (取地址)
         // 注意：解引用 (*) 在 LVal::Deref 中处理，不在这里
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}
