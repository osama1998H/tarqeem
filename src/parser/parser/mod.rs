//! Recursive descent parser for Tarqeem
//!
//! This module provides the Parser that converts a stream of tokens into
//! an Abstract Syntax Tree (AST).
//!
//! # Module Organization
//!
//! The parser is split into submodules for maintainability:
//! - `decl_parser`: Declaration parsing (var, function, class, etc.)
//! - `stmt_parser`: Statement parsing (if, while, for, match, etc.)
//! - `expr_parser`: Expression parsing (Pratt parsing for precedence)

mod decl_parser;
mod expr_parser;
mod stmt_parser;

use super::ast::*;
use crate::error::codes::{ERR_EXPECTED_SEMICOLON, ERR_UNEXPECTED_TOKEN};
use crate::error::{Diagnostic, Span};
use crate::lexer::{Lexer, Token, TokenKind};

/// The Parser converts a token stream into an AST.
pub struct Parser {
    tokens: Vec<Token>,
    pub(crate) current: usize,
    errors: Vec<Diagnostic>,
    panic_mode: bool,
    /// Line comments pending to be attached to the next statement
    pub(crate) pending_comments: Vec<String>,
}

/// Contextual keywords: احصل/عيّن/حالة are reserved only inside خاصية accessor
/// blocks and تطابق arms; elsewhere they act as ordinary identifiers
/// (same pattern as expect_type_name's type-keyword mapping).
/// عين (no shadda) normalizes to عيّن, matching the lexer's keyword aliasing.
pub(crate) fn identifier_like_name(kind: &TokenKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        TokenKind::Get => Some("احصل".to_string()),
        TokenKind::Set => Some("عيّن".to_string()),
        TokenKind::Case => Some("حالة".to_string()),
        _ => None,
    }
}

impl Parser {
    /// Create a new parser from source code.
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer.tokenize();

        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
            pending_comments: Vec::new(),
        }
    }

    /// Create a new parser from a pre-tokenized token stream.
    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
            pending_comments: Vec::new(),
        }
    }

    /// Skips all newline tokens.
    pub(crate) fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    /// Synchronize after an error to continue parsing.
    pub(crate) fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }

            match self.peek().kind {
                TokenKind::Let        // متغير
                | TokenKind::Const    // ثابت
                | TokenKind::Function // دالة
                | TokenKind::Class    // صنف
                | TokenKind::Interface // ميثاق
                | TokenKind::Enum     // تعداد
                | TokenKind::If       // إذا
                | TokenKind::While    // طالما
                | TokenKind::For      // لكل
                | TokenKind::Do       // افعل
                | TokenKind::Return   // أرجع
                | TokenKind::Try      // حاول
                | TokenKind::Match    // تطابق
                | TokenKind::Import   // استورد
                | TokenKind::Export   // صدّر
                | TokenKind::Alhamdulillah => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    /// Synchronize to the next class member.
    pub(crate) fn synchronize_to_member(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            match self.peek().kind {
                TokenKind::Public     // عام
                | TokenKind::Private  // خاص
                | TokenKind::Protected // محمي
                | TokenKind::Static   // مشترك
                | TokenKind::Function // دالة
                | TokenKind::Async    // غير_متزامن
                | TokenKind::Constructor // منشئ
                | TokenKind::RightBrace => {
                    return;
                }
                TokenKind::Identifier(_) => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    /// Synchronize to the next match arm.
    pub(crate) fn synchronize_to_arm(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            match self.peek().kind {
                TokenKind::Case | TokenKind::Default | TokenKind::RightBrace => return,
                _ => {}
            }
            self.advance();
        }
    }

    /// Report an error.
    pub(crate) fn report_error(&mut self, diagnostic: Diagnostic) {
        if !self.panic_mode {
            self.errors.push(diagnostic);
        }
        self.panic_mode = true;
    }

    /// Get all errors collected during parsing.
    pub fn get_errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    /// Consume a doc comment if present.
    pub(crate) fn consume_doc_comment(&mut self) -> Option<String> {
        if let TokenKind::DocComment(comment) = &self.peek().kind {
            let comment = comment.clone();
            self.advance();
            // Skip any newlines after doc comment
            self.skip_newlines();
            Some(comment)
        } else {
            None
        }
    }

    /// Collect any line comments before a statement.
    pub(crate) fn collect_line_comments(&mut self) {
        while let TokenKind::LineComment(comment) = &self.peek().kind {
            self.pending_comments.push(comment.clone());
            self.advance();
            // Skip newlines between comments
            self.skip_newlines();
        }
    }

    /// Take the pending comments and clear them.
    pub(crate) fn take_pending_comments(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_comments)
    }

    /// Capture a trailing comment on the same line.
    pub(crate) fn capture_trailing_comment(&mut self) -> Option<String> {
        if let TokenKind::LineComment(comment) = &self.peek().kind {
            let comment = comment.clone();
            self.advance();
            Some(comment)
        } else {
            None
        }
    }

    /// Parse the source into an AST.
    pub fn parse(&mut self) -> Result<Ast, Diagnostic> {
        self.skip_newlines();

        // Check for the start marker (بسم_الله)
        if !self.check(&TokenKind::Bismillah) {
            return Err(Diagnostic::error(
                "متوقع 'بسم_الله' في بداية الملف",
                self.current_span(),
            ));
        }

        // Capture the span of بسم_الله
        let bismillah_span = self.current_span();
        self.advance(); // Consume بسم_الله

        // Skip newlines after بسم_الله
        self.skip_newlines();

        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Check for end marker
            if self.check(&TokenKind::Alhamdulillah) {
                break;
            }

            match self.parse_declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize();
                }
            }

            // Skip newlines after each statement
            self.skip_newlines();
        }

        // Check for the end marker (الحمد_لله)
        if !self.check(&TokenKind::Alhamdulillah) {
            return Err(Diagnostic::error(
                "متوقع 'الحمد_لله' في نهاية الملف",
                self.current_span(),
            ));
        }

        // Capture the span of الحمد_لله
        let alhamdulillah_span = self.current_span();
        self.advance(); // Consume الحمد_لله

        // Skip newlines after الحمد_لله
        self.skip_newlines();

        // Check if there's any code after الحمد_لله
        if !self.is_at_end() {
            return Err(Diagnostic::error(
                "لا يُسمح بأي كود بعد علامة 'الحمد_لله'",
                self.current_span(),
            ));
        }

        // Return errors if there were any during parsing
        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }

        Ok(Ast::with_markers(
            statements,
            bismillah_span,
            alhamdulillah_span,
        ))
    }

    // Token helper methods

    /// Check if at end of tokens.
    pub(crate) fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    /// Peek at the current token.
    pub(crate) fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// Look ahead one token (for lookahead parsing).
    pub(crate) fn peek_next(&self) -> Option<&Token> {
        if self.current + 1 < self.tokens.len() {
            Some(&self.tokens[self.current + 1])
        } else {
            None
        }
    }

    /// Get the previous token.
    pub(crate) fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    /// Advance to the next token.
    pub(crate) fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous().clone()
    }

    /// Check if current token matches a kind.
    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    /// Check if current token is an identifier (including contextual keywords).
    pub(crate) fn check_identifier(&self) -> bool {
        identifier_like_name(&self.peek().kind).is_some()
    }

    /// Match and consume a token if it matches.
    pub(crate) fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expect a specific token kind or error.
    pub(crate) fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<Token, Diagnostic> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(Diagnostic::error(message, self.current_span())
                .with_code(ERR_UNEXPECTED_TOKEN.to_string()))
        }
    }

    /// Expect an identifier (including contextual keywords) or error.
    pub(crate) fn expect_identifier(&mut self, message: &str) -> Result<String, Diagnostic> {
        if let Some(name) = identifier_like_name(&self.peek().kind) {
            self.advance();
            Ok(name)
        } else {
            Err(Diagnostic::error(message, self.current_span()))
        }
    }

    /// Expect a type name (identifier, contextual keyword, or type keyword).
    pub(crate) fn expect_type_name(&mut self) -> Result<String, Diagnostic> {
        let token = self.peek().clone();
        if let Some(name) = identifier_like_name(&token.kind) {
            self.advance();
            return Ok(name);
        }
        match &token.kind {
            TokenKind::TypeInt => {
                self.advance();
                Ok("عدد".to_string())
            }
            TokenKind::TypeFloat => {
                self.advance();
                Ok("عدد_عشري".to_string())
            }
            TokenKind::TypeString => {
                self.advance();
                Ok("نص".to_string())
            }
            TokenKind::TypeBool => {
                self.advance();
                Ok("منطقي".to_string())
            }
            TokenKind::TypeArray => {
                self.advance();
                Ok("مصفوفة".to_string())
            }
            TokenKind::TypeMap => {
                self.advance();
                Ok("قاموس".to_string())
            }
            TokenKind::TypeVoid => {
                self.advance();
                Ok("void".to_string()) // Internal name only
            }
            TokenKind::TypeAny => {
                self.advance();
                Ok("أي".to_string())
            }
            _ => Err(Diagnostic::error("متوقع اسم النوع", self.current_span())
                .with_code(ERR_UNEXPECTED_TOKEN.to_string())),
        }
    }

    /// Expect a string literal or error.
    pub(crate) fn expect_string(&mut self, message: &str) -> Result<String, Diagnostic> {
        if let TokenKind::StringLiteral(s) = &self.peek().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(Diagnostic::error(message, self.current_span())
                .with_code(ERR_UNEXPECTED_TOKEN.to_string()))
        }
    }

    /// Consume a semicolon (or accept implicit semicolon).
    pub(crate) fn consume_semicolon(&mut self) -> Result<(), Diagnostic> {
        if self.match_token(&TokenKind::Semicolon) || self.match_token(&TokenKind::ArabicSemicolon)
        {
            Ok(())
        } else {
            if self.check(&TokenKind::RightBrace) || self.is_at_end() {
                return Ok(());
            }

            // Allow omission if current token is a line comment (trailing comment)
            // or a newline (next statement is on next line)
            if matches!(
                self.peek().kind,
                TokenKind::LineComment(_) | TokenKind::Newline
            ) {
                return Ok(());
            }

            let prev_line = self.previous_span().line;
            let curr_line = self.current_span().line;
            if curr_line > prev_line {
                return Ok(());
            }

            Err(Diagnostic::error("متوقع '؛'", self.current_span())
                .with_code(ERR_EXPECTED_SEMICOLON.to_string()))
        }
    }

    /// Get the span of the current token.
    pub(crate) fn current_span(&self) -> Span {
        self.peek().span
    }

    /// Get the span of the previous token.
    pub(crate) fn previous_span(&self) -> Span {
        self.previous().span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable_declaration() {
        let mut parser = Parser::new("بسم_الله\nمتغير س = 5;\nالحمد_لله");
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl {
                name,
                mutable,
                init,
                ..
            } => {
                assert_eq!(name, "س");
                assert!(*mutable);
                assert!(init.is_some());
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parse_function_declaration() {
        let source = r#"
            بسم_الله
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::FuncDecl { name, params, .. } => {
                assert_eq!(name, "جمع");
                assert_eq!(params.len(), 2);
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_if_statement() {
        let source = r#"
            بسم_الله
            إذا (س > 5) {
                اطبع("كبير");
            } وإلا {
                اطبع("صغير");
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::If { else_branch, .. } => {
                assert!(else_branch.is_some());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_parse_class_declaration() {
        let source = r#"
            بسم_الله
            صنف شخص {
                خاص اسم: نص;

                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }

                عام دالة احصل_اسم() -> نص {
                    أرجع هذا.اسم;
                }
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::ClassDecl { name, members, .. } => {
                assert_eq!(name, "شخص");
                assert_eq!(members.len(), 3); // field, constructor, method
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_expressions() {
        let source = "بسم_الله\n1 + 2 * 3;\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_parse_array_literal() {
        let source = "بسم_الله\n[1، 2، 3];\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        match &ast.statements[0].kind {
            StmtKind::Expr(expr) => match &expr.kind {
                ExprKind::Array(elements) => {
                    assert_eq!(elements.len(), 3);
                }
                _ => panic!("Expected Array"),
            },
            _ => panic!("Expected Expr"),
        }
    }

    #[test]
    fn test_missing_file_start_marker() {
        let source = "متغير س = 5;\nالحمد_لله";
        let mut parser = Parser::new(source);
        let result = parser.parse();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("بسم_الله") || err.message.contains("bismillah"));
    }

    #[test]
    fn test_missing_file_end_marker() {
        let source = "بسم_الله\nمتغير س = 5;";
        let mut parser = Parser::new(source);
        let result = parser.parse();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("الحمد_لله") || err.message.contains("alhamdulillah"));
    }
}
