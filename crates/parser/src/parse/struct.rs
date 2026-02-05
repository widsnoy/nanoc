use crate::{parse::Parser, syntax_kind::SyntaxKind};

impl Parser<'_> {
    /// 解析结构体定义
    pub(super) fn parse_struct_def(&mut self) -> bool {
        self.start_node(SyntaxKind::STRUCT_DEF);

        if !self.expect_or_else_recovery(SyntaxKind::STRUCT_KW, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }
        if !self.parse_name() {
            self.finish_node();
            return false;
        }
        if !self.expect_or_else_recovery(SyntaxKind::L_BRACE, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }

        // 解析第一个字段
        if !self.at(SyntaxKind::R_BRACE) && !self.parse_struct_field() {
            self.finish_node();
            return false;
        }

        while self.at(SyntaxKind::COMMA) {
            self.bump();
            if self.at(SyntaxKind::R_BRACE) {
                break;
            }
            if !self.parse_struct_field() {
                self.finish_node();
                return false;
            }
        }

        let success =
            self.expect_or_else_recovery(SyntaxKind::R_BRACE, SyntaxKind::is_decl_recovery);
        self.finish_node();
        success
    }

    /// 解析结构体字段
    fn parse_struct_field(&mut self) -> bool {
        self.start_node(SyntaxKind::STRUCT_FIELD);
        if !self.parse_name() {
            self.finish_node();
            return false;
        }
        if !self.expect_or_else_recovery(SyntaxKind::COLON, SyntaxKind::is_decl_recovery) {
            self.finish_node();
            return false;
        }
        let success = self.parse_type();
        self.finish_node();
        success
    }
}
