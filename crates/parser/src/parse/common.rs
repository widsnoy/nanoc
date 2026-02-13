use crate::parse::{Parser, ParserError};
use syntax::SyntaxKind;

impl Parser<'_> {
    /// 解析名称
    pub(super) fn parse_name(&mut self) -> bool {
        self.bump_trivia();
        self.start_node(SyntaxKind::NAME);
        let success = self.expect(SyntaxKind::IDENT);
        self.finish_node();
        success
    }

    /// 解析基础类型: [const] PrimitType | Pointer Type | '[' Type ';' Expr ']'
    pub(super) fn parse_type(&mut self) -> bool {
        self.bump_trivia();
        self.start_node(SyntaxKind::TYPE);

        let success = if self.at(SyntaxKind::STAR) {
            // 指针类型
            if !self.parse_pointer() {
                self.finish_node();
                return false;
            }
            self.parse_type() // 递归解析指向的类型
        } else if self.at(SyntaxKind::L_BRACK) {
            // 数组类型
            self.bump();
            if !self.parse_type() {
                self.finish_node();
                return false;
            }
            if !self.expect(SyntaxKind::SEMI) {
                self.finish_node();
                return false;
            }
            if !self.parse_exp() {
                self.finish_node();
                return false;
            }
            self.expect(SyntaxKind::R_BRACK)
        } else {
            // 基础类型（可能带 const 前缀）
            if self.at(SyntaxKind::CONST_KW) {
                self.bump();
            }
            self.parse_primitive_type()
        };

        self.finish_node();
        success
    }

    /// 解析原始类型: 'void' | 'i32' | 'f32' | 'struct' Name
    pub(super) fn parse_primitive_type(&mut self) -> bool {
        self.bump_trivia();
        self.start_node(SyntaxKind::PRIMIT_TYPE);
        let current_token = self.peek();

        let success = if matches!(
            current_token,
            SyntaxKind::I32_KW
                | SyntaxKind::I8_KW
                | SyntaxKind::U8_KW
                | SyntaxKind::U32_KW
                | SyntaxKind::I64_KW
                | SyntaxKind::U64_KW
                | SyntaxKind::BOOL_KW
                | SyntaxKind::VOID_KW
        ) {
            self.bump();
            true
        } else if current_token == SyntaxKind::STRUCT_KW {
            self.bump();
            self.parse_name() // 传播返回值
        } else {
            let range = self.current_range();
            self.bump(); // eat it
            self.parse_errors.push(ParserError::Expected {
                expected: vec![SyntaxKind::PRIMIT_TYPE],
                range,
            });
            false
        };

        self.finish_node();
        success
    }

    /// 解析指针: '*' ('mut' | 'const')
    pub(super) fn parse_pointer(&mut self) -> bool {
        self.start_node(SyntaxKind::POINTER);
        self.bump(); // consume '*'

        if self.at(SyntaxKind::MUT_KW) || self.at(SyntaxKind::CONST_KW) {
            self.bump();
            self.finish_node();
            true
        } else {
            // 报错：期望 'mut' 或 'const'
            self.skip_until(&[SyntaxKind::MUT_KW, SyntaxKind::CONST_KW, SyntaxKind::EOF]);
            self.finish_node();
            false
        }
    }
}
