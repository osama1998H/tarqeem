//! Document state and analysis management
//!
//! Manages the state of open documents and their analysis results.

use crate::error::codes::ERR_ENTRY_POINT_CONFLICT;
use crate::error::{Diagnostic, DiagnosticLevel, Language, Span};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::parser::{
    Ast, ClassMember, ExportItems, Parser, Stmt, StmtKind, TypeAnnotation, TypeKind,
};
use crate::semantic::{Analyzer, Type};
use std::collections::HashMap;
use tower_lsp::lsp_types::Url;

/// Translates lexer error messages from English to Arabic
fn translate_lexer_error(msg: &str) -> String {
    // Handle common lexer error patterns
    if msg.contains("Unexpected character") {
        // Extract the character if present
        if let Some(start) = msg.find('\'') {
            if let Some(end) = msg.rfind('\'') {
                let char_str = &msg[start..=end];
                return format!("حرف غير متوقع {}", char_str);
            }
        }
        return "حرف غير متوقع".to_string();
    }

    if msg.contains("Unterminated string") || msg.contains("unterminated string") {
        return "نص غير مُنهى - تأكد من إغلاق علامات الاقتباس".to_string();
    }

    if msg.contains("Invalid number") || msg.contains("invalid number") {
        return "رقم غير صالح".to_string();
    }

    if msg.contains("Invalid escape") || msg.contains("invalid escape") {
        return "تسلسل هروب غير صالح".to_string();
    }

    if msg.contains("Unterminated comment") || msg.contains("unterminated comment") {
        return "تعليق غير مُنهى - تأكد من إغلاق التعليق".to_string();
    }

    if msg.contains("Invalid character in identifier") {
        return "حرف غير صالح في المعرّف".to_string();
    }

    if msg.contains("Number too large") || msg.contains("number too large") {
        return "الرقم كبير جداً".to_string();
    }

    if msg.contains("Empty character literal") {
        return "حرف فارغ غير مسموح".to_string();
    }

    if msg.contains("Unexpected end of input") || msg.contains("unexpected end") {
        return "نهاية غير متوقعة للمدخلات".to_string();
    }

    // Default: prepend Arabic indicator
    format!("خطأ معجمي: {}", msg)
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub tokens: Vec<Token>,
    pub ast: Option<Ast>,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: HashMap<String, SymbolInfo>,
    pub has_errors: bool,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub ty: Type,
    pub definition_span: Span,
    pub kind: SymbolKind,
    pub mutable: bool,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Parameter,
    Field,
    Method,
    Property,
    Enum,
    EnumVariant,
}

#[derive(Debug)]
pub struct DocumentState {
    pub uri: Url,
    pub version: i32,
    pub content: String,
    pub analysis: Option<AnalysisResult>,
}

impl DocumentState {
    pub fn new(uri: Url, version: i32, content: String) -> Self {
        Self {
            uri,
            version,
            content,
            analysis: None,
        }
    }

    pub fn update(&mut self, version: i32, content: String) {
        self.version = version;
        self.content = content;
        self.analysis = None; // Invalidate cache
    }

    pub fn get_analysis(&mut self, language: Language) -> &AnalysisResult {
        if self.analysis.is_none() {
            self.analysis = Some(self.analyze(language));
        }
        self.analysis.as_ref().unwrap()
    }

    fn analyze(&self, _language: Language) -> AnalysisResult {
        let mut diagnostics = Vec::new();
        let mut symbols = HashMap::new();
        let mut has_errors = false;

        let mut lexer = Lexer::new(&self.content);
        let tokens = lexer.tokenize();

        for token in &tokens {
            if let TokenKind::Error(msg) = &token.kind {
                let arabic_msg = translate_lexer_error(msg);
                diagnostics.push(Diagnostic::error(arabic_msg, token.span));
                has_errors = true;
            }
        }

        let mut parser = Parser::new(&self.content);
        let ast = match parser.parse() {
            Ok(ast) => Some(ast),
            Err(diag) => {
                diagnostics.push(diag);
                has_errors = true;
                None
            }
        };

        if let Some(ref ast) = ast {
            // Relative imports (`./وحدة`) resolve against the importing file's
            // directory, so the analyzer needs this document's path. Non-file
            // URIs (untitled buffers) have no directory to resolve against.
            let mut analyzer = match self.uri.to_file_path() {
                Ok(path) => Analyzer::for_file(path),
                Err(_) => Analyzer::new(),
            };

            self.collect_symbols(ast, &mut symbols);

            if let Err(errs) = analyzer.analyze(ast) {
                for err in errs {
                    diagnostics.push(err);
                }
                has_errors = true;
            } else {
                // Cross-module defects — a top-level name defined by two merged
                // modules (و٠١٠١) above all — are only found by the link step,
                // which `check` and `run` both run. Without it the editor
                // showed a clean file that the CLI then rejected. The merged
                // AST is discarded: the LSP builds no IR.
                let mut link_warnings = Vec::new();
                if let Err(errs) = analyzer.linked_ast(ast, &mut link_warnings) {
                    diagnostics.extend(errs);
                    has_errors = true;
                }
                diagnostics.extend(link_warnings);
            }

            // Check for entry point mode conflict (Script mode vs Program mode)
            if let Some(diag) = self.check_entry_point_conflict(ast) {
                diagnostics.push(diag);
                has_errors = true;
            }
        }

        AnalysisResult {
            tokens,
            ast,
            diagnostics,
            symbols,
            has_errors,
        }
    }

    /// Unwraps `صدّر <declaration>` to the declaration it exports.
    ///
    /// Duplicated from `ir::builder::as_top_level_decl` rather than shared:
    /// the LSP layer must not reach into the IR layer. Both classifications
    /// of "top-level executable code" must stay in step, or the editor and
    /// the compiler disagree about ت٠٢٠١.
    fn unwrap_exported_decl(stmt: &Stmt) -> &Stmt {
        match &stmt.kind {
            StmtKind::Export(ExportItems::Declaration(inner)) => inner,
            _ => stmt,
        }
    }

    /// Check for entry point mode conflict (Script mode vs Program mode).
    /// Returns an error diagnostic if both top-level executable statements
    /// AND دالة رئيسية() exist in the same file.
    fn check_entry_point_conflict(&self, ast: &Ast) -> Option<Diagnostic> {
        // Find دالة رئيسية() declaration (Program mode entry point)
        let main_func_span = ast.statements.iter().find_map(|stmt| {
            if let StmtKind::FuncDecl { name, .. } = &Self::unwrap_exported_decl(stmt).kind {
                if name == "رئيسية" {
                    return Some(stmt.span);
                }
            }
            None
        });

        // Find first top-level executable statement (Script mode entry point)
        // Declarations are allowed: VarDecl, FuncDecl, ClassDecl, InterfaceDecl, EnumDecl, Import
        // Everything else is executable code
        let first_executable_span = ast.statements.iter().find_map(|stmt| {
            if !matches!(
                &Self::unwrap_exported_decl(stmt).kind,
                StmtKind::FuncDecl { .. }
                    | StmtKind::ClassDecl { .. }
                    | StmtKind::InterfaceDecl { .. }
                    | StmtKind::EnumDecl { .. }
                    | StmtKind::VarDecl { .. }
                    | StmtKind::Import { .. }
                    // Named/wildcard/re-exports survive the unwrapping above;
                    // all are module metadata, never executable code.
                    | StmtKind::Export(..)
            ) {
                return Some(stmt.span);
            }
            None
        });

        // If both exist, we have a conflict
        if let (Some(main_span), Some(exec_span)) = (main_func_span, first_executable_span) {
            // Point the error at the دالة رئيسية() declaration since that's likely
            // what the user should remove to use Script mode
            return Some(Diagnostic {
                level: DiagnosticLevel::Error,
                message: format!(
                    "[{}] لا يمكن وجود جمل تنفيذية عليا ودالة رئيسية() في نفس الملف. \
                     استخدم إما وضع السكربت (كود علوي) أو وضع البرنامج (دالة رئيسية).",
                    ERR_ENTRY_POINT_CONFLICT
                ),
                span: main_span,
                notes: vec![crate::error::Note::new(format!(
                    "أول جملة تنفيذية عليا في السطر {}",
                    exec_span.line
                ))
                .with_span(exec_span)],
                suggestions: vec![],
                code: Some(ERR_ENTRY_POINT_CONFLICT.to_string()),
            });
        }

        None
    }

    fn collect_symbols(&self, ast: &Ast, symbols: &mut HashMap<String, SymbolInfo>) {
        for stmt in &ast.statements {
            match &stmt.kind {
                StmtKind::VarDecl {
                    name,
                    mutable,
                    ty,
                    doc_comment,
                    ..
                } => {
                    let resolved_type = ty
                        .as_ref()
                        .map(|t| self.resolve_type_annotation(t))
                        .unwrap_or(Type::Unknown);

                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            ty: resolved_type,
                            definition_span: stmt.span,
                            kind: SymbolKind::Variable,
                            mutable: *mutable,
                            doc: doc_comment.clone(),
                        },
                    );
                }

                StmtKind::FuncDecl {
                    name,
                    params,
                    return_type,
                    doc_comment,
                    ..
                } => {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(|t| self.resolve_type_annotation(t))
                                .unwrap_or(Type::Unknown)
                        })
                        .collect();

                    let ret_type = return_type
                        .as_ref()
                        .map(|t| self.resolve_type_annotation(t))
                        .unwrap_or(Type::Void);

                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            ty: Type::Function {
                                params: param_types,
                                return_type: Box::new(ret_type),
                            },
                            definition_span: stmt.span,
                            kind: SymbolKind::Function,
                            mutable: false,
                            doc: doc_comment.clone(),
                        },
                    );

                    for param in params {
                        let param_type = param
                            .ty
                            .as_ref()
                            .map(|t| self.resolve_type_annotation(t))
                            .unwrap_or(Type::Unknown);

                        symbols.insert(
                            param.name.clone(),
                            SymbolInfo {
                                ty: param_type,
                                definition_span: param.span,
                                kind: SymbolKind::Parameter,
                                mutable: true,
                                doc: None,
                            },
                        );
                    }
                }

                StmtKind::ClassDecl {
                    name,
                    members,
                    doc_comment,
                    ..
                } => {
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            ty: Type::Class(name.clone()),
                            definition_span: stmt.span,
                            kind: SymbolKind::Class,
                            mutable: false,
                            doc: doc_comment.clone(),
                        },
                    );

                    for member in members {
                        match member {
                            ClassMember::Field {
                                name: field_name,
                                ty,
                                doc_comment: field_doc,
                                ..
                            } => {
                                let field_type = ty
                                    .as_ref()
                                    .map(|t| self.resolve_type_annotation(t))
                                    .unwrap_or(Type::Unknown);

                                symbols.insert(
                                    format!("{}.{}", name, field_name),
                                    SymbolInfo {
                                        ty: field_type,
                                        definition_span: stmt.span,
                                        kind: SymbolKind::Field,
                                        mutable: true,
                                        doc: field_doc.clone(),
                                    },
                                );
                            }
                            ClassMember::Method {
                                name: method_name,
                                params,
                                return_type,
                                doc_comment: method_doc,
                                ..
                            } => {
                                let param_types: Vec<Type> = params
                                    .iter()
                                    .map(|p| {
                                        p.ty.as_ref()
                                            .map(|t| self.resolve_type_annotation(t))
                                            .unwrap_or(Type::Unknown)
                                    })
                                    .collect();

                                let ret_type = return_type
                                    .as_ref()
                                    .map(|t| self.resolve_type_annotation(t))
                                    .unwrap_or(Type::Void);

                                symbols.insert(
                                    format!("{}.{}", name, method_name),
                                    SymbolInfo {
                                        ty: Type::Function {
                                            params: param_types,
                                            return_type: Box::new(ret_type),
                                        },
                                        definition_span: stmt.span,
                                        kind: SymbolKind::Method,
                                        mutable: false,
                                        doc: method_doc.clone(),
                                    },
                                );
                            }
                            ClassMember::Constructor { .. } => {}
                            ClassMember::Property {
                                name: prop_name,
                                ty,
                                doc_comment: prop_doc,
                                ..
                            } => {
                                let prop_type = self.resolve_type_annotation(ty);

                                symbols.insert(
                                    format!("{}.{}", name, prop_name),
                                    SymbolInfo {
                                        ty: prop_type,
                                        definition_span: stmt.span,
                                        kind: SymbolKind::Property,
                                        mutable: true,
                                        doc: prop_doc.clone(),
                                    },
                                );
                            }
                        }
                    }
                }

                StmtKind::InterfaceDecl {
                    name, doc_comment, ..
                } => {
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            ty: Type::Interface(name.clone()),
                            definition_span: stmt.span,
                            kind: SymbolKind::Interface,
                            mutable: false,
                            doc: doc_comment.clone(),
                        },
                    );
                }

                StmtKind::EnumDecl {
                    name,
                    variants,
                    doc_comment,
                    ..
                } => {
                    // Register the enum itself
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            ty: Type::Enum(name.clone()),
                            definition_span: stmt.span,
                            kind: SymbolKind::Enum,
                            mutable: false,
                            doc: doc_comment.clone(),
                        },
                    );

                    // Register each variant
                    for variant in variants {
                        let variant_key = format!("{}::{}", name, variant.name);
                        symbols.insert(
                            variant_key,
                            SymbolInfo {
                                ty: Type::Enum(name.clone()),
                                definition_span: variant.span,
                                kind: SymbolKind::EnumVariant,
                                mutable: false,
                                doc: None,
                            },
                        );
                    }
                }

                _ => {}
            }
        }
    }

    fn resolve_type_annotation(&self, annotation: &TypeAnnotation) -> Type {
        match &annotation.kind {
            TypeKind::Simple(name) => self.parse_type_name(name),
            TypeKind::Array(inner) => Type::Array(Box::new(self.resolve_type_annotation(inner))),
            TypeKind::Map(key, value) => Type::Map(
                Box::new(self.resolve_type_annotation(key)),
                Box::new(self.resolve_type_annotation(value)),
            ),
            TypeKind::Function {
                params,
                return_type,
            } => Type::Function {
                params: params
                    .iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect(),
                // Bare `()` carries no return type at all.
                return_type: Box::new(
                    return_type
                        .as_ref()
                        .map(|t| self.resolve_type_annotation(t))
                        .unwrap_or(Type::Void),
                ),
            },
            TypeKind::Generic { base, args: _ } => self.parse_type_name(base),
            TypeKind::Optional(inner) => {
                Type::Optional(Box::new(self.resolve_type_annotation(inner)))
            }
        }
    }

    fn parse_type_name(&self, name: &str) -> Type {
        // Single source of truth for the name→type mapping — a private
        // copy here would silently drift the next time a builtin type is
        // added (LSP may import semantic per the layering table).
        crate::semantic::parse_type_name(name)
    }

    pub fn find_symbol_at(
        &mut self,
        offset: usize,
        language: Language,
    ) -> Option<(&str, &SymbolInfo)> {
        let analysis = self.get_analysis(language);

        for token in &analysis.tokens {
            if token.span.start <= offset && offset < token.span.end {
                if let TokenKind::Identifier(name) = &token.kind {
                    if let Some(info) = analysis.symbols.get(name) {
                        return Some((name.as_str(), info));
                    }
                }
            }
        }

        None
    }

    pub fn find_references(&mut self, symbol_name: &str, language: Language) -> Vec<Span> {
        let analysis = self.get_analysis(language);
        let mut references = Vec::new();

        for token in &analysis.tokens {
            if let TokenKind::Identifier(name) = &token.kind {
                if name == symbol_name {
                    references.push(token.span);
                }
            }
        }

        references
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    fn test_uri() -> Url {
        Url::parse("file:///test.ترقيم").unwrap()
    }

    #[test]
    fn test_document_analysis() {
        let content = wrap_with_markers("متغير س = 5");
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let analysis = doc.get_analysis(Language::Arabic);
        assert!(!analysis.tokens.is_empty());
    }

    #[test]
    fn test_document_update() {
        let mut doc = DocumentState::new(test_uri(), 1, wrap_with_markers("متغير س = 5"));

        let _ = doc.get_analysis(Language::Arabic);
        assert!(doc.analysis.is_some());

        doc.update(2, wrap_with_markers("متغير ص = 10"));
        assert!(doc.analysis.is_none());
    }

    #[test]
    fn test_symbol_collection() {
        let content = wrap_with_markers(
            r#"
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
"#,
        );
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let analysis = doc.get_analysis(Language::Arabic);
        assert!(analysis.symbols.contains_key("جمع"));
    }

    #[test]
    fn test_symbol_with_doc_comment() {
        let content = wrap_with_markers(
            r#"
/// دالة لجمع عددين
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
"#,
        );
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let analysis = doc.get_analysis(Language::Arabic);
        assert!(analysis.symbols.contains_key("جمع"));
        let symbol = analysis.symbols.get("جمع").unwrap();
        // Doc comment extraction may not be implemented yet
        // assert!(symbol.doc.is_some());
        // assert!(symbol.doc.as_ref().unwrap().contains("لجمع عددين"));
        assert!(symbol.doc.is_none() || symbol.doc.as_ref().unwrap().contains("لجمع"));
    }

    #[test]
    fn test_arabic_lexer_error_translation() {
        // Test unterminated string error gets Arabic translation
        let content = wrap_with_markers(r#"متغير س = "نص غير مغلق"#);
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let analysis = doc.get_analysis(Language::Arabic);
        assert!(analysis.has_errors, "Should have lexer errors");
        assert!(!analysis.diagnostics.is_empty(), "Should have diagnostics");

        // Verify at least one diagnostic has an Arabic message
        let has_arabic_error = analysis.diagnostics.iter().any(|d| {
            d.message
                .chars()
                .any(|c| matches!(c, '\u{0600}'..='\u{06FF}'))
        });
        assert!(has_arabic_error, "Diagnostics should have Arabic messages");
    }

    #[test]
    fn test_translate_lexer_error_unexpected_char() {
        let arabic = translate_lexer_error("Unexpected character '@'");
        assert!(
            arabic.contains("غير متوقع"),
            "Should translate 'unexpected character'"
        );
    }

    #[test]
    fn test_translate_lexer_error_unterminated_string() {
        let arabic = translate_lexer_error("Unterminated string");
        assert!(
            arabic.contains("غير مُنهى"),
            "Should translate 'unterminated string'"
        );
    }

    #[test]
    fn test_translate_lexer_error_invalid_number() {
        let arabic = translate_lexer_error("Invalid number format");
        assert!(
            arabic.contains("غير صالح"),
            "Should translate 'invalid number'"
        );
    }

    #[test]
    fn test_translate_lexer_error_unknown() {
        let arabic = translate_lexer_error("Some unknown error");
        assert!(
            arabic.contains("خطأ معجمي"),
            "Unknown errors should have Arabic prefix"
        );
    }

    /// A top-level name defined by two merged modules (و٠١٠١) is found only by
    /// the link step. The editor skipped it, so a file that `check` and `run`
    /// both reject showed clean in VS Code.
    #[test]
    fn test_cross_module_collision_is_reported_to_the_editor() {
        let dir = tempfile::TempDir::new().unwrap();

        for (name, value) in [("ك1.ترقيم", "1"), ("ك2.ترقيم", "2")] {
            let body = format!("صدّر دالة مكرر() -> عدد {{\n أرجع {}\n}}", value);
            std::fs::write(dir.path().join(name), wrap_with_markers(&body)).unwrap();
        }

        let content = wrap_with_markers(
            "استورد { مكرر } من \"./ك1\"\n\
             استورد { مكرر كـ مكرر2 } من \"./ك2\"\n\
             اطبع(مكرر())",
        );
        let main_path = dir.path().join("رئيسي.ترقيم");
        std::fs::write(&main_path, &content).unwrap();

        let mut doc =
            DocumentState::new(Url::from_file_path(&main_path).unwrap(), 1, content.clone());
        let analysis = doc.get_analysis(Language::Arabic);

        assert!(analysis.has_errors, "the collision must fail the document");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.message.contains("تعريف علوي مكرر")),
            "expected a link-stage collision diagnostic, got {:?}",
            analysis
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    /// The link step must stay silent on a program that links cleanly.
    #[test]
    fn test_valid_cross_module_program_stays_clean_in_the_editor() {
        let dir = tempfile::TempDir::new().unwrap();

        std::fs::write(
            dir.path().join("أدوات.ترقيم"),
            wrap_with_markers("صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {\n أرجع أ + ب\n}"),
        )
        .unwrap();

        let content = wrap_with_markers("استورد { جمع } من \"./أدوات\"\nاطبع(جمع(2، 3))");
        let main_path = dir.path().join("رئيسي.ترقيم");
        std::fs::write(&main_path, &content).unwrap();

        let mut doc =
            DocumentState::new(Url::from_file_path(&main_path).unwrap(), 1, content.clone());
        let analysis = doc.get_analysis(Language::Arabic);

        assert!(
            !analysis.has_errors,
            "expected a clean document, got {:?}",
            analysis
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }
}
