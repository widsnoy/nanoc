# airyc

Based on SysY language, added structure and pointer support

```text
CompUnit    := {GlobalDecl}
GlobalDecl  := VarDecl | FuncDef | StructDef

Type        := 'int' | 'float' | 'struct' Name

VarDecl     := ['const'] Type VarDef {',' VarDef} ';'
VarDef      := Pointer ArrayDecl ['=' InitVal]
InitVal     := Expr | '{' [InitVal {',' InitVal}] '}'

Pointer     := {'*' ['const']}
ArrayDecl   := Name {'[' Expr ']'}

FuncDef     := FuncType Name '(' [FuncFParams] ')' Block
FuncType    := ('void' | Type) Pointer
FuncFParams := FuncFParam {',' FuncFParam}
FuncFParam  := Type Pointer Name ['[' ']' {'[' Expr ']'}]
FuncRParams := Expr {',' Expr}

StructDef   := 'struct' Name '{' [StructField {',' StructField}] '}'
StructField := Type Pointer ArrayDecl

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
```
