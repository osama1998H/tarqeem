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
    /// How many `(`/`[` are open around the expression being parsed. Inside one,
    /// a newline is trivia rather than a statement terminator (issue #255).
    pub(crate) bracket_depth: usize,
}

/// Runs `body` with the bracket-nesting depth raised, so newlines inside the
/// brackets are trivia. Restores the depth even when `body` returns an error, or
/// a malformed argument list would leave every following statement joined to the
/// next line.
pub(crate) fn within_brackets<T>(parser: &mut Parser, body: impl FnOnce(&mut Parser) -> T) -> T {
    parser.bracket_depth += 1;
    let result = body(parser);
    parser.bracket_depth -= 1;
    result
}

/// One run of comments and blank lines in front of a declaration.
#[derive(Default)]
pub(crate) struct LeadingTrivia {
    /// Every comment in the run except `doc`, in source order. A doc block that
    /// is not the last in the run lands here and is rendered `//`.
    pub(crate) comments: Vec<String>,
    /// The doc block written closest to the declaration — the one documenting it.
    pub(crate) doc: Option<String>,
    /// Index in `comments` where `doc` was written. A demoted doc is re-inserted
    /// there rather than appended, so `fmt` cannot reorder a run it did not
    /// write: appending would move a demoted `/// وثيقة` below a `//` that came
    /// after it in the source.
    doc_position: usize,
}

impl LeadingTrivia {
    /// A later doc block is nearer the declaration, so it takes over as its
    /// documentation and the previous one is demoted back into its own slot.
    fn push_doc(&mut self, doc: String) {
        if let Some(previous) = self.doc.replace(doc) {
            self.comments.insert(self.doc_position, previous);
        }
        self.doc_position = self.comments.len();
    }

    /// Gives up on attaching `doc` and puts it back where it was written. Used
    /// when what follows the run owns no `doc_comment` field, so the text would
    /// otherwise be dropped — and erased from the user's file by `fmt -w`.
    pub(crate) fn demote_doc(&mut self) {
        if let Some(doc) = self.doc.take() {
            self.comments.insert(self.doc_position, doc);
        }
    }
}

/// Contextual keywords: احصل/عيّن/حالة are reserved only inside خاصية accessor
/// blocks and تطابق arms; elsewhere they act as ordinary identifiers
/// (same pattern as expect_type_name's type-keyword mapping). Returns the
/// token's own text so the spelling the user wrote is preserved (عين and عيّن
/// are distinct identifiers, as they would be if they weren't keywords) — the
/// lexer NFC-normalizes the whole source up front, so the lexeme is already
/// normalized exactly like an Identifier's name.
pub(crate) fn identifier_like_name(token: &Token) -> Option<&str> {
    match &token.kind {
        TokenKind::Identifier(name) => Some(name),
        TokenKind::Get | TokenKind::Set | TokenKind::Case => Some(&token.lexeme),
        _ => None,
    }
}

/// A name the user chose, in a position where the grammar can hold nothing else:
/// after `دالة`, before a `:` in a member or parameter list, after `.`. Type
/// names are keywords here (`عدد` is `TokenKind::TypeInt`), so a method or
/// parameter named after a type was rejected outright — issue #202, which the
/// shipped stdlib hits with `دالة عدد()` and `دالة اطبع(نص: نص)`.
///
/// Deliberately *not* folded into `identifier_like_name`, even though that is
/// where the احصل/عيّن/حالة precedent lives: that helper also backs
/// `check_identifier`, which is the name-or-type disambiguator for enum-variant
/// payloads and `looks_like_type`. Widening it there would make `مصفوفة<عدد>`
/// parse as a field named `مصفوفة` and strand the `<عدد>`.
///
/// Returns the lexeme rather than a canonical spelling, so `اي` stays `اي` — the
/// #183 rule that `tarqeem fmt` must never rename a user's identifier. The lexer
/// NFC-normalizes the whole source, so the lexeme is already normalized.
pub(crate) fn declaration_name(token: &Token) -> Option<&str> {
    identifier_like_name(token).or_else(|| {
        token
            .kind
            .is_type_keyword()
            .then_some(token.lexeme.as_str())
    })
}

/// A name in enum-variant position, which additionally allows the boolean
/// literals: `خطأ` is `TokenKind::False`, and `stdlib/اختبار/نتائج.ترقيم`
/// declares a variant with exactly that name (issue #196).
///
/// Unambiguous because a variant is only ever *referenced* through `::`. A bare
/// `خطأ` in an expression or a match pattern still resolves to the literal, so
/// nothing outside variant position changes. Class names are pointedly excluded:
/// `جديد خطأ()` parses its callee as a primary expression, which would yield
/// `false` rather than a type name — which is why the stdlib's `صنف خطأ` has to
/// be renamed (#243) instead.
pub(crate) fn variant_name(token: &Token) -> Option<&str> {
    declaration_name(token).or_else(|| {
        matches!(token.kind, TokenKind::True | TokenKind::False).then_some(token.lexeme.as_str())
    })
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
            bracket_depth: 0,
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
            bracket_depth: 0,
        }
    }

    /// Skips all newline tokens.
    pub(crate) fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    /// Non-consuming lookahead: true if, after skipping any run of
    /// `Newline` tokens and comment tokens (line, doc, or block-doc), the
    /// next real token has the same kind as `terminator`. Stays read-only
    /// so a caller that gets `false` can still route a genuine leading
    /// comment to `collect_line_comments`/`consume_doc_comment` instead of
    /// having it stolen by a terminator check that guessed wrong.
    pub(crate) fn check_terminator_after_trivia(&self, terminator: &TokenKind) -> bool {
        let mut idx = self.current;
        while idx < self.tokens.len() {
            match &self.tokens[idx].kind {
                TokenKind::Newline => idx += 1,
                kind if kind.is_comment() => idx += 1,
                _ => break,
            }
        }
        idx < self.tokens.len()
            && std::mem::discriminant(&self.tokens[idx].kind) == std::mem::discriminant(terminator)
    }

    /// Advances past a run of `Newline` and comment tokens, discarding their text.
    fn skip_trivia_run(&mut self) {
        while self.check(&TokenKind::Newline) || self.peek().kind.is_comment() {
            self.advance();
        }
    }

    /// Consumes the trivia preceding `terminator` when
    /// `check_terminator_after_trivia` confirms one is there, leaving the
    /// terminator itself unconsumed; otherwise consumes nothing. Lets a
    /// statement-list loop break on a terminator that follows trailing
    /// comments or blank lines without misattaching those comments to the
    /// next declaration.
    pub(crate) fn match_terminator_after_trivia(&mut self, terminator: &TokenKind) -> bool {
        if !self.check_terminator_after_trivia(terminator) {
            return false;
        }
        self.skip_trivia_run();
        // A statement list that reached its terminator has nothing left to attach
        // a pending comment to — anything still here escaped a loop that collected
        // it (parse_class_member, the accessor/interface/enum loops) and would
        // otherwise be stolen by the next parse_declaration().
        self.pending_comments.clear();
        true
    }

    /// Like `match_terminator_after_trivia`, but hands back the comments it
    /// consumed instead of discarding them. Used only by `parse_block`, which
    /// has somewhere to put them (`Block::dangling_comments`); every other
    /// statement-list loop terminates a node with no field for a comment and
    /// keeps using the discarding sibling above.
    pub(crate) fn take_terminator_trivia_comments(
        &mut self,
        terminator: &TokenKind,
    ) -> Option<Vec<String>> {
        if !self.check_terminator_after_trivia(terminator) {
            return None;
        }
        let mut comments = Vec::new();
        while self.check(&TokenKind::Newline) || self.peek().kind.is_comment() {
            if let TokenKind::LineComment(c)
            | TokenKind::DocComment(c)
            | TokenKind::BlockDocComment(c) = &self.peek().kind
            {
                comments.push(c.clone());
            }
            self.advance();
        }
        self.pending_comments.clear();
        Some(comments)
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
                | TokenKind::RightBrace
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
            // Field/method names: the contextual keywords احصل/عيّن/حالة and, since
            // a member may be named after a type (#202), the type keywords too —
            // recovery has to resume at `عدد: نص` or it skips the whole member.
            if self.check_declaration_name() {
                return;
            }

            match self.peek().kind {
                TokenKind::Public     // عام
                | TokenKind::Private  // خاص
                | TokenKind::Protected // محمي
                | TokenKind::Static   // مشترك
                | TokenKind::Function // دالة
                | TokenKind::Async    // غير_متزامن
                | TokenKind::Constructor // منشئ
                | TokenKind::Property // خاصية
                | TokenKind::RightBrace => {
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
                // حالة mid-line is an identifier use, not the next arm head.
                // Resume only when it plausibly starts an arm: at a line start,
                // right after the match block's '{', or after a comment
                // (comment tokens swallow their trailing newline).
                TokenKind::Case => {
                    let starts_arm = self.current == 0 || {
                        let prev = &self.previous().kind;
                        matches!(prev, TokenKind::Newline | TokenKind::LeftBrace)
                            || prev.is_comment()
                    };
                    if starts_arm {
                        return;
                    }
                }
                TokenKind::Default | TokenKind::RightBrace => return,
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
    ///
    /// Accepts both `///` and `/** */`. Before this accepted `BlockDocComment`,
    /// a `/** */` preceding a declaration was left unconsumed and fell through
    /// to the expression parser as a hard error (`رمز غير متوقع:
    /// BlockDocComment(..)`) — see issue #201. It is only safe to attach now
    /// that the formatter re-prefixes `///` on every doc line; attaching it
    /// while the formatter still stripped markers would have turned that loud
    /// error into silent corruption.
    ///
    /// A `/** */` is only taken when it *starts its own line*. Documentation
    /// describes what follows it, so a block comment trailing code on the same
    /// line (`خاص اسم: نص /** ملاحظة */`) annotates that line, and taking it
    /// here would silently re-attach it to the *next* member — `tarqeem doc`
    /// would then publish the note under the wrong name, and `fmt -w` would
    /// rewrite the file that way. Leaving it unconsumed preserves the loud
    /// error it produced before #201. `///` deliberately keeps its old
    /// behaviour: the lexer already refuses to merge a trailing `///` forward
    /// (`is_line_start`, lexer.rs), and the parser's acceptance of it here
    /// predates this change.
    pub(crate) fn consume_doc_comment(&mut self) -> Option<String> {
        let comment = match &self.peek().kind {
            TokenKind::DocComment(comment) => comment.clone(),
            TokenKind::BlockDocComment(comment) if self.at_line_start() => comment.clone(),
            _ => return None,
        };
        self.advance();
        // Skip any newlines after doc comment
        self.skip_newlines();
        Some(comment)
    }

    /// True when nothing but a line break or an opening brace precedes the
    /// current token, i.e. it is the first thing on its line.
    fn at_line_start(&self) -> bool {
        self.current == 0
            || matches!(
                self.previous().kind,
                TokenKind::Newline | TokenKind::LeftBrace
            )
    }

    /// True when the doc comment at the current position documents the *file*
    /// rather than whatever follows it — the only place a `///` can be kept with
    /// its marker when nothing below it can hold a doc comment.
    ///
    /// It is the file's doc when any of these holds:
    ///
    /// 1. **A nearer doc block follows.** That one documents the declaration, so
    ///    this one cannot. Covers the 22 stdlib files whose header is followed by
    ///    a `//` banner and then the real doc.
    /// 2. **Nothing follows** (`الحمد_لله`/`Eof`). Required, not theoretical:
    ///    without it a header with no declaration after it survives one `fmt`
    ///    pass and is discarded by the *second*, because
    ///    `match_terminator_after_trivia` drops trailing trivia — silent loss
    ///    that only appears on a second run.
    /// 3. **What follows owns no `doc_comment` field** — `استورد`, a bare
    ///    `صدّر *`/`صدّر { … }`, or anything routed through `parse_statement`.
    ///    There the doc would be demoted into `leading_comments` and re-emitted
    ///    as `//`. `stdlib/اختبار/توكيدات.ترقيم` is exactly this: header,
    ///    `//` note, `استورد` — its seven `///` lines depend on this clause.
    ///
    /// Otherwise the declaration below owns the doc and keeps it, which is what
    /// the 20 corpus files whose header sits directly above a declaration have
    /// always done.
    fn doc_comment_is_module_header(&self) -> bool {
        match &self.peek().kind {
            TokenKind::DocComment(_) => {}
            TokenKind::BlockDocComment(_) if self.at_line_start() => {}
            _ => return false,
        }

        let mut idx = self.current + 1;
        let mut nearer_doc = false;
        while idx < self.tokens.len() {
            match &self.tokens[idx].kind {
                TokenKind::Newline | TokenKind::LineComment(_) => idx += 1,
                TokenKind::DocComment(_) | TokenKind::BlockDocComment(_) => {
                    nearer_doc = true;
                    idx += 1;
                }
                _ => break,
            }
        }

        if idx >= self.tokens.len() || nearer_doc {
            return true;
        }

        match self.tokens[idx].kind {
            TokenKind::Alhamdulillah | TokenKind::Eof => true,
            // Every declaration that carries a `doc_comment` field.
            TokenKind::Let
            | TokenKind::Const
            | TokenKind::Function
            | TokenKind::Async
            | TokenKind::Class
            | TokenKind::Interface
            | TokenKind::Enum => false,
            // `صدّر <decl>` threads the doc into the inner declaration; a
            // wildcard or named export list has nowhere to put it.
            // A declaration that carries no documentation of its own: a doc
            // above it can only be describing the file. Hoisting is what keeps
            // the text at all — `استورد` demotes it to `//`, and a re-export
            // drops it outright (both were silent losses before this).
            TokenKind::Import => true,
            TokenKind::Export => self.export_has_no_doc_field(idx),
            // Executable code in script mode. A doc directly above it reads as
            // documenting that statement, so it keeps demoting into a leading
            // comment as before; once a comment run separates the two there is
            // nothing left for it to be adjacent to.
            _ => self.tokens[self.current + 1..idx]
                .iter()
                .any(|token| token.kind.is_comment()),
        }
    }

    /// True when the `صدّر` at the current position is a re-export rather than
    /// an exported declaration, so it can carry no documentation.
    pub(crate) fn export_is_reexport(&self) -> bool {
        self.export_has_no_doc_field(self.current)
    }

    /// True when the `صدّر` at `idx` is a re-export rather than an exported
    /// declaration, i.e. `صدّر *` or `صدّر { … }`.
    fn export_has_no_doc_field(&self, idx: usize) -> bool {
        let mut next = idx + 1;
        while next < self.tokens.len() {
            match &self.tokens[next].kind {
                TokenKind::Newline => next += 1,
                kind if kind.is_comment() => next += 1,
                _ => break,
            }
        }

        next < self.tokens.len()
            && matches!(
                self.tokens[next].kind,
                TokenKind::Star | TokenKind::LeftBrace
            )
    }

    /// Consumes the *whole* run of blank lines and comments in front of a
    /// declaration.
    ///
    /// Replaces a `collect_line_comments()` + `consume_doc_comment()` pair, each
    /// of which ran exactly once: a `///` block stopped the line-comment loop
    /// before it began, so any comment written *after* that block was left in
    /// the stream and fell through the declaration dispatch into
    /// `parse_statement`, where it became `ب٠٠٠١ رمز غير متوقع` (issue #203).
    pub(crate) fn collect_leading_trivia(&mut self) -> LeadingTrivia {
        let mut trivia = LeadingTrivia::default();

        loop {
            self.skip_newlines();

            // Every arm but the last consumes exactly one token, so `current`
            // strictly increases until the loop exits.
            match &self.peek().kind {
                TokenKind::LineComment(comment) => {
                    let comment = comment.clone();
                    self.advance();
                    trivia.comments.push(comment);
                }
                TokenKind::DocComment(comment) => {
                    let comment = comment.clone();
                    self.advance();
                    trivia.push_doc(comment);
                }
                // A `/** */` trailing code on the same line annotates that line,
                // so it is left for capture_trailing_comment — the #201 rule
                // that consume_doc_comment encodes with the same guard.
                TokenKind::BlockDocComment(comment) if self.at_line_start() => {
                    let comment = comment.clone();
                    self.advance();
                    trivia.push_doc(comment);
                }
                _ => break,
            }
        }

        trivia
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
        match &self.peek().kind {
            TokenKind::LineComment(comment)
            | TokenKind::DocComment(comment)
            | TokenKind::BlockDocComment(comment) => {
                let comment = comment.clone();
                self.advance();
                Some(comment)
            }
            _ => None,
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

        // Peel a file-level doc comment before the declaration loop can claim
        // it: 42 of the 43 stdlib files open with one, and it documents the
        // module, not whatever declaration happens to follow.
        let module_doc = if self.doc_comment_is_module_header() {
            self.consume_doc_comment()
        } else {
            None
        };

        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Stop at the end marker, even when it is preceded by trailing
            // comments or blank lines — those belong to nothing else in
            // the file, so they must not be handed to parse_declaration.
            if self.match_terminator_after_trivia(&TokenKind::Alhamdulillah) {
                break;
            }

            let before = self.current;
            match self.parse_declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize();
                }
            }

            // synchronize() may land on a token it does not consume (e.g.
            // RightBrace); force forward progress so a future non-consuming
            // error path can never loop forever.
            if self.current == before && !self.is_at_end() {
                self.advance();
            }

            // Skip newlines after each statement
            self.skip_newlines();
        }

        // A real error found during recovery must win over the generic
        // end-marker diagnostic below, which fires whenever synchronize()
        // overshot to EOF and (unlike most parser errors) carries no error
        // code.
        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
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

        let mut ast = Ast::with_markers(statements, bismillah_span, alhamdulillah_span);
        ast.module_doc = module_doc;
        Ok(ast)
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
        identifier_like_name(self.peek()).is_some()
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
        if let Some(name) = identifier_like_name(self.peek()).map(str::to_string) {
            self.advance();
            Ok(name)
        } else {
            Err(Diagnostic::error(message, self.current_span()))
        }
    }

    /// True if the current token can name a declaration, member or parameter.
    pub(crate) fn check_declaration_name(&self) -> bool {
        declaration_name(self.peek()).is_some()
    }

    /// Expect a declaration/member/parameter name, which may be a type keyword.
    pub(crate) fn expect_declaration_name(&mut self, message: &str) -> Result<String, Diagnostic> {
        if let Some(name) = declaration_name(self.peek()).map(str::to_string) {
            self.advance();
            Ok(name)
        } else {
            Err(Diagnostic::error(message, self.current_span()))
        }
    }

    /// Expect an enum-variant name, which may also be a boolean literal keyword.
    pub(crate) fn expect_variant_name(&mut self, message: &str) -> Result<String, Diagnostic> {
        if let Some(name) = variant_name(self.peek()).map(str::to_string) {
            self.advance();
            Ok(name)
        } else {
            Err(Diagnostic::error(message, self.current_span()))
        }
    }

    /// Expect a type name (identifier, contextual keyword, or type keyword).
    pub(crate) fn expect_type_name(&mut self) -> Result<String, Diagnostic> {
        let token = self.peek().clone();
        if let Some(name) = identifier_like_name(&token) {
            let name = name.to_string();
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

            // Allow omission if current token is any comment (trailing comment)
            // or a newline (next statement is on next line)
            if self.peek().kind.is_comment() || self.check(&TokenKind::Newline) {
                return Ok(());
            }

            // Needed for /* */ comments, which produce no token — only the
            // line-number bump reveals a line break occurred.
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
