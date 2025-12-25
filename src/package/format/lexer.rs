//! محلل الرموز لصيغة الحزمة
//! Lexer for package format

use super::error::{FormatError, FormatErrorKind, FormatResult, Location};
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Colon,
    Dash,
    Newline,
    Indent(usize),
    Identifier(String),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Comment(String),
    Eof,
}

impl Token {
    pub fn is_value(&self) -> bool {
        matches!(
            self,
            Token::String(_) | Token::Number(_) | Token::Bool(_) | Token::Null
        )
    }
}

#[allow(dead_code)]
pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            line: 1,
            column: 1,
            at_line_start: true,
        }
    }

    pub fn location(&self) -> Location {
        Location::new(self.line, self.column)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
                self.at_line_start = true;
            } else {
                self.column += 1;
                if !ch.is_whitespace() {
                    self.at_line_start = false;
                }
            }
        }
        c
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_indent(&mut self) -> usize {
        let mut count = 0;
        while let Some(&c) = self.peek() {
            match c {
                ' ' => {
                    count += 1;
                    self.advance();
                }
                '\t' => {
                    count += 4;
                    self.advance();
                }
                _ => break,
            }
        }
        count
    }

    fn read_identifier(&mut self, first: char) -> String {
        let mut result = String::new();
        result.push(first);

        while let Some(&c) = self.peek() {
            if c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '/'
                || c == '@'
                || is_arabic_char(c)
            {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }

        result
    }

    fn read_string(&mut self, quote: char) -> FormatResult<String> {
        let start_loc = self.location();
        let mut result = String::new();

        loop {
            match self.advance() {
                Some(c) if c == quote => break,
                Some('\\') => match self.advance() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some(c) => result.push(c),
                    None => {
                        return Err(FormatError::new(
                            FormatErrorKind::UnterminatedString,
                            start_loc,
                        ))
                    }
                },
                Some('\n') => {
                    return Err(FormatError::new(
                        FormatErrorKind::UnterminatedString,
                        start_loc,
                    ))
                }
                Some(c) => result.push(c),
                None => {
                    return Err(FormatError::new(
                        FormatErrorKind::UnterminatedString,
                        start_loc,
                    ))
                }
            }
        }

        Ok(result)
    }

    fn read_number_or_version(&mut self, first: char) -> FormatResult<Token> {
        let start_loc = self.location();
        let mut result = String::new();
        result.push(convert_arabic_digit(first));

        let mut dot_count = 0;

        while let Some(&c) = self.peek() {
            if is_digit(c) {
                result.push(convert_arabic_digit(c));
                self.advance();
            } else if c == '.' || c == '٫' {
                result.push('.');
                dot_count += 1;
                self.advance();
            } else {
                break;
            }
        }

        if dot_count > 1 {
            Ok(Token::String(result))
        } else {
            result.parse::<f64>().map_or_else(
                |_| {
                    Err(FormatError::new(
                        FormatErrorKind::InvalidNumber(result),
                        start_loc,
                    ))
                },
                |n| Ok(Token::Number(n)),
            )
        }
    }

    fn read_comment(&mut self) -> String {
        let mut result = String::new();
        while let Some(&c) = self.peek() {
            if c == '\n' {
                break;
            }
            result.push(c);
            self.advance();
        }
        result.trim().to_string()
    }

    pub fn next_token(&mut self) -> FormatResult<Token> {
        if self.at_line_start {
            let indent = self.read_indent();
            if let Some(&c) = self.peek() {
                if c == '\n' {
                    self.advance();
                    return Ok(Token::Newline);
                }
                if c == '#' {
                    self.advance();
                    let comment = self.read_comment();
                    return Ok(Token::Comment(comment));
                }
                if indent > 0 {
                    self.at_line_start = false;
                    return Ok(Token::Indent(indent));
                }
            }
            self.at_line_start = false;
        }

        self.skip_whitespace();

        let loc = self.location();

        match self.advance() {
            None => Ok(Token::Eof),
            Some('\n') => {
                self.at_line_start = true;
                Ok(Token::Newline)
            }
            Some(':') => Ok(Token::Colon),
            Some('-') => {
                if let Some(&c) = self.peek() {
                    if is_digit(c) {
                        self.advance();
                        match self.read_number_or_version(c)? {
                            Token::Number(n) => return Ok(Token::Number(-n)),
                            Token::String(s) => return Ok(Token::String(format!("-{}", s))),
                            other => return Ok(other),
                        }
                    }
                }
                Ok(Token::Dash)
            }
            Some('#') => {
                let comment = self.read_comment();
                Ok(Token::Comment(comment))
            }
            Some('"') => {
                let s = self.read_string('"')?;
                Ok(Token::String(s))
            }
            Some('\'') => {
                let s = self.read_string('\'')?;
                Ok(Token::String(s))
            }
            Some(c) if is_digit(c) => self.read_number_or_version(c),
            Some(c)
                if c.is_alphabetic() || is_arabic_char(c) || c == '_' || c == '/' || c == '@' =>
            {
                let ident = self.read_identifier(c);
                match ident.as_str() {
                    "نعم" | "صحيح" | "true" | "yes" => Ok(Token::Bool(true)),
                    "لا" | "خطأ" | "false" | "no" => Ok(Token::Bool(false)),
                    "لا_شيء" | "فارغ" | "null" | "nil" => Ok(Token::Null),
                    _ => Ok(Token::Identifier(ident)),
                }
            }
            Some(c) => Err(FormatError::new(FormatErrorKind::UnexpectedChar(c), loc)),
        }
    }

    pub fn tokenize(&mut self) -> FormatResult<Vec<(Token, Location)>> {
        let mut tokens = Vec::new();

        loop {
            let loc = self.location();
            let token = self.next_token()?;
            let is_eof = matches!(token, Token::Eof);
            tokens.push((token, loc));
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }
}

fn is_arabic_char(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '٠'..='٩')
}

fn convert_arabic_digit(c: char) -> char {
    match c {
        '٠' => '0',
        '١' => '1',
        '٢' => '2',
        '٣' => '3',
        '٤' => '4',
        '٥' => '5',
        '٦' => '6',
        '٧' => '7',
        '٨' => '8',
        '٩' => '9',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("اسم: مكتبتي");
        assert!(matches!(lexer.next_token(), Ok(Token::Identifier(_))));
        assert!(matches!(lexer.next_token(), Ok(Token::Colon)));
        assert!(matches!(lexer.next_token(), Ok(Token::Identifier(_))));
    }

    #[test]
    fn test_string_tokens() {
        let mut lexer = Lexer::new("\"مرحبا بالعالم\"");
        if let Ok(Token::String(s)) = lexer.next_token() {
            assert_eq!(s, "مرحبا بالعالم");
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_arabic_numbers() {
        let mut lexer = Lexer::new("١٢٣");
        if let Ok(Token::Number(n)) = lexer.next_token() {
            assert_eq!(n, 123.0);
        } else {
            panic!("Expected number token");
        }
    }

    #[test]
    fn test_arabic_decimal() {
        let mut lexer = Lexer::new("٣٫١٤");
        if let Ok(Token::Number(n)) = lexer.next_token() {
            assert!((n - 3.14).abs() < 0.01);
        } else {
            panic!("Expected number token");
        }
    }

    #[test]
    fn test_boolean_arabic() {
        let mut lexer = Lexer::new("نعم لا صحيح خطأ");
        assert!(matches!(lexer.next_token(), Ok(Token::Bool(true))));
        assert!(matches!(lexer.next_token(), Ok(Token::Bool(false))));
        assert!(matches!(lexer.next_token(), Ok(Token::Bool(true))));
        assert!(matches!(lexer.next_token(), Ok(Token::Bool(false))));
    }

    #[test]
    fn test_indent() {
        let mut lexer = Lexer::new("    اسم: قيمة");
        assert!(matches!(lexer.next_token(), Ok(Token::Indent(4))));
        assert!(matches!(lexer.next_token(), Ok(Token::Identifier(_))));
    }

    #[test]
    fn test_comment() {
        let mut lexer = Lexer::new("# هذا تعليق");
        if let Ok(Token::Comment(c)) = lexer.next_token() {
            assert_eq!(c, "هذا تعليق");
        } else {
            panic!("Expected comment token");
        }
    }

    #[test]
    fn test_list_dash() {
        let mut lexer = Lexer::new("- عنصر");
        assert!(matches!(lexer.next_token(), Ok(Token::Dash)));
        assert!(matches!(lexer.next_token(), Ok(Token::Identifier(_))));
    }
}
