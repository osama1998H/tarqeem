//! Main lexer implementation
//!
//! Tarqeem is an Arabic-only programming language. English identifiers and keywords
//! are not supported.

use super::keywords::lookup_keyword;
use super::token::{Token, TokenKind};
use crate::error::Span;
use crate::utils::StringInterner;
use unicode_normalization::UnicodeNormalization;

/// The Tarqeem lexer, responsible for tokenizing Arabic source code.
///
/// The lexer can optionally use a string interner to deduplicate identifier
/// strings, reducing memory usage and enabling faster string comparisons.
pub struct Lexer<'a> {
    source: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    token_start: usize,
    token_start_line: usize,
    token_start_column: usize,
    /// Optional string interner for identifier deduplication.
    interner: Option<&'a mut StringInterner>,
}

impl Lexer<'static> {
    /// Creates a new lexer without string interning.
    ///
    /// This is the backward-compatible constructor. For better memory efficiency,
    /// use [`Lexer::with_interner`] instead.
    pub fn new(source: &str) -> Self {
        let normalized: String = source.nfc().collect();
        Lexer {
            source: normalized.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            token_start: 0,
            token_start_line: 1,
            token_start_column: 1,
            interner: None,
        }
    }
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer with string interning support.
    ///
    /// All identifiers will be interned, reducing memory usage and enabling
    /// O(1) string comparisons in later compilation phases.
    ///
    /// # Example
    ///
    /// ```
    /// use tarqeem::{Lexer, StringInterner};
    ///
    /// let mut interner = StringInterner::new();
    /// let mut lexer = Lexer::with_interner("متغير س = 5", &mut interner);
    /// let tokens = lexer.tokenize();
    /// ```
    pub fn with_interner(source: &str, interner: &'a mut StringInterner) -> Self {
        let normalized: String = source.nfc().collect();
        Lexer {
            source: normalized.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            token_start: 0,
            token_start_line: 1,
            token_start_column: 1,
            interner: Some(interner),
        }
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(doc_token) = self.skip_whitespace_and_comments() {
            return doc_token;
        }

        self.token_start = self.position;
        self.token_start_line = self.line;
        self.token_start_column = self.column;

        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }

        let c = self.advance();

        if c == '\n' {
            return self.make_token(TokenKind::Newline);
        }

        if self.is_digit(c) {
            return self.scan_number(c);
        }

        if self.is_english_letter(c) {
            return self.scan_english_identifier_error(c);
        }

        if self.is_identifier_start(c) {
            return self.scan_identifier(c);
        }

        if c == '"' || c == '\'' || c == '«' {
            return self.scan_string(c);
        }

        self.scan_operator(c)
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.position]
        }
    }

    fn peek_next(&self) -> char {
        if self.position + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.position + 1]
        }
    }

    fn advance(&mut self) -> char {
        let c = self.source[self.position];
        self.position += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.position] != expected {
            false
        } else {
            self.position += 1;
            self.column += 1;
            true
        }
    }

    fn current_lexeme(&self) -> String {
        self.source[self.token_start..self.position]
            .iter()
            .collect()
    }

    fn make_token(&self, kind: TokenKind) -> Token {
        Token::new(
            kind,
            Span::new(
                self.token_start,
                self.position,
                self.token_start_line,
                self.token_start_column,
            ),
            self.current_lexeme(),
        )
    }

    fn make_error(&self, message: &str) -> Token {
        Token::new(
            TokenKind::Error(message.to_string()),
            Span::new(
                self.token_start,
                self.position,
                self.token_start_line,
                self.token_start_column,
            ),
            self.current_lexeme(),
        )
    }

    fn is_digit(&self, c: char) -> bool {
        c.is_ascii_digit() || self.is_arabic_digit(c)
    }

    fn is_arabic_digit(&self, c: char) -> bool {
        matches!(c, '٠'..='٩')
    }

    fn arabic_digit_value(&self, c: char) -> Option<u8> {
        match c {
            '٠' => Some(0),
            '١' => Some(1),
            '٢' => Some(2),
            '٣' => Some(3),
            '٤' => Some(4),
            '٥' => Some(5),
            '٦' => Some(6),
            '٧' => Some(7),
            '٨' => Some(8),
            '٩' => Some(9),
            _ => None,
        }
    }

    fn digit_value(&self, c: char) -> u8 {
        if let Some(v) = self.arabic_digit_value(c) {
            v
        } else {
            c.to_digit(10).unwrap_or(0) as u8
        }
    }

    fn is_identifier_start(&self, c: char) -> bool {
        c == '_' || self.is_arabic_letter(c)
    }

    fn is_identifier_continue(&self, c: char) -> bool {
        self.is_identifier_start(c)
            || c.is_ascii_digit()
            || self.is_arabic_digit(c)
            || self.is_arabic_diacritic(c)
            || c == '\u{0640}' // Arabic tatweel (kashida)
    }

    fn is_arabic_diacritic(&self, c: char) -> bool {
        matches!(c, '\u{064B}'..='\u{0652}')
    }

    fn is_english_letter(&self, c: char) -> bool {
        c.is_ascii_alphabetic()
    }

    fn is_arabic_letter(&self, c: char) -> bool {
        matches!(c,
            '\u{0621}'..='\u{063A}' |  // Arabic letters (alef through za)
            '\u{0641}'..='\u{064A}' |  // Arabic letters (fa through ya)
            '\u{066E}'..='\u{066F}' |  // Arabic letter dotless beh/qaf
            '\u{0671}'..='\u{06D3}' |  // Arabic letters extended
            '\u{06D5}'              |  // Arabic letter ae
            '\u{06E5}'..='\u{06E6}' |  // Arabic small waw/ya
            '\u{06EE}'..='\u{06EF}' |  // Arabic letters dal/ra with inverted v
            '\u{06FA}'..='\u{06FC}' |  // Arabic letters sheen/dad/ghain with dot below
            '\u{06FF}'              |  // Arabic letter heh with inverted v
            '\u{0750}'..='\u{077F}' |  // Arabic Supplement
            '\u{08A0}'..='\u{08FF}' |  // Arabic Extended-A
            '\u{FB50}'..='\u{FDFF}' |  // Arabic Presentation Forms-A
            '\u{FE70}'..='\u{FEFF}'    // Arabic Presentation Forms-B
        )
    }

    fn skip_whitespace_and_comments(&mut self) -> Option<Token> {
        loop {
            if self.is_at_end() {
                break;
            }

            let c = self.peek();
            match c {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '/' if self.peek_next() == '/' => {
                    if self.position + 2 < self.source.len()
                        && self.source[self.position + 2] == '/'
                    {
                        return Some(self.scan_doc_comment());
                    }
                    self.advance(); // consume first /
                    self.advance(); // consume second /
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek_next() == '*' => {
                    if self.position + 2 < self.source.len()
                        && self.source[self.position + 2] == '*'
                        && (self.position + 3 >= self.source.len()
                            || self.source[self.position + 3] != '*')
                    {
                        return Some(self.scan_block_doc_comment());
                    }
                    self.advance(); // consume /
                    self.advance(); // consume *
                    let mut depth = 1;
                    while !self.is_at_end() && depth > 0 {
                        if self.peek() == '*' && self.peek_next() == '/' {
                            self.advance();
                            self.advance();
                            depth -= 1;
                        } else if self.peek() == '/' && self.peek_next() == '*' {
                            self.advance();
                            self.advance();
                            depth += 1;
                        } else {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
        None
    }

    fn scan_doc_comment(&mut self) -> Token {
        self.token_start = self.position;
        self.token_start_line = self.line;
        self.token_start_column = self.column;

        self.advance(); // first /
        self.advance(); // second /
        self.advance(); // third /

        let mut content = String::new();

        loop {
            if self.peek() == ' ' {
                self.advance();
            }

            while !self.is_at_end() && self.peek() != '\n' {
                content.push(self.advance());
            }

            if self.is_at_end() {
                break;
            }

            let newline_pos = self.position;
            self.advance();

            while !self.is_at_end() && (self.peek() == ' ' || self.peek() == '\t') {
                self.advance();
            }

            if self.peek() == '/'
                && self.peek_next() == '/'
                && self.position + 2 < self.source.len()
                && self.source[self.position + 2] == '/'
            {
                content.push('\n');
                self.advance(); // first /
                self.advance(); // second /
                self.advance(); // third /
                continue;
            }

            self.position = newline_pos + 1;
            self.line = self.token_start_line + content.matches('\n').count() + 1;
            self.column = 1;
            break;
        }

        self.make_token(TokenKind::DocComment(content.trim_end().to_string()))
    }

    fn scan_block_doc_comment(&mut self) -> Token {
        self.token_start = self.position;
        self.token_start_line = self.line;
        self.token_start_column = self.column;

        self.advance(); // /
        self.advance(); // *
        self.advance(); // *

        let mut content = String::new();
        let mut depth = 1;

        while !self.is_at_end() && depth > 0 {
            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                depth -= 1;
            } else if self.peek() == '/' && self.peek_next() == '*' {
                content.push(self.advance());
                content.push(self.advance());
                depth += 1;
            } else {
                content.push(self.advance());
            }
        }

        let cleaned = content
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with('*') && !trimmed.starts_with("*/") {
                    trimmed[1..].trim_start()
                } else {
                    line.trim()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        self.make_token(TokenKind::BlockDocComment(cleaned))
    }

    fn scan_number(&mut self, first: char) -> Token {
        let mut value: i64 = self.digit_value(first) as i64;
        let mut overflowed = false;

        while !self.is_at_end() && self.is_digit(self.peek()) {
            let c = self.advance();
            let digit = self.digit_value(c) as i64;

            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => {
                    overflowed = true;
                }
            }
        }

        if self.peek() == '.' && self.is_digit(self.peek_next()) {
            self.advance(); // consume '.'

            let mut fraction = 0.0;
            let mut divisor = 10.0;

            while !self.is_at_end() && self.is_digit(self.peek()) {
                let c = self.advance();
                fraction += self.digit_value(c) as f64 / divisor;
                divisor *= 10.0;
            }

            return self.make_token(TokenKind::FloatLiteral(value as f64 + fraction));
        }

        if self.peek() == 'e' || self.peek() == 'E' {
            self.advance();

            let mut exp_sign = 1i32;
            if self.peek() == '+' {
                self.advance();
            } else if self.peek() == '-' {
                exp_sign = -1;
                self.advance();
            }

            let mut exp: i32 = 0;
            while !self.is_at_end() && self.is_digit(self.peek()) {
                let c = self.advance();
                exp = exp * 10 + self.digit_value(c) as i32;
            }

            let float_val = value as f64 * 10f64.powi(exp_sign * exp);
            return self.make_token(TokenKind::FloatLiteral(float_val));
        }

        if overflowed {
            return self.make_token(TokenKind::Error(
                "Integer literal too large / العدد الصحيح كبير جداً".to_string(),
            ));
        }

        self.make_token(TokenKind::IntLiteral(value))
    }

    fn scan_identifier(&mut self, first: char) -> Token {
        let mut ident = String::new();
        ident.push(first);

        while !self.is_at_end() && self.is_identifier_continue(self.peek()) {
            ident.push(self.advance());
        }

        if let Some(keyword) = lookup_keyword(&ident) {
            self.make_token(keyword)
        } else {
            // Intern the identifier if an interner is available
            if let Some(ref mut interner) = self.interner {
                // Intern to deduplicate, but still store the string in the token
                // for now (full Symbol-based AST would be a larger refactor)
                let _symbol = interner.intern(&ident);
            }
            self.make_token(TokenKind::Identifier(ident))
        }
    }

    fn scan_english_identifier_error(&mut self, first: char) -> Token {
        let mut ident = String::new();
        ident.push(first);

        while !self.is_at_end() {
            let c = self.peek();
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(self.advance());
            } else {
                break;
            }
        }

        self.make_error(&format!(
            "English identifiers are not allowed. Use Arabic instead / المعرفات الإنجليزية غير مسموح بها. استخدم العربية بدلاً من '{}'"
            , ident
        ))
    }

    fn scan_string(&mut self, quote: char) -> Token {
        let closing = match quote {
            '«' => '»',
            _ => quote,
        };

        let mut value = String::new();

        while !self.is_at_end() && self.peek() != closing {
            if self.peek() == '\n' {
                return self.make_error("Unterminated string / نص غير مكتمل");
            }

            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    return self.make_error("Unterminated escape / تسلسل هروب غير مكتمل");
                }
                let escaped = self.advance();
                value.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    '\'' => '\'',
                    '0' => '\0',
                    _ => escaped,
                });
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return self.make_error("Unterminated string / نص غير مكتمل");
        }

        self.advance(); // consume closing quote
        self.make_token(TokenKind::StringLiteral(value))
    }

    fn scan_operator(&mut self, c: char) -> Token {
        match c {
            '(' => self.make_token(TokenKind::LeftParen),
            ')' => self.make_token(TokenKind::RightParen),
            '{' => self.make_token(TokenKind::LeftBrace),
            '}' => self.make_token(TokenKind::RightBrace),
            '[' => self.make_token(TokenKind::LeftBracket),
            ']' => self.make_token(TokenKind::RightBracket),
            '.' => self.make_token(TokenKind::Dot),
            ':' => self.make_token(TokenKind::Colon),
            '?' => self.make_token(TokenKind::Question),
            ',' => self.make_token(TokenKind::Comma),
            '،' => self.make_token(TokenKind::ArabicComma), // Arabic comma
            ';' => self.make_token(TokenKind::Semicolon),
            '؛' => self.make_token(TokenKind::ArabicSemicolon), // Arabic semicolon
            '+' => {
                if self.match_char('+') {
                    self.make_token(TokenKind::PlusPlus)
                } else if self.match_char('=') {
                    self.make_token(TokenKind::PlusEqual)
                } else {
                    self.make_token(TokenKind::Plus)
                }
            }
            '-' => {
                if self.match_char('-') {
                    self.make_token(TokenKind::MinusMinus)
                } else if self.match_char('=') {
                    self.make_token(TokenKind::MinusEqual)
                } else if self.match_char('>') {
                    self.make_token(TokenKind::Arrow)
                } else {
                    self.make_token(TokenKind::Minus)
                }
            }
            '*' => {
                if self.match_char('*') {
                    self.make_token(TokenKind::StarStar)
                } else if self.match_char('=') {
                    self.make_token(TokenKind::StarEqual)
                } else {
                    self.make_token(TokenKind::Star)
                }
            }
            '/' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::SlashEqual)
                } else {
                    self.make_token(TokenKind::Slash)
                }
            }
            '%' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::PercentEqual)
                } else {
                    self.make_token(TokenKind::Percent)
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::EqualEqual)
                } else if self.match_char('>') {
                    self.make_token(TokenKind::FatArrow)
                } else {
                    self.make_token(TokenKind::Equal)
                }
            }
            '!' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::BangEqual)
                } else {
                    self.make_token(TokenKind::Bang)
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::LessEqual)
                } else {
                    self.make_token(TokenKind::Less)
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::GreaterEqual)
                } else {
                    self.make_token(TokenKind::Greater)
                }
            }
            '&' => {
                if self.match_char('&') {
                    self.make_token(TokenKind::And)
                } else {
                    self.make_error("Expected '&&' / متوقع '&&'")
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.make_token(TokenKind::Or)
                } else {
                    self.make_error("Expected '||' / متوقع '||'")
                }
            }

            _ => self.make_error(&format!(
                "Unexpected character '{}' / حرف غير متوقع '{}'",
                c, c
            )),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token.kind == TokenKind::Eof {
            None
        } else {
            Some(token)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_source() {
        let mut lexer = Lexer::new("");
        let token = lexer.next_token();
        assert_eq!(token.kind, TokenKind::Eof);
    }

    #[test]
    fn test_arabic_keywords() {
        let mut lexer = Lexer::new("متغير ثابت دالة إذا وإلا");
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Const);
        assert_eq!(tokens[2].kind, TokenKind::Function);
        assert_eq!(tokens[3].kind, TokenKind::If);
        assert_eq!(tokens[4].kind, TokenKind::Else);
    }

    #[test]
    fn test_english_keywords_not_allowed() {
        let mut lexer = Lexer::new("let const function if else");
        let tokens: Vec<_> = lexer.tokenize();

        assert!(matches!(&tokens[0].kind, TokenKind::Error(_)));
        assert!(matches!(&tokens[1].kind, TokenKind::Error(_)));
        assert!(matches!(&tokens[2].kind, TokenKind::Error(_)));
        assert!(matches!(&tokens[3].kind, TokenKind::Error(_)));
        assert!(matches!(&tokens[4].kind, TokenKind::Error(_)));
    }

    #[test]
    fn test_arabic_variable_declaration() {
        let source = "متغير س = 5";
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "س"));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert_eq!(tokens[3].kind, TokenKind::IntLiteral(5));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 1e10");
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
        assert!(matches!(tokens[1].kind, TokenKind::FloatLiteral(f) if (f - 3.14).abs() < 0.001));
        assert!(matches!(tokens[2].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""مرحبا" 'hello'"#);
        let tokens: Vec<_> = lexer.tokenize();

        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "مرحبا"));
        assert!(matches!(&tokens[1].kind, TokenKind::StringLiteral(s) if s == "hello"));
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / == != <= >= -> =>");
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[4].kind, TokenKind::EqualEqual);
        assert_eq!(tokens[5].kind, TokenKind::BangEqual);
        assert_eq!(tokens[6].kind, TokenKind::LessEqual);
        assert_eq!(tokens[7].kind, TokenKind::GreaterEqual);
        assert_eq!(tokens[8].kind, TokenKind::Arrow);
        assert_eq!(tokens[9].kind, TokenKind::FatArrow);
    }

    #[test]
    fn test_comments() {
        let source = r#"
            متغير س = 5 // هذا تعليق
            متغير ص = 10
        "#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        let keywords: Vec<_> = tokens.iter().filter(|t| t.kind == TokenKind::Let).collect();
        assert_eq!(keywords.len(), 2);
    }

    #[test]
    fn test_english_identifiers_produce_errors() {
        let source = r#"متغير userName = "أحمد""#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(&tokens[1].kind, TokenKind::Error(msg) if msg.contains("English")));
    }

    #[test]
    fn test_arabic_only_identifiers() {
        let source = r#"متغير اسم_المستخدم = "أحمد""#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "اسم_المستخدم"));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert!(matches!(&tokens[3].kind, TokenKind::StringLiteral(s) if s == "أحمد"));
    }

    #[test]
    fn test_arabic_identifier_with_underscore() {
        let source = "متغير _خاص = 5";
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "_خاص"));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert_eq!(tokens[3].kind, TokenKind::IntLiteral(5));
    }

    #[test]
    fn test_arabic_identifier_with_digits() {
        let source = "متغير رقم1 = 10";
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "رقم1"));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert_eq!(tokens[3].kind, TokenKind::IntLiteral(10));
    }

    #[test]
    fn test_block_doc_comment() {
        let source = r#"/**
 * دالة لحساب المضروب
 * @معامل ن - العدد المدخل
 */
دالة عاملي(ن: عدد) {}"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert!(
            matches!(&tokens[0].kind, TokenKind::BlockDocComment(s) if s.contains("دالة لحساب المضروب"))
        );
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::Function);
    }

    #[test]
    fn test_regular_comment_skipped() {
        let source = r#"// هذا تعليق عادي
متغير س = 5"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Newline);
        assert_eq!(tokens[1].kind, TokenKind::Let);
    }

    #[test]
    fn test_lexer_with_interner() {
        use crate::utils::StringInterner;

        let source = "متغير س = 5";
        let mut interner = StringInterner::new();
        let mut lexer = Lexer::with_interner(source, &mut interner);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "س"));

        // The identifier should have been interned
        assert_eq!(interner.len(), 1);
        assert!(interner.contains("س"));
    }

    #[test]
    fn test_interner_deduplicates_identifiers() {
        use crate::utils::StringInterner;

        let source = "متغير س = س + س";
        let mut interner = StringInterner::new();
        let mut lexer = Lexer::with_interner(source, &mut interner);
        let _tokens = lexer.tokenize();

        // Even though 'س' appears 3 times, it should be interned only once
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_interner_multiple_identifiers() {
        use crate::utils::StringInterner;

        let source = "متغير س = 5\nمتغير ص = س + 10";
        let mut interner = StringInterner::new();
        let mut lexer = Lexer::with_interner(source, &mut interner);
        let _tokens = lexer.tokenize();

        // Two unique identifiers: 'س' and 'ص'
        assert_eq!(interner.len(), 2);
        assert!(interner.contains("س"));
        assert!(interner.contains("ص"));
    }
}
