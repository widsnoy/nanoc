// use tower_lsp_server::ls_types::{SemanticToken, SemanticTokenType};
//
// use syntax::SyntaxKind;

// /// LSP 语义 token 类型定义
// pub const LEGEND_TYPE: &[SemanticTokenType] = &[
//     SemanticTokenType::KEYWORD,
//     SemanticTokenType::TYPE,
//     SemanticTokenType::FUNCTION,
//     SemanticTokenType::VARIABLE,
//     SemanticTokenType::NUMBER,
//     SemanticTokenType::STRING,
//     SemanticTokenType::COMMENT,
//     SemanticTokenType::OPERATOR,
//     SemanticTokenType::PARAMETER,
//     SemanticTokenType::STRUCT,
// ];

// /// 将 SyntaxKind 映射到 LSP 语义 token 类型
// pub fn syntax_kind_to_semantic_token_type(kind: SyntaxKind) -> Option<u32> {
//     let token_type = match kind {
//         // 关键字
//         SyntaxKind::FN_KW
//         | SyntaxKind::LET_KW
//         | SyntaxKind::CONST_KW
//         | SyntaxKind::MUT_KW
//         | SyntaxKind::STRUCT_KW
//         | SyntaxKind::IF_KW
//         | SyntaxKind::ELSE_KW
//         | SyntaxKind::WHILE_KW
//         | SyntaxKind::BREAK_KW
//         | SyntaxKind::CONTINUE_KW
//         | SyntaxKind::RETURN_KW => 0, // KEYWORD
//
//         // 类型
//         SyntaxKind::INT_KW | SyntaxKind::FLOAT_KW | SyntaxKind::VOID_KW => 1, // TYPE
//
//         // 标识符（需要更精细的分类）
//         SyntaxKind::IDENT => 3, // VARIABLE
//
//         // 字面量
//         SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL => 4, // NUMBER
//
//         // 注释
//         SyntaxKind::COMMENT_LINE | SyntaxKind::COMMENT_BLOCK => 6, // COMMENT
//
//         // 运算符
//         SyntaxKind::PLUS
//         | SyntaxKind::MINUS
//         | SyntaxKind::STAR
//         | SyntaxKind::SLASH
//         | SyntaxKind::PERCENT
//         | SyntaxKind::EQEQ
//         | SyntaxKind::NEQ
//         | SyntaxKind::LT
//         | SyntaxKind::GT
//         | SyntaxKind::LTEQ
//         | SyntaxKind::GTEQ
//         | SyntaxKind::AMPAMP
//         | SyntaxKind::PIPEPIPE
//         | SyntaxKind::BANG
//         | SyntaxKind::EQ => 7, // OPERATOR
//
//         // 其他 token 不需要语义高亮
//         _ => return None,
//     };
//
//     Some(token_type)
// }
