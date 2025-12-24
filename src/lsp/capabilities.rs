//! LSP server capabilities configuration
//!
//! Defines what features the Tarqeem language server supports.

use crate::lsp::handlers::get_semantic_tokens_legend;
use tower_lsp::lsp_types::{
    CodeActionProviderCapability, CompletionOptions, FoldingRangeProviderCapability,
    HoverProviderCapability, InlayHintOptions, InlayHintServerCapabilities, OneOf, RenameOptions,
    SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensServerCapabilities,
    ServerCapabilities, SignatureHelpOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkDoneProgressOptions,
};

pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: None,
            },
        )),

        hover_provider: Some(HoverProviderCapability::Simple(true)),

        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some(vec![
                ".".to_string(),  // Member access
                ":".to_string(),  // Type annotation
                "\"".to_string(), // String imports
                "/".to_string(),  // Path imports
            ]),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
            all_commit_characters: None,
            completion_item: None,
        }),

        definition_provider: Some(OneOf::Left(true)),

        references_provider: Some(OneOf::Left(true)),

        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
        })),

        document_symbol_provider: Some(OneOf::Left(true)),

        workspace_symbol_provider: Some(OneOf::Left(true)),

        document_formatting_provider: Some(OneOf::Left(true)),

        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),

        code_lens_provider: None,

        document_link_provider: None,

        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),

        selection_range_provider: None,

        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: get_semantic_tokens_legend(),
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
            },
        )),

        inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
            InlayHintOptions {
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
            },
        ))),

        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec![
                "(".to_string(), // Opening paren
                "،".to_string(), // Arabic comma
                ",".to_string(), // English comma
            ]),
            retrigger_characters: Some(vec![
                "،".to_string(), // Arabic comma
                ",".to_string(), // English comma
            ]),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
        }),

        type_definition_provider: None,

        implementation_provider: None,

        declaration_provider: None,

        execute_command_provider: None,

        workspace: None,

        experimental: None,

        moniker_provider: None,

        linked_editing_range_provider: None,

        inline_value_provider: None,

        diagnostic_provider: None,

        call_hierarchy_provider: None,

        document_on_type_formatting_provider: None,

        document_range_formatting_provider: None,

        color_provider: None,

        document_highlight_provider: None,

        position_encoding: None,

        ..Default::default()
    }
}
