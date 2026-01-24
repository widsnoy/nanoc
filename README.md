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

AssignStmt  := LVal '=' Expr ';'
ExprStmt    := [Expr] ';'
IfStmt      := 'if' '(' Expr ')' Stmt ['else' Stmt]
WhileStmt   := 'while' '(' Expr ')' Stmt
BreakStmt   := 'break' ';'
ContinueStmt:= 'continue' ';'
ReturnStmt  := 'return' [Expr] ';'

Expr        := LOrExpr
LOrExpr     := LAndExpr {'||' LAndExpr}
LAndExpr    := EqExpr {'&&' EqExpr}
EqExpr      := RelExpr {('==' | '!=') RelExpr}
RelExpr     := AddExpr {('<' | '>' | '<=' | '>=') AddExpr}
AddExpr     := MulExpr {('+' | '-') MulExpr}
MulExpr     := UnaryExpr {('*' | '/' | '%') UnaryExpr}

UnaryExpr   := PrimaryExpr
             | DerefExpr
             | UnaryOp UnaryExpr
             | CallExpr
DerefExpr   := '*' UnaryExpr
UnaryOp     := '+' | '-' | '!' | '&'
CallExpr    := Name '(' [FuncRParams] ')'
PrimaryExpr := '(' Expr ')' | LVal | Literal

LVal        := IndexVal | DerefExpr
IndexVal    := Name {'[' Expr ']'}

FuncRParams := Expr {',' Expr}
Literal     := IntConst | FloatConst
Name        := Ident
```
