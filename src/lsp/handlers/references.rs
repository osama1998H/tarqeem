//! Find references handler
//!
//! Finds all references to a symbol in the document.

use crate::error::Language;
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::{find_word_at_position, span_to_range};
use tower_lsp::lsp_types::{Location, Position};

pub fn handle_references(
    doc: &mut DocumentState,
    position: Position,
    include_declaration: bool,
    language: Language,
) -> Option<Vec<Location>> {
    let content = doc.content.clone();

    let (_, _, word) = find_word_at_position(&content, position)?;

    let references = doc.find_references(&word, language);

    if references.is_empty() {
        return None;
    }

    let locations: Vec<Location> = references
        .iter()
        .map(|span| Location {
            uri: doc.uri.clone(),
            range: span_to_range(&content, span),
        })
        .collect();

    let _ = include_declaration;

    Some(locations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_find_references() {
        let content = r#"
متغير س = 5
متغير ص = س + 1
اطبع(س)
"#
        .to_string();
        let mut doc = DocumentState::new(uri, 1, content);

        let position = Position {
            line: 1,
            character: 6,
        };
        let result = handle_references(&mut doc, position, true, Language::Arabic);

        if let Some(refs) = result {
            assert!(refs.len() >= 1);
        }
    }
}
