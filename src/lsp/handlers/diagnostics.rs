//! Diagnostics publishing
//!
//! Converts Tarqeem diagnostics to LSP format and publishes them.

use crate::error::{Diagnostic, DiagnosticLevel, Language};
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::span_to_range;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    NumberOrString, PublishDiagnosticsParams, Url,
};

pub fn publish_diagnostics(
    doc: &mut DocumentState,
    language: Language,
) -> PublishDiagnosticsParams {
    let content = doc.content.clone();
    let uri = doc.uri.clone();
    let version = doc.version;

    let analysis = doc.get_analysis(language);

    let diagnostics: Vec<LspDiagnostic> = analysis
        .diagnostics
        .iter()
        .map(|diag| convert_diagnostic(diag, &content, &uri, language))
        .collect();

    PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: Some(version),
    }
}

fn convert_diagnostic(
    diag: &Diagnostic,
    content: &str,
    uri: &Url,
    language: Language,
) -> LspDiagnostic {
    let severity = match diag.level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        DiagnosticLevel::Info => DiagnosticSeverity::INFORMATION,
        DiagnosticLevel::Hint => DiagnosticSeverity::HINT,
    };

    let message = match language {
        Language::Arabic => &diag.message_ar,
        Language::English => &diag.message,
    };

    let range = span_to_range(content, &diag.span);

    let related_information: Option<Vec<DiagnosticRelatedInformation>> = if diag.notes.is_empty() {
        None
    } else {
        Some(
            diag.notes
                .iter()
                .filter_map(|note| {
                    note.span.as_ref().map(|span| {
                        let note_message = match language {
                            Language::Arabic => &note.message_ar,
                            Language::English => &note.message,
                        };
                        DiagnosticRelatedInformation {
                            location: Location {
                                uri: uri.clone(),
                                range: span_to_range(content, span),
                            },
                            message: note_message.clone(),
                        }
                    })
                })
                .collect(),
        )
    };

    LspDiagnostic {
        range,
        severity: Some(severity),
        code: diag
            .code
            .as_ref()
            .map(|c| NumberOrString::String(c.clone())),
        code_description: None,
        source: Some("tarqeem".to_string()),
        message: message.clone(),
        related_information,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;
    use tower_lsp::lsp_types::Url;

    fn test_uri() -> Url {
        Url::parse("file:///test.trq").unwrap()
    }

    #[test]
    fn test_convert_diagnostic() {
        let diag = Diagnostic::error(
            "Undefined variable 'x'",
            "المتغير 'x' غير معرف",
            Span::new(0, 1, 1, 1),
        );

        let content = "x";
        let uri = test_uri();

        let lsp_diag = convert_diagnostic(&diag, content, &uri, Language::Arabic);
        assert_eq!(lsp_diag.message, "المتغير 'x' غير معرف");
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));

        let lsp_diag_en = convert_diagnostic(&diag, content, &uri, Language::English);
        assert_eq!(lsp_diag_en.message, "Undefined variable 'x'");
    }

    #[test]
    fn test_severity_mapping() {
        let content = "x";
        let span = Span::new(0, 1, 1, 1);
        let uri = test_uri();

        let error = Diagnostic::error("error", "خطأ", span.clone());
        let warning = Diagnostic::warning("warning", "تحذير", span);

        let lsp_error = convert_diagnostic(&error, content, &uri, Language::English);
        let lsp_warning = convert_diagnostic(&warning, content, &uri, Language::English);

        assert_eq!(lsp_error.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_warning.severity, Some(DiagnosticSeverity::WARNING));
    }
}
