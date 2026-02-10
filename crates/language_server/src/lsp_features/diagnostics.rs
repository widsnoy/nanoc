use crate::utils::position_trans::text_range_to_ls_range;
use analyzer::error::AnalyzeError;
use miette::Diagnostic as _;
use tools::LineIndex;
use tower_lsp_server::ls_types::*;

/// 将所有错误转换为 LSP Diagnostic
pub fn compute_diagnostics(errors: &[AnalyzeError], line_index: &LineIndex) -> Vec<Diagnostic> {
    errors
        .iter()
        .filter_map(|error| error_to_diagnostic(error, line_index))
        .collect()
}

/// 将单个错误转换为 LSP Diagnostic
fn error_to_diagnostic(error: &AnalyzeError, line_index: &LineIndex) -> Option<Diagnostic> {
    let message = error.to_string();
    let code = error.code().map(|c| c.to_string());
    let range = text_range_to_ls_range(line_index, *error.range());

    Some(Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: code.map(NumberOrString::String),
        message,
        source: None,
        ..Default::default()
    })
}
