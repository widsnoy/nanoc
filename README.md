# airyc

A toy programming language

## Grammar

```text
CompUnit    := {Header}{GlobalDecl}

Header      := 'import' Path

GlobalDecl  := VarDef | FuncDef | StructDef | FuncAttach

Type        := ['const'] PrimitType | Pointer Type | '[' Type ';' Expr ']'
PrimitType  := 'void' | 'i32' | 'f32' | 'struct' Name
Pointer     := '*' ('mut' | 'const')

VarDef      := 'let' Name ':' Type ['=' InitVal] ';'
InitVal     := Expr | '{' [InitVal {',' InitVal}] '}'

FuncDef     :=  FuncSign (';' | Block)
FuncSign    := 'fn' Name '(' [FuncFParams] ')' ['->' Type]
FuncFParams := FuncFParam {',' FuncFParam}
FuncFParam  := Name: Type
FuncRParams := Expr {',' Expr}
FuncAttach  := 'attach' Name Block

StructDef   := 'struct' Name '{' [StructField {',' StructField}] '}'
StructField := Name: Type

Block       := '{' {BlockItem} '}'
BlockItem   := VarDef | Stmt

Stmt        := AssignStmt
             | ExprStmt
             | Block
             | IfStmt
             | WhileStmt
             | BreakStmt
             | ContinueStmt
             | ReturnStmt

AssignStmt  := Expr '=' Expr ';'
ExprStmt    := [Expr] ';'
IfStmt      := 'if' '(' Expr ')' Stmt ['else' Stmt]
WhileStmt   := 'while' '(' Expr ')' Stmt
BreakStmt   := 'break' ';'
ContinueStmt:= 'continue' ';'
ReturnStmt  := 'return' [Expr] ';'

Expr        := BinaryExpr
             | UnaryExpr
             | CallExpr
             | ParenExpr
             | PostfixExpr
             | IndexVal
             | Literal

BinaryExpr  := Expr BinaryOp Expr
BinaryOp    := '||' | '&&' | '==' | '!=' 
             | '<' | '>' | '<=' | '>=' 
             | '+' | '-' | '*' | '/' | '%'

UnaryExpr   := UnaryOp Expr
UnaryOp     := '+' | '-' | '!' | '&' | '*'

PostfixExpr := Expr PostfixOp FieldAccess
PostfixOp   := '.' | '->'

CallExpr    := Name '(' [FuncRParams] ')'
ParenExpr   := '(' Expr ')'
IndexVal    := Name {'[' Expr ']'}
FieldAccess := Name {'[' Expr ']'}

Literal     := IntConst | FloatConst
Name        := Ident
Path        := Ident
```

## Semantic

todo

## Reference
[Rust](https://rust-lang.org/)  
[SysY](https://gitlab.eduxiji.net/csc1/nscscc/compiler2021/-/blob/master/SysY%E8%AF%AD%E8%A8%80%E5%AE%9A%E4%B9%89.pdf)  
[compiler-dev-test-cases](https://github.com/pku-minic/compiler-dev-test-cases/tree/master/testcases)
