CompUnit := {Decl | FuncDef}  
Type := '**int**' | '**float**' | **Ident**  
Decl := ConstDecl | VarDecl  
ConstDecl := '**const**' Type ConstDef {',' ConstDef} ';'  
ConstDef := {'\*' ['**const**']} **Ident** {'[' ConstExp ']'} '=' ConstInitVal ';'  
ConstInitVal := ConstExp | ['{' [ConstInitVal {',' ConstInitVal}] '}']  
VarDecl := Type VarDef {',' VarDef} ';'  
VarDef := {'\*' ['**const**']}  **Ident** {'[' ConstExp ']'} ['=' InitVal]  
InitVal := Exp | '{' [InitVal {',' InitVal}] '}'  
FuncDef := FuncType {'\*' ['**const**']} **Ident** '(' [FuncFParams] ')' Block  
FuncType := '**void**' | Type  
FuncFParams := FuncFParam {',' FuncFParam}  
FuncFParam := Type {'\*' ['**const**']} Ident ['[' ']' {'[' Exp ']'}]  
Block := '{' Decl | Stmt '}'  
Stmt := Lval '=' Exp ';' | [Exp] ';' | Block | '**if**' '(' Cond ')' Stmt ['**else**' Stmt] | '**while**' '(' Cond ')' Stmt | '**break**' ';' | '**continue**' ';' | '**return**' [Exp] ';'  
Exp := AddExp  
Cond := LOrExp  
Lval := **Ident** {'[' Exp ']'} | '\*' UnaryExp
PrimaryExp := '(' Exp ')' | Lval | Number 
Number := **IntConst** | **FloatConst**  
UnaryOp := '+' | '-' | '!' | '&'
FuncRParams := Exp {',' Exp}  
UnaryExp := PrimaryExp | **Ident** '(' [FuncRParams] ')' | UnaryOp UnaryExp  
MulExp := UnaryExp | MulExp ('\*' | '/' | '%') UnaryExp  
AddExp := MulExp | AddExp ('+' | '-') MulExp  
RelExp := AddExp | RelExp ('<' | '>' | '<=' | '>=') AddExp  
EqExp := RelExp | EqExp ('==' | '!=') RelExp  
LAndExp := EqExp | LAndExp '&&' EqExp  
LOrExp := LOrExp | LOrExp '||' LAndExp  
ConstExp := AddExp
