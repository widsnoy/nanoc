use tower_lsp_server::ls_types::{CompletionItem, CompletionItemKind};

/// Airyc 语言关键字列表
///
/// 包含所有语言关键字及其分类
const KEYWORDS: &[(&str, &str)] = &[
    // 类型关键字
    ("i32", "32bit integer type"),
    ("f32", "32bit float type"),
    ("void", "void type"),
    // 控制流关键字
    ("if", "condition statement"),
    ("else", "条件分支"),
    ("while", "循环语句"),
    ("break", "跳出循环"),
    ("continue", "继续下一次循环"),
    ("return", "返回语句"),
    // 声明关键字
    ("let", "变量声明"),
    ("fn", "函数声明"),
    ("struct", "结构体声明"),
    ("const", "常量修饰符"),
    ("mut", "可变修饰符"),
    // 其他关键字
    ("attach", "函数附加"),
];

/// 生成所有关键字的补全项
///
/// # 返回值
/// 返回包含所有 Airyc 关键字的 CompletionItem 列表
pub(crate) fn complete_keywords() -> Vec<CompletionItem> {
    KEYWORDS
        .iter()
        .map(|(keyword, description)| CompletionItem {
            label: keyword.to_string(),
            label_details: None,
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(description.to_string()),
            documentation: None,
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: None,
            insert_text_format: None,
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        })
        .collect()
}
