//! Server state management
//!
//! Manages the state of the LSP server including open documents,
//! configuration, and cached analysis results.

use crate::error::Language;
use crate::lsp::analysis::DocumentState;
use dashmap::DashMap;
use std::sync::Arc;
use tower_lsp::lsp_types::Url;

pub struct ServerState {
    documents: DashMap<Url, DocumentState>,
    language: Language,
    workspace_root: Option<Url>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
            language: Language::Arabic,
            workspace_root: None,
        }
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn set_workspace_root(&mut self, root: Option<Url>) {
        self.workspace_root = root;
    }

    pub fn workspace_root(&self) -> Option<&Url> {
        self.workspace_root.as_ref()
    }

    pub fn open_document(&self, uri: Url, version: i32, content: String) {
        self.documents
            .insert(uri.clone(), DocumentState::new(uri, version, content));
    }

    pub fn update_document(&self, uri: &Url, version: i32, content: String) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.update(version, content);
        }
    }

    pub fn close_document(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get_document(
        &self,
        uri: &Url,
    ) -> Option<dashmap::mapref::one::Ref<'_, Url, DocumentState>> {
        self.documents.get(uri)
    }

    pub fn get_document_mut(
        &self,
        uri: &Url,
    ) -> Option<dashmap::mapref::one::RefMut<'_, Url, DocumentState>> {
        self.documents.get_mut(uri)
    }

    pub fn document_uris(&self) -> Vec<Url> {
        self.documents.iter().map(|r| r.key().clone()).collect()
    }

    pub fn is_document_open(&self, uri: &Url) -> bool {
        self.documents.contains_key(uri)
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedState = Arc<tokio::sync::RwLock<ServerState>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_lifecycle() {
        let state = ServerState::new();

        // Open
        state.open_document(uri.clone(), 1, "متغير س = 5".to_string());
        assert!(state.is_document_open(&uri));

        // Update
        state.update_document(&uri, 2, "متغير ص = 10".to_string());
        {
            let doc = state.get_document(&uri).unwrap();
            assert_eq!(doc.version, 2);
            assert_eq!(doc.content, "متغير ص = 10");
        }

        // Close
        state.close_document(&uri);
        assert!(!state.is_document_open(&uri));
    }

    #[test]
    fn test_language_setting() {
        let mut state = ServerState::new();
        assert_eq!(state.language(), Language::Arabic);

        state.set_language(Language::English);
        assert_eq!(state.language(), Language::English);
    }
}
