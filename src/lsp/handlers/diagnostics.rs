//! Diagnostics publishing
//!
//! Converts Tarqeem diagnostics to LSP format and publishes them.

use crate::error::{Diagnostic, DiagnosticLevel, Language};
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::span_to_range;
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    Location, NumberOrString, PublishDiagnosticsParams, Url,
};

/// Valid category letters for error codes
const VALID_CATEGORIES: &[char] = &['ق', 'ب', 'د', 'ن', 'ص', 'و', 'ت', 'ح', 'م'];

/// Generate documentation URL for an error code.
/// Format: tarqeem://explain/<category>/<code>
fn error_code_url(code: &str) -> Option<Url> {
    let category = code.chars().next()?;

    if !VALID_CATEGORIES.contains(&category) {
        return None;
    }

    Url::parse(&format!("tarqeem://explain/{}/{}", category, code)).ok()
}

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

    // Arabic-only: ترقيم لغة برمجة عربية
    let _language = language; // Mark as used
    let message = &diag.message;

    let range = span_to_range(content, &diag.span);

    let related_information: Option<Vec<DiagnosticRelatedInformation>> = if diag.notes.is_empty() {
        None
    } else {
        Some(
            diag.notes
                .iter()
                .filter_map(|note| {
                    note.span.as_ref().map(|span| {
                        // Arabic-only
                        let note_message = &note.message;
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
        code_description: diag
            .code
            .as_ref()
            .and_then(|c| error_code_url(c).map(|href| CodeDescription { href })),
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
        Url::parse("file:///test.ترقيم").unwrap()
    }

    #[test]
    fn test_convert_diagnostic() {
        // Arabic-only: ترقيم لغة برمجة عربية
        let diag = Diagnostic::error("المتغير 'x' غير معرف", Span::new(0, 1, 1, 1));

        let content = "x";
        let uri = test_uri();

        let lsp_diag = convert_diagnostic(&diag, content, &uri, Language::Arabic);
        assert_eq!(lsp_diag.message, "المتغير 'x' غير معرف");
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));

        // Even with English language setting, output should be Arabic
        let lsp_diag_with_en = convert_diagnostic(&diag, content, &uri, Language::English);
        assert_eq!(lsp_diag_with_en.message, "المتغير 'x' غير معرف");
    }

    #[test]
    fn test_severity_mapping() {
        // Arabic-only: ترقيم لغة برمجة عربية
        let content = "x";
        let span = Span::new(0, 1, 1, 1);
        let uri = test_uri();

        let error = Diagnostic::error("خطأ", span);
        let warning = Diagnostic::warning("تحذير", span);

        let lsp_error = convert_diagnostic(&error, content, &uri, Language::Arabic);
        let lsp_warning = convert_diagnostic(&warning, content, &uri, Language::Arabic);

        assert_eq!(lsp_error.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_warning.severity, Some(DiagnosticSeverity::WARNING));
        // Verify Arabic messages are used
        assert_eq!(lsp_error.message, "خطأ");
        assert_eq!(lsp_warning.message, "تحذير");
    }

    #[test]
    fn test_error_code_url_warning() {
        let url = error_code_url("ح٠٠٠١");
        assert!(url.is_some());
        let url = url.unwrap();
        assert_eq!(url.scheme(), "tarqeem");
        assert_eq!(url.host_str(), Some("explain"));
    }

    #[test]
    fn test_error_code_url_semantic() {
        let url = error_code_url("د٠٣٠١");
        assert!(url.is_some());
        let url = url.unwrap();
        assert_eq!(url.scheme(), "tarqeem");
        assert_eq!(url.host_str(), Some("explain"));
    }

    #[test]
    fn test_error_code_url_invalid_category() {
        let url = error_code_url("X0001");
        assert!(url.is_none());
    }

    #[test]
    fn test_error_code_url_empty() {
        let url = error_code_url("");
        assert!(url.is_none());
    }

    #[test]
    fn test_diagnostic_with_code_description() {
        let content = "x";
        let uri = test_uri();
        let mut diag = Diagnostic::error("خطأ", Span::new(0, 1, 1, 1));
        diag.code = Some("ح٠٠٠١".to_string());

        let lsp_diag = convert_diagnostic(&diag, content, &uri, Language::Arabic);

        assert!(lsp_diag.code_description.is_some());
        let code_desc = lsp_diag.code_description.unwrap();
        assert_eq!(code_desc.href.scheme(), "tarqeem");
        assert_eq!(code_desc.href.host_str(), Some("explain"));
    }

    #[test]
    fn test_diagnostic_without_code_no_description() {
        let content = "x";
        let uri = test_uri();
        let diag = Diagnostic::error("خطأ", Span::new(0, 1, 1, 1));

        let lsp_diag = convert_diagnostic(&diag, content, &uri, Language::Arabic);

        assert!(lsp_diag.code_description.is_none());
    }
}
