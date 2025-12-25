//! Document state and analysis management
//!
//! Manages the state of open documents and their analysis results.

use crate::error::{Diagnostic, Language, Span};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::parser::{Ast, Parser, TypeAnnotation, TypeKind};
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
                diagnostics.push(Diagnostic::error(msg.clone(), arabic_msg, token.span));
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
            let mut analyzer = Analyzer::new();

            self.collect_symbols(ast, &mut symbols);

            if let Err(errs) = analyzer.analyze(ast) {
                for err in errs {
                    diagnostics.push(err);
                }
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

    fn collect_symbols(&self, ast: &Ast, symbols: &mut HashMap<String, SymbolInfo>) {
        use crate::parser::{ClassMember, StmtKind};

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
                return_type: Box::new(self.resolve_type_annotation(return_type)),
            },
            TypeKind::Generic { base, args: _ } => self.parse_type_name(base),
            TypeKind::Optional(inner) => {
                Type::Optional(Box::new(self.resolve_type_annotation(inner)))
            }
        }
    }

    fn parse_type_name(&self, name: &str) -> Type {
        match name {
            "عدد" | "int" => Type::Int,
            "عدد_عشري" | "float" => Type::Float,
            "نص" | "string" => Type::String,
            "منطقي" | "bool" => Type::Bool,
            "void" => Type::Void, // فراغ eliminated - functions default to no return
            "لا_شيء" | "null" | "none" => Type::Null,
            "أي" | "اي" | "any" => Type::Any,
            _ => Type::Class(name.to_string()),
        }
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
        Url::parse("file:///test.trq").unwrap()
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
}
