use analyzer::module::Module;
use rowan::NodeOrToken;
use rowan::TextRange;
use syntax::{SyntaxKind, SyntaxToken};
use tools::LineIndex;
use tower_lsp_server::ls_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType};

/// LSP 语义 token 类型定义
pub const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,  // 0 - fn, let, if, while 等
    SemanticTokenType::TYPE,     // 1 - i32, f32, void, 自定义类型
    SemanticTokenType::STRUCT,   // 2 - struct 名称
    SemanticTokenType::FUNCTION, // 3 - 函数名
    SemanticTokenType::VARIABLE, // 4 - 变量（包括参数）
    SemanticTokenType::NUMBER,   // 5 - 数字字面量
    SemanticTokenType::COMMENT,  // 6 - 注释
    SemanticTokenType::OPERATOR, // 7 - 运算符
];

/// 语义 token 修饰符
pub const LEGEND_MODIFIER: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::READONLY,    // 0 - const 变量
    SemanticTokenModifier::DECLARATION, // 1 - 定义处
];

/// 语义 token 构建器
pub struct SemanticTokensBuilder {
    tokens: Vec<SemanticToken>,
    prev_line: u32,
    prev_char: u32,
}

impl SemanticTokensBuilder {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            prev_line: 0,
            prev_char: 0,
        }
    }

    /// 添加一个 token
    pub fn push(
        &mut self,
        line: u32,
        char: u32,
        length: u32,
        token_type: u32,
        token_modifiers: u32,
    ) {
        let delta_line = line - self.prev_line;
        let delta_start = if delta_line == 0 {
            char - self.prev_char
        } else {
            char
        };

        self.tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: token_modifiers,
        });

        self.prev_line = line;
        self.prev_char = char;
    }

    pub fn build(self) -> Vec<SemanticToken> {
        self.tokens
    }
}

/// 计算文档的语义 tokens
pub fn compute_semantic_tokens(module: &Module, line_index: &LineIndex) -> Vec<SemanticToken> {
    let mut builder = SemanticTokensBuilder::new();
    let root = syntax::SyntaxNode::new_root(module.green_tree.clone());

    // 遍历所有 token（包括 trivia）
    for node_or_token in root.descendants_with_tokens() {
        if let NodeOrToken::Token(token) = node_or_token {
            let kind = token.kind();
            let range = token.text_range();

            // 跳过空白符和换行
            if matches!(kind, SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE) {
                continue;
            }

            // 检查是否在 ERROR 节点下
            let in_error = token
                .parent_ancestors()
                .any(|n| n.kind() == SyntaxKind::ERROR);

            // 计算位置
            let (line, col) = line_index.get_row_column(range.start().into());
            let length: u32 = range.len().into();

            // 确定 token 类型和修饰符
            let (token_type, modifiers) = if in_error {
                // 让编辑器的诊断信息来处理错误高亮
                continue;
            } else {
                classify_token(kind, range, module, &token)
            };

            if let Some(token_type) = token_type {
                builder.push(line, col, length, token_type, modifiers);
            }
        }
    }

    builder.build()
}

/// 分类 token（结合语义信息）
fn classify_token(
    kind: SyntaxKind,
    range: TextRange,
    module: &Module,
    token: &SyntaxToken,
) -> (Option<u32>, u32) {
    match kind {
        // 关键字
        SyntaxKind::FN_KW
        | SyntaxKind::LET_KW
        | SyntaxKind::CONST_KW
        | SyntaxKind::MUT_KW
        | SyntaxKind::STRUCT_KW
        | SyntaxKind::IF_KW
        | SyntaxKind::ELSE_KW
        | SyntaxKind::WHILE_KW
        | SyntaxKind::BREAK_KW
        | SyntaxKind::CONTINUE_KW
        | SyntaxKind::RETURN_KW => (Some(0), 0), // KEYWORD

        // 内置类型关键字
        SyntaxKind::INT_KW | SyntaxKind::FLOAT_KW | SyntaxKind::VOID_KW => {
            (Some(1), 0) // TYPE
        }

        // 标识符 - 需要查询语义信息
        SyntaxKind::IDENT => classify_identifier(range, module, token),

        // 字面量
        SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL => (Some(5), 0), // NUMBER

        // 注释
        SyntaxKind::COMMENT_LINE | SyntaxKind::COMMENT_BLOCK => (Some(6), 0), // COMMENT

        // 运算符
        SyntaxKind::PLUS
        | SyntaxKind::MINUS
        | SyntaxKind::STAR
        | SyntaxKind::SLASH
        | SyntaxKind::PERCENT
        | SyntaxKind::EQ
        | SyntaxKind::EQEQ
        | SyntaxKind::NEQ
        | SyntaxKind::LT
        | SyntaxKind::GT
        | SyntaxKind::LTEQ
        | SyntaxKind::GTEQ
        | SyntaxKind::AMP
        | SyntaxKind::AMPAMP
        | SyntaxKind::PIPEPIPE
        | SyntaxKind::BANG => (Some(7), 0), // OPERATOR

        _ => (None, 0),
    }
}

/// 分类标识符
fn classify_identifier(
    range: TextRange,
    module: &Module,
    token: &SyntaxToken,
) -> (Option<u32>, u32) {
    // 直接查询 module 中的变量信息
    if let Some(var) = module.get_varaible_by_range(range.into()) {
        let mut modifiers = 1 << 1;

        // 检查是否为 const
        if var.is_const() {
            modifiers |= 1 << 0; // READONLY
        }

        return (Some(4), modifiers); // VARIABLE
    }

    // funcation
    if let Some(node) = token.parent().and_then(|n| n.parent())
        && matches!(
            node.kind(),
            SyntaxKind::FUNC_SIGN | SyntaxKind::FUNC_ATTACH | SyntaxKind::CALL_EXPR
        )
    {
        return (Some(3), 0);
    }

    // struct
    if let Some(node) = token.parent().and_then(|n| n.parent())
        && matches!(
            node.kind(),
            SyntaxKind::PRIMIT_TYPE | SyntaxKind::STRUCT_DEF
        )
    {
        return (Some(2), 0);
    }

    // 默认为变量（可能是未解析的标识符）
    (Some(4), 0) // VARIABLE
}
