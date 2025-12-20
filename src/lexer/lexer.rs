//! Main lexer implementation

use super::keywords::lookup_keyword;
use super::token::{Token, TokenKind};
use crate::error::Span;
use unicode_normalization::UnicodeNormalization;

/// The Tarqeem lexer
///
/// Converts source code into a stream of tokens.
/// Supports both Arabic and English keywords and identifiers.
pub struct Lexer {
    /// Source code (NFC normalized)
    source: Vec<char>,
    /// Current position in source
    position: usize,
    /// Current line number (1-indexed)
    line: usize,
    /// Current column number (1-indexed)
    column: usize,
    /// Start position of current token
    token_start: usize,
    /// Start line of current token
    token_start_line: usize,
    /// Start column of current token
    token_start_column: usize,
}

impl Lexer {
    /// Create a new lexer from source code
    pub fn new(source: &str) -> Self {
        // Normalize Unicode to NFC form for consistent comparison
        let normalized: String = source.nfc().collect();
        Self {
            source: normalized.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            token_start: 0,
            token_start_line: 1,
            token_start_column: 1,
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        // Check for doc comments first
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

        // Check for newlines
        if c == '\n' {
            return self.make_token(TokenKind::Newline);
        }

        // Numbers (including Arabic-Indic numerals)
        if self.is_digit(c) {
            return self.scan_number(c);
        }

        // Identifiers and keywords
        if self.is_identifier_start(c) {
            return self.scan_identifier(c);
        }

        // Strings
        if c == '"' || c == '\'' || c == '«' {
            return self.scan_string(c);
        }

        // Operators and punctuation
        self.scan_operator(c)
    }

    /// Tokenize entire source and return all tokens
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

    // ============ Helper Methods ============

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

    // ============ Character Classification ============

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
        c.is_alphabetic() || c == '_' || self.is_arabic_letter(c)
    }

    fn is_identifier_continue(&self, c: char) -> bool {
        self.is_identifier_start(c) || c.is_ascii_digit() || self.is_arabic_digit(c)
    }

    fn is_arabic_letter(&self, c: char) -> bool {
        // Arabic Unicode letter ranges (excluding punctuation and digits)
        // Note: We explicitly exclude Arabic comma (،) and semicolon (؛) as they are punctuation
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

    // ============ Whitespace and Comments ============

    /// Skip whitespace and regular comments, but return doc comments as tokens
    /// Returns Some(Token) if a doc comment was found, None otherwise
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
                    // Check if it's a doc comment ///
                    if self.position + 2 < self.source.len() && self.source[self.position + 2] == '/' {
                        // Doc comment - don't skip, return as token
                        return Some(self.scan_doc_comment());
                    }
                    // Regular single-line comment - skip it
                    self.advance(); // consume first /
                    self.advance(); // consume second /
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek_next() == '*' => {
                    // Check if it's a block doc comment /**
                    if self.position + 2 < self.source.len() && self.source[self.position + 2] == '*' {
                        // Check it's not /*** (which is not a doc comment)
                        if self.position + 3 >= self.source.len() || self.source[self.position + 3] != '*' {
                            // Block doc comment - don't skip, return as token
                            return Some(self.scan_block_doc_comment());
                        }
                    }
                    // Regular multi-line comment - skip it
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

    /// Scan a single-line doc comment (///)
    fn scan_doc_comment(&mut self) -> Token {
        self.token_start = self.position;
        self.token_start_line = self.line;
        self.token_start_column = self.column;

        // Skip the /// prefix
        self.advance(); // first /
        self.advance(); // second /
        self.advance(); // third /

        // Collect the content
        let mut content = String::new();

        // Handle multiple consecutive doc comment lines
        loop {
            // Skip leading space after ///
            if self.peek() == ' ' {
                self.advance();
            }

            // Collect the rest of the line
            while !self.is_at_end() && self.peek() != '\n' {
                content.push(self.advance());
            }

            // Check if the next line is also a doc comment
            if self.is_at_end() {
                break;
            }

            // Skip the newline
            let newline_pos = self.position;
            self.advance();

            // Skip whitespace
            while !self.is_at_end() && (self.peek() == ' ' || self.peek() == '\t') {
                self.advance();
            }

            // Check if next line is also a doc comment
            if self.peek() == '/' && self.peek_next() == '/' {
                if self.position + 2 < self.source.len() && self.source[self.position + 2] == '/' {
                    // Another doc comment line - add newline and continue
                    content.push('\n');
                    self.advance(); // first /
                    self.advance(); // second /
                    self.advance(); // third /
                    continue;
                }
            }

            // Not a doc comment line - backtrack to after the newline
            // We need to reset position to just after the newline we consumed
            self.position = newline_pos + 1;
            self.line = self.token_start_line + content.matches('\n').count() + 1;
            self.column = 1;
            break;
        }

        self.make_token(TokenKind::DocComment(content.trim_end().to_string()))
    }

    /// Scan a block doc comment (/** ... */)
    fn scan_block_doc_comment(&mut self) -> Token {
        self.token_start = self.position;
        self.token_start_line = self.line;
        self.token_start_column = self.column;

        // Skip the /** prefix
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

        // Clean up the content: remove leading * from each line (common doc style)
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

    // ============ Token Scanners ============

    fn scan_number(&mut self, first: char) -> Token {
        let mut value: i64 = self.digit_value(first) as i64;

        // Consume integer part
        while !self.is_at_end() && self.is_digit(self.peek()) {
            let c = self.advance();
            value = value * 10 + self.digit_value(c) as i64;
        }

        // Check for decimal point
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

        // Check for exponent
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

        self.make_token(TokenKind::IntLiteral(value))
    }

    fn scan_identifier(&mut self, first: char) -> Token {
        let mut ident = String::new();
        ident.push(first);

        while !self.is_at_end() && self.is_identifier_continue(self.peek()) {
            ident.push(self.advance());
        }

        // Check if it's a keyword
        if let Some(keyword) = lookup_keyword(&ident) {
            self.make_token(keyword)
        } else {
            self.make_token(TokenKind::Identifier(ident))
        }
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
            // Single-character tokens
            '(' => self.make_token(TokenKind::LeftParen),
            ')' => self.make_token(TokenKind::RightParen),
            '{' => self.make_token(TokenKind::LeftBrace),
            '}' => self.make_token(TokenKind::RightBrace),
            '[' => self.make_token(TokenKind::LeftBracket),
            ']' => self.make_token(TokenKind::RightBracket),
            '.' => self.make_token(TokenKind::Dot),
            ':' => self.make_token(TokenKind::Colon),
            '?' => self.make_token(TokenKind::Question),

            // Comma (ASCII and Arabic)
            ',' => self.make_token(TokenKind::Comma),
            '،' => self.make_token(TokenKind::ArabicComma),

            // Semicolon (ASCII and Arabic)
            ';' => self.make_token(TokenKind::Semicolon),
            '؛' => self.make_token(TokenKind::ArabicSemicolon),

            // Two-character operators
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

impl Iterator for Lexer {
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
    fn test_english_keywords() {
        let mut lexer = Lexer::new("let const function if else");
        let tokens: Vec<_> = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Const);
        assert_eq!(tokens[2].kind, TokenKind::Function);
        assert_eq!(tokens[3].kind, TokenKind::If);
        assert_eq!(tokens[4].kind, TokenKind::Else);
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

        // Should skip comments and parse both variables
        let keywords: Vec<_> = tokens.iter().filter(|t| t.kind == TokenKind::Let).collect();
        assert_eq!(keywords.len(), 2);
    }

    #[test]
    fn test_mixed_arabic_english() {
        let source = r#"
            متغير userName = "أحمد"
            const userAge = 25
            اطبع(userName)
        "#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer
            .tokenize()
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Newline))
            .collect();

        // First variable declaration: متغير userName = "أحمد"
        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "userName"));

        // Second variable declaration: const userAge = 25
        // tokens[4] = const, tokens[5] = userAge
        assert_eq!(tokens[4].kind, TokenKind::Const);
        assert!(matches!(&tokens[5].kind, TokenKind::Identifier(s) if s == "userAge"));
    }

    #[test]
    fn test_doc_comment_single_line() {
        let source = r#"/// هذا تعليق توثيقي
دالة اختبار() {}"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert!(matches!(&tokens[0].kind, TokenKind::DocComment(s) if s == "هذا تعليق توثيقي"));
        // Next token should be Newline (doc comment stops before it) or Function
        // depending on how we handle the newline after doc comment
        assert_eq!(tokens[1].kind, TokenKind::Function);
    }

    #[test]
    fn test_doc_comment_multi_line() {
        let source = r#"/// سطر أول
/// سطر ثاني
دالة اختبار() {}"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        assert!(
            matches!(&tokens[0].kind, TokenKind::DocComment(s) if s == "سطر أول\nسطر ثاني")
        );
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

        assert!(matches!(&tokens[0].kind, TokenKind::BlockDocComment(s) if s.contains("دالة لحساب المضروب")));
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::Function);
    }

    #[test]
    fn test_regular_comment_skipped() {
        let source = r#"// هذا تعليق عادي
متغير س = 5"#;
        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize();

        // Regular comment should be skipped
        assert_eq!(tokens[0].kind, TokenKind::Newline);
        assert_eq!(tokens[1].kind, TokenKind::Let);
    }
}
