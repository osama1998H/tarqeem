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

/// Get the server capabilities
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // Document synchronization
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: None,
            },
        )),

        // Hover support (معلومات عند التحويم)
        hover_provider: Some(HoverProviderCapability::Simple(true)),

        // Completion support (إكمال تلقائي)
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

        // Go to definition (انتقال للتعريف)
        definition_provider: Some(OneOf::Left(true)),

        // Find references (البحث عن المراجع)
        references_provider: Some(OneOf::Left(true)),

        // Rename (إعادة التسمية)
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
        })),

        // Document symbols (مخطط المستند)
        document_symbol_provider: Some(OneOf::Left(true)),

        // Workspace symbols
        workspace_symbol_provider: Some(OneOf::Left(true)),

        // Document formatting (تنسيق الكود)
        document_formatting_provider: Some(OneOf::Left(true)),

        // Code actions (إجراءات الكود)
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),

        // Code lens (not yet implemented)
        code_lens_provider: None,

        // Document links (not yet implemented)
        document_link_provider: None,

        // Folding ranges (طي الكود)
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),

        // Selection ranges (not yet implemented)
        selection_range_provider: None,

        // Semantic tokens (رموز دلالية)
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

        // Inlay hints (تلميحات مضمنة)
        inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
            InlayHintOptions {
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
            },
        ))),

        // Signature help (مساعدة التوقيع)
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

        // Type definition
        type_definition_provider: None,

        // Implementation
        implementation_provider: None,

        // Declaration
        declaration_provider: None,

        // Execute command
        execute_command_provider: None,

        // Workspace
        workspace: None,

        // Experimental
        experimental: None,

        // Moniker
        moniker_provider: None,

        // Linked editing range
        linked_editing_range_provider: None,

        // Inline value
        inline_value_provider: None,

        // Diagnostic (not yet implemented)
        diagnostic_provider: None,

        // Call hierarchy
        call_hierarchy_provider: None,

        // Document on type formatting
        document_on_type_formatting_provider: None,

        // Document range formatting
        document_range_formatting_provider: None,

        // Color provider
        color_provider: None,

        // Document highlight
        document_highlight_provider: None,

        // Position encoding
        position_encoding: None,

        // Use default for any other fields
        ..Default::default()
    }
}
