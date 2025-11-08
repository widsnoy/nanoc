#let grammar-table(..body) = {
  table(
    columns: (auto, auto, 1fr),
    align: (left + horizon, center + horizon, left + horizon),
    inset: 5pt,
    stroke: none,
    ..body
  )
}

#let LHS(body) = body

#let arrow = sym.arrow.r

#let comment(body) = {
  text(fill: gray)[#body]
}

#title("nanoc grammar")
#grammar-table(
  [],
  [],
  [],

  LHS[CompUnit],
  arrow,
  [ { CompUnit } ( Decl #sym.bar FuncDef #sym.bar StructDef #sym.bar ImplDef ) ],
  LHS[Decl],
  arrow,
  [ ConstDecl #sym.bar VarDecl ],
  LHS[ConstDecl],
  arrow,
  [ 'const' BType ConstDef { ',' ConstDef } ';' ],
  LHS[BType],
  arrow,
  [ ( 'int' #sym.bar 'float' #sym.bar Ident ) { '\*' } ],
  [],
  [],
  [ #comment("支持指针类型，如 int*, float*, struct S*") ],
  LHS[ConstDef],
  arrow,
  [ Ident { '[' ConstExp ']' } '=' ConstInitVal ],
  LHS[ConstInitVal],
  arrow,
  [ ConstExp #sym.bar '{' [ ConstInitVal { ',' ConstInitVal } ] '}' ],
  LHS[VarDecl],
  arrow,
  [ BType VarDef { ',' VarDef } ';' ],
  LHS[VarDef],
  arrow,
  [ Ident { '[' ConstExp ']' } [ '=' InitVal ] ],
  LHS[InitVal],
  arrow,
  [ Exp #sym.bar '{' [ InitVal { ',' InitVal } ] '}' ],
  LHS[FuncDef],
  arrow,
  [ FuncType Ident '(' [ FuncFParams ] ')' Block ],
  LHS[FuncType],
  arrow,
  [ 'void' #sym.bar 'int' #sym.bar 'float' ],
  LHS[FuncFParams],
  arrow,
  [ FuncFParam { ',' FuncFParam } ],
  LHS[FuncFParam],
  arrow,
  [ BType Ident [ '[' ']' { '[' Exp ']' } ] ],
  LHS[Block],
  arrow,
  [ '{' { BlockItem } '}' ],
  LHS[BlockItem],
  arrow,
  [ Decl #sym.bar Stmt ],
  LHS[Stmt],
  arrow,
  [
    LVal '=' Exp ';' \
    #sym.bar [ Exp ] ';' \
    #sym.bar Block \
    #sym.bar 'if' '(' Cond ')' Stmt [ 'else' Stmt ] \
    #sym.bar 'while' '(' Cond ')' Stmt \
    #sym.bar 'break' ';' \
    #sym.bar 'continue' ';' \
    #sym.bar 'return' [ Exp ] ';'
  ],

  LHS[StructDef],
  arrow,
  [ 'struct' Ident '{' { StructField } '}' ],
  LHS[StructField],
  arrow,
  [ BType Ident { '[' ConstExp ']' } ';' ],
  [],
  [],
  [ #comment("结构体字段定义") ],
  LHS[ImplDef],
  arrow,
  [ 'impl' Ident '{' { MethodDef } '}' ],
  LHS[MethodDef],
  arrow,
  [ FuncType Ident '(' [ FuncFParams ] ')' Block ],
  [],
  [],
  [ #comment("方法隐式带 this 指针") ],
  [],
  [],
  [],

  [],
  [],
  [ #comment("使用 pratt parse") ],

  LHS[Exp],
  arrow,
  [ PrimaryExp \
    #sym.bar Exp BinaryOp Exp \
    #sym.bar UnaryOp Exp \
    #sym.bar Exp '[' Exp ']' \
    #sym.bar Exp '.' Ident \
    #sym.bar Exp '->' Ident \
    #sym.bar Exp '(' [ FuncRParams ] ')' ],
  [],
  [],
  [ #comment("-> 为指针成员访问") ],
  LHS[Cond],
  arrow,
  [ Exp ],
  LHS[LVal],
  arrow,
  [ Ident { '[' Exp ']' #sym.bar '.' Ident } ],
  [],
  [],
  [],
  LHS[PrimaryExp],
  arrow,
  [ '(' Exp ')' #sym.bar Ident #sym.bar Number ],
  LHS[Number],
  arrow,
  [ IntConst #sym.bar floatConst ],
  LHS[UnaryOp],
  arrow,
  [ '+' #sym.bar '−' #sym.bar '!' #sym.bar '&' #sym.bar '\*' ],
  [],
  [],
  [ #comment("& 为取地址，* 为解引用") ],
  LHS[BinaryOp],
  arrow,
  [ '+' #sym.bar '−' #sym.bar '\*' #sym.bar '/' #sym.bar '%' \
    #sym.bar '<' #sym.bar '>' #sym.bar '<=' #sym.bar '>=' \
    #sym.bar '==' #sym.bar '!=' \
    #sym.bar '&&' #sym.bar '||' ],
  [],
  [],
  [],
)
