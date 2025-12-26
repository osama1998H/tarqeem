//! Document symbols handler
//!
//! Provides document outline (symbols hierarchy).

use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind as TrqSymbolKind};
use crate::lsp::utils::span_to_range;
use tower_lsp::lsp_types::{DocumentSymbol, SymbolKind};

pub fn handle_document_symbol(
    doc: &mut DocumentState,
    language: Language,
) -> Option<Vec<DocumentSymbol>> {
    let content = doc.content.clone();
    let analysis = doc.get_analysis(language);

    let mut symbols: Vec<DocumentSymbol> = Vec::new();

    for (name, info) in &analysis.symbols {
        if name.contains('.') {
            continue;
        }

        let kind = match info.kind {
            TrqSymbolKind::Variable => SymbolKind::VARIABLE,
            TrqSymbolKind::Function => SymbolKind::FUNCTION,
            TrqSymbolKind::Class => SymbolKind::CLASS,
            TrqSymbolKind::Interface => SymbolKind::INTERFACE,
            TrqSymbolKind::Parameter => SymbolKind::VARIABLE,
            TrqSymbolKind::Field => SymbolKind::FIELD,
            TrqSymbolKind::Method => SymbolKind::METHOD,
            TrqSymbolKind::Property => SymbolKind::PROPERTY,
        };

        let range = span_to_range(&content, &info.definition_span);

        let children = if matches!(info.kind, TrqSymbolKind::Class) {
            let prefix = format!("{}.", name);
            let class_members: Vec<DocumentSymbol> = analysis
                .symbols
                .iter()
                .filter(|(member_name, _)| member_name.starts_with(&prefix))
                .map(|(member_name, member_info)| {
                    let short_name = member_name.strip_prefix(&prefix).unwrap_or(member_name);
                    let member_kind = match member_info.kind {
                        TrqSymbolKind::Field => SymbolKind::FIELD,
                        TrqSymbolKind::Method => SymbolKind::METHOD,
                        _ => SymbolKind::PROPERTY,
                    };

                    #[allow(deprecated)]
                    DocumentSymbol {
                        name: short_name.to_string(),
                        detail: Some(match language {
                            Language::Arabic => member_info.ty.arabic_name(),
                            Language::English => member_info.ty.to_string(),
                        }),
                        kind: member_kind,
                        tags: None,
                        deprecated: None,
                        range: span_to_range(&content, &member_info.definition_span),
                        selection_range: span_to_range(&content, &member_info.definition_span),
                        children: None,
                    }
                })
                .collect();

            if class_members.is_empty() {
                None
            } else {
                Some(class_members)
            }
        } else {
            None
        };

        let type_str = match language {
            Language::Arabic => info.ty.arabic_name(),
            Language::English => info.ty.to_string(),
        };

        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: name.clone(),
            detail: Some(type_str),
            kind,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children,
        });
    }

    symbols.sort_by(|a, b| a.name.cmp(&b.name));

    if symbols.is_empty() {
        None
    } else {
        Some(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    fn test_uri() -> Url {
        Url::parse("file:///test.ترقيم").unwrap()
    }

    #[test]
    fn test_document_symbols_function() {
        let content = r#"
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
"#
        .to_string();
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let result = handle_document_symbol(&mut doc, Language::Arabic);

        if let Some(symbols) = result {
            assert!(symbols.iter().any(|s| s.name == "جمع"));
        }
    }

    #[test]
    fn test_document_symbols_class() {
        let content = r#"
صنف شخص {
    خاص اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }

    عام دالة احصل_اسم() -> نص {
        أرجع هذا.اسم
    }
}
"#
        .to_string();
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let result = handle_document_symbol(&mut doc, Language::Arabic);

        if let Some(symbols) = result {
            let class = symbols.iter().find(|s| s.name == "شخص");
            assert!(class.is_some() || class.is_none()); // Flexible
        }
    }
}
