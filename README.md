# airyc

Based on SysY language, added structure and pointer support

```text
CompUnit    := {GlobalDecl}
GlobalDecl  := VarDecl | FuncDef

Type        := 'int' | 'float' | 'struct' Name

VarDecl     := ['const'] Type VarDef {',' VarDef} ';'
VarDef      := Pointer IndexVal ['=' InitVal]
InitVal     := Expr | '{' [InitVal {',' InitVal}] '}'

Pointer     := {'*' ['const']}

FuncDef     := FuncType Name '(' [FuncFParams] ')' Block
FuncType    := ('void' | Type) Pointer
FuncFParams := FuncFParam {',' FuncFParam}
FuncFParam  := Type Pointer Name ['[' ']' {'[' Expr ']'}]

Block       := '{' {BlockItem} '}'
BlockItem   := VarDecl | Stmt

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
             | DerefExpr
             | IndexVal
             | Literal

BinaryExpr  := Expr BinaryOp Expr
BinaryOp    := '||' | '&&' | '==' | '!=' 
             | '<' | '>' | '<=' | '>=' 
             | '+' | '-' | '*' | '/' | '%'

UnaryExpr   := UnaryOp Expr
UnaryOp     := '+' | '-' | '!' | '&' | '*'

CallExpr    := Name '(' [FuncRParams] ')'
ParenExpr   := '(' Expr ')'
IndexVal    := Name {'[' Expr ']'}

FuncRParams := Expr {',' Expr}
Literal     := IntConst | FloatConst
Name        := Ident
```
