// pub(crate) mod keyword;
//
// use analyzer::module::Module;
// use tools::LineIndex;
// use tower_lsp_server::ls_types::{CompletionContext, CompletionItem, CompletionResponse, Position};
//
// pub(crate) fn completion(
//     _pos: Position,
//     context: Option<CompletionContext>,
//     _line_index: &LineIndex,
//     _module: &Module,
//     _text: &str,
// ) -> Option<CompletionResponse> {
//     // let mut items: Vec<CompletionItem> = Vec::new();
//
//     // // 检查触发上下文
//     // if let Some(ctx) = context {
//     //     // 如果是触发字符触发（如 . 或 ->）
//     //     if let Some(trigger_char) = &ctx.trigger_character {
//     //         match trigger_char.as_str() {
//     //             "." => {
//     //                 // TODO: 结构体字段补全
//     //             }
//     //             "->" => {
//     //                 // TODO: 指针解引用补全
//     //             }
//     //             _ => {}
//     //         }
//     //     }
//     // }
// }
