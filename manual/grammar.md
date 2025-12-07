CompUnit := {Decl | FuncDef}  
Type := '**int**' | '**float**' | **struct** **Ident**  
Decl := ConstDecl | VarDecl  
ConstDecl := '**const**' Type ConstDef {',' ConstDef} ';'  
ConstDef := {'\*' ['**const**']} **Ident** {'[' ConstExp ']'} '=' ConstInitVal 
ConstInitVal := ConstExp | ['{' [ConstInitVal {',' ConstInitVal}] '}']  
VarDecl := Type VarDef {',' VarDef} ';'  
VarDef := {'\*' ['**const**']}  **Ident** {'[' ConstExp ']'} ['=' InitVal]  
InitVal := Exp | '{' [InitVal {',' InitVal}] '}'  
FuncDef := FuncType **Ident** '(' [FuncFParams] ')' Block  
FuncType := ('**void**' | Type) {'\*' ['**const**']} 
FuncFParams := FuncFParam {',' FuncFParam}  
FuncFParam := Type {'\*' ['**const**']} Ident ['[' ']' {'[' ConstExp ']'}]  
Block := '{' {Decl | Stmt} '}'   
Stmt :=   
    Lval '=' Exp ';' |    
    [Exp] ';' |  
    Block |  
    '**if**' '(' Exp ')' Stmt ['**else**' Stmt] |  
    '**while**' '(' Exp ')' Stmt |  
    '**break**' ';' |  
    '**continue**' ';' |  
    '**return**' [Exp] ';'  
Exp := AddExp   
Lval := **Ident** {'[' Exp ']'} | '\*' UnaryExp
Literal := **IntConst** | **FloatConst**  
FuncCall := **Ident** '(' [FuncRParams] ')'
UnaryOp := '+' | '-' | '!' | '&'
FuncRParams := Exp {',' Exp}  
ParenExp := '(' Exp ')'
PrimaryExp := ParenExp | Lval | Literal | FuncCall  
UnaryExp := PrimaryExp | UnaryOp UnaryExp  
MulExp := UnaryExp | MulExp ('\*' | '/' | '%') UnaryExp
AddExp := MulExp | AddExp ('+' | '-') MulExp  
RelExp := AddExp | RelExp ('<' | '>' | '<=' | '>=') AddExp  
EqExp := RelExp | EqExp ('==' | '!=') RelExp  
LAndExp := EqExp | LAndExp '&&' EqExp  
LOrExp := LOrExp | LOrExp '||' LAndExp  
ConstExp := AddExp
