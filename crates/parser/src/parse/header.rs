//! Header 和 Path 解析
use syntax::SyntaxKind;

use crate::parse::{Parser, ParserError};

impl Parser<'_> {
    pub(crate) fn parse_header(&mut self) {
        self.start_node(SyntaxKind::HEADER);

        if self.at(SyntaxKind::IMPORT_KW) {
            self.bump();
        }

        self.parse_path();

        self.finish_node();
    }

    /// 解析 Path: STRING_LITERAL [:: IDENT]
    /// 例如: "../lib" 或 "../lib"::Symbol
    fn parse_path(&mut self) {
        self.start_node(SyntaxKind::PATH);

        if self.at(SyntaxKind::STRING_LITERAL) {
            self.bump();

            if self.at(SyntaxKind::COLONCOLON) {
                self.bump();

                if !self.expect(SyntaxKind::IDENT) {
                    // expect 已经记录了错误
                }
            }
        } else {
            let range = self.current_range();
            self.parse_errors.push(ParserError::Expected {
                expected: vec![SyntaxKind::STRING_LITERAL],
                range,
            });
        }

        self.finish_node();
    }
}
