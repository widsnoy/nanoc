use crate::parse::Parser;
use syntax::SyntaxKind;

impl Parser<'_> {
    pub(super) fn parse_name(&mut self) {
        self.start_node(SyntaxKind::NAME);
        self.expect_or_else_recovery(SyntaxKind::IDENT, SyntaxKind::is_decl_recovery);
        self.finish_node();
    }

    /// 解析基础类型: [const] PrimitType | Pointer Type | '[' Type ';' Expr ']'
    pub(super) fn parse_type(&mut self) {
        self.start_node(SyntaxKind::TYPE);
        if self.at(SyntaxKind::STAR) {
            self.parse_pointer();
            self.parse_type();
        } else if self.at(SyntaxKind::L_BRACK) {
            self.bump();
            self.parse_type(); // 数组内部是 Type，允许 const
            self.expect_or_else_recovery(SyntaxKind::SEMI, SyntaxKind::is_decl_recovery);
            self.parse_exp();
            self.expect_or_else_recovery(SyntaxKind::R_BRACK, SyntaxKind::is_decl_recovery);
        } else {
            // 处理可选的 const 前缀
            if self.at(SyntaxKind::CONST_KW) {
                self.bump();
            }
            self.parse_primitive_type();
        }
        self.finish_node();
    }

    /// 解析原始类型: 'void' | 'i32' | 'f32' | 'struct' Name
    pub(super) fn parse_primitive_type(&mut self) {
        self.start_node(SyntaxKind::PRIMIT_TYPE);
        let current_token = self.peek();
        if matches!(
            current_token,
            SyntaxKind::INT_KW | SyntaxKind::FLOAT_KW | SyntaxKind::VOID_KW
        ) {
            self.bump();
        } else if current_token == SyntaxKind::STRUCT_KW {
            self.bump();
            self.parse_name();
        }
        self.finish_node();
    }

    /// 解析指针: '*' ('mut' | 'const')
    pub(super) fn parse_pointer(&mut self) {
        self.start_node(SyntaxKind::POINTER);
        self.bump(); // consume '*'
        if self.at(SyntaxKind::MUT_KW) || self.at(SyntaxKind::CONST_KW) {
            self.bump();
        } else {
            // 报错：期望 'mut' 或 'const'
            self.skip_until(&[
                SyntaxKind::MUT_KW,
                SyntaxKind::CONST_KW,
                SyntaxKind::INT_KW,
                SyntaxKind::FLOAT_KW,
                SyntaxKind::VOID_KW,
                SyntaxKind::STRUCT_KW,
                SyntaxKind::SEMI,
                SyntaxKind::EOF,
            ]);
        }
        self.finish_node();
    }
}
