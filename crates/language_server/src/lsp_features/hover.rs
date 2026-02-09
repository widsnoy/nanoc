use analyzer::module::Module;
use tools::LineIndex;
use tower_lsp_server::ls_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::utils::get_at_position::{
    get_function_id_at_position, get_reference_id_at_position, get_struct_id_at_position,
    get_variable_id_at_position,
};
use crate::utils::position_trans::text_range_to_ls_range;

pub(crate) fn hover(pos: Position, line_index: &LineIndex, module: &Module) -> Option<Hover> {
    // 先检查是否为引用，如果是引用则显示被引用元素的定义
    if let Some(ref_id) = get_reference_id_at_position(module, line_index, &pos)
        && let Some(refer) = module.get_reference_by_id(*ref_id)
    {
        return match refer.tag {
            analyzer::module::ReferenceTag::VarRead(variable_id) => {
                build_hover_for_variable(module, variable_id, line_index, refer.range)
            }
            analyzer::module::ReferenceTag::FuncCall(function_id) => {
                build_hover_for_function(module, function_id, line_index, refer.range)
            }
            analyzer::module::ReferenceTag::FieldRead(field_id) => {
                build_hover_for_field(module, field_id, line_index, refer.range)
            }
        };
    }

    // 检查是否为变量定义
    if let Some(var_id) = get_variable_id_at_position(module, line_index, &pos)
        && let Some(variable) = module.get_varaible_by_id(*var_id)
    {
        return build_hover_for_variable(module, *var_id, line_index, variable.range);
    }

    // 检查是否为函数定义
    if let Some(func_id) = get_function_id_at_position(module, line_index, &pos)
        && let Some(function) = module.get_function_by_id(func_id)
    {
        return build_hover_for_function(module, func_id, line_index, function.range);
    }

    // 检查是否为结构体定义
    if let Some(struct_id) = get_struct_id_at_position(module, line_index, &pos)
        && let Some(struct_def) = module.get_struct_by_id(struct_id)
    {
        return build_hover_for_struct(module, struct_id, line_index, struct_def.range);
    }

    None
}

/// 为变量构建 hover 信息
fn build_hover_for_variable(
    module: &Module,
    var_id: analyzer::module::VariableID,
    line_index: &LineIndex,
    range: tools::TextRange,
) -> Option<Hover> {
    let variable = module.get_varaible_by_id(var_id)?;
    let value = module.get_value_by_range(variable.range);
    let signature = format_variable_signature(variable, value);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```rust\n{}\n```", signature),
        }),
        range: Some(text_range_to_ls_range(line_index, range)),
    })
}

/// 为函数构建 hover 信息
fn build_hover_for_function(
    module: &Module,
    func_id: analyzer::module::FunctionID,
    line_index: &LineIndex,
    range: tools::TextRange,
) -> Option<Hover> {
    let function = module.get_function_by_id(func_id)?;
    let signature = format_function_signature(&function, module);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```rust\n{}\n```", signature),
        }),
        range: Some(text_range_to_ls_range(line_index, range)),
    })
}

/// 为结构体构建 hover 信息（展开所有字段）
fn build_hover_for_struct(
    module: &Module,
    struct_id: analyzer::module::StructID,
    line_index: &LineIndex,
    range: tools::TextRange,
) -> Option<Hover> {
    let struct_def = module.get_struct_by_id(struct_id)?;
    let definition = format_struct_definition(&struct_def, module);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```rust\n{}\n```", definition),
        }),
        range: Some(text_range_to_ls_range(line_index, range)),
    })
}

/// 为字段构建 hover 信息
fn build_hover_for_field(
    module: &Module,
    field_id: analyzer::module::FieldID,
    line_index: &LineIndex,
    range: tools::TextRange,
) -> Option<Hover> {
    let field = module.get_field_by_id(field_id)?;
    let signature = format!("{}: {}", field.name, field.ty);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```rust\n{}\n```", signature),
        }),
        range: Some(text_range_to_ls_range(line_index, range)),
    })
}

/// 格式化变量签名
fn format_variable_signature(
    variable: &analyzer::module::Variable,
    value: Option<&analyzer::value::Value>,
) -> String {
    let v = match value {
        Some(analyzer::value::Value::Int(x)) => x.to_string(),
        Some(analyzer::value::Value::Float(x)) => x.to_string(),
        _ => variable.ty.to_string(),
    };
    format!("{}: {}", variable.name, v)
}

/// 格式化函数签名
fn format_function_signature(function: &analyzer::module::Function, module: &Module) -> String {
    let params = function
        .params
        .iter()
        .filter_map(|param_id| {
            let var = module.get_varaible_by_id(*param_id)?;
            Some(format!("{}: {}", var.name, var.ty))
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("fn {}({}) -> {}", function.name, params, function.ret_type)
}

/// 格式化结构体定义（包含所有字段）
fn format_struct_definition(struct_def: &analyzer::module::Struct, module: &Module) -> String {
    if struct_def.fields.is_empty() {
        return format!("struct {} {{}}", struct_def.name);
    }

    let fields = struct_def
        .fields
        .iter()
        .filter_map(|field_id| {
            let field = module.fields.get(field_id.index)?;
            Some(format!("    {}: {}", field.name, field.ty))
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!("struct {} {{\n{},\n}}", struct_def.name, fields)
}
