//! Tarqeem Language Server implementation
//!
//! Implements the Language Server Protocol for Tarqeem using tower-lsp.

use crate::error::Language;
use crate::lsp::analysis::DocumentState;
use crate::lsp::capabilities::server_capabilities;
use crate::lsp::handlers::{
    handle_code_actions, handle_completion, handle_definition, handle_document_symbol,
    handle_folding_ranges, handle_formatting, handle_hover, handle_inlay_hints,
    handle_prepare_rename, handle_references, handle_rename, handle_semantic_tokens_full,
    handle_signature_help, publish_diagnostics,
};
use crate::lsp::state::ServerState;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

pub struct TarqeemLanguageServer {
    client: Client,
    documents: DashMap<Url, DocumentState>,
    state: Arc<RwLock<ServerState>>,
}

impl TarqeemLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            state: Arc::new(RwLock::new(ServerState::new())),
        }
    }

    async fn get_language(&self) -> Language {
        self.state.read().await.language()
    }

    async fn publish_diagnostics_for(&self, uri: &Url) {
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(uri) {
            let params = publish_diagnostics(&mut doc, language);
            self.client
                .publish_diagnostics(params.uri, params.diagnostics, params.version)
                .await;
        }
    }
}

#[async_trait]
impl LanguageServer for TarqeemLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            self.state.write().await.set_workspace_root(Some(root_uri));
        }

        if let Some(locale) = params.locale {
            let language = if locale.starts_with("ar") {
                Language::Arabic
            } else {
                Language::English
            };
            self.state.write().await.set_language(language);
        }

        Ok(InitializeResult {
            capabilities: server_capabilities(),
            server_info: Some(ServerInfo {
                name: "tarqeem-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "Tarqeem language server initialized | خادم لغة ترقيم جاهز",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let content = params.text_document.text;

        self.documents.insert(
            uri.clone(),
            DocumentState::new(uri.clone(), version, content),
        );

        self.publish_diagnostics_for(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().next() {
            if let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.update(version, change.text);
            }
        }

        self.publish_diagnostics_for(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);

        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.publish_diagnostics_for(&params.text_document.uri)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_hover(&mut doc, position, language))
        } else {
            Ok(None)
        }
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_signature_help(&mut doc, position, language))
        } else {
            Ok(None)
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_completion(&mut doc, position, language))
        } else {
            Ok(None)
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_definition(&mut doc, position, language))
        } else {
            Ok(None)
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_references(
                &mut doc,
                position,
                include_declaration,
                language,
            ))
        } else {
            Ok(None)
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_prepare_rename(&mut doc, position, language))
        } else {
            Ok(None)
        }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_rename(&mut doc, position, new_name, language))
        } else {
            Ok(None)
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            if let Some(symbols) = handle_document_symbol(&mut doc, language) {
                Ok(Some(DocumentSymbolResponse::Nested(symbols)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        if let Some(doc) = self.documents.get(&uri) {
            Ok(handle_formatting(&doc.content))
        } else {
            Ok(None)
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_code_actions(&mut doc, range, language))
        } else {
            Ok(None)
        }
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_folding_ranges(&mut doc, language))
        } else {
            Ok(None)
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_semantic_tokens_full(&mut doc, language))
        } else {
            Ok(None)
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let language = self.get_language().await;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            Ok(handle_inlay_hints(&mut doc, range, language))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_capabilities_p0_p1() {
        let caps = server_capabilities();
        assert!(caps.hover_provider.is_some());
        assert!(caps.completion_provider.is_some());
        assert!(caps.definition_provider.is_some());
        assert!(caps.signature_help_provider.is_some());
        assert!(caps.references_provider.is_some());
        assert!(caps.rename_provider.is_some());
        assert!(caps.document_symbol_provider.is_some());
        assert!(caps.document_formatting_provider.is_some());
    }

    #[test]
    fn test_server_capabilities_p2() {
        let caps = server_capabilities();
        assert!(caps.code_action_provider.is_some());
        assert!(caps.inlay_hint_provider.is_some());
        assert!(caps.semantic_tokens_provider.is_some());
        assert!(caps.folding_range_provider.is_some());
    }
}
