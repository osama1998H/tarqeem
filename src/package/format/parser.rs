//! محلل صيغة الحزمة
//! Parser for package format

use super::error::{FormatError, FormatErrorKind, FormatResult, Location};
use super::lexer::{Lexer, Token};
use super::value::Value;
use indexmap::IndexMap;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    current_loc: Location,
    peeked: Option<(Token, Location)>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> FormatResult<Self> {
        let mut lexer = Lexer::new(source);
        let current_loc = lexer.location();
        let current = lexer.next_token()?;

        Ok(Self {
            lexer,
            current,
            current_loc,
            peeked: None,
        })
    }

    fn location(&self) -> Location {
        self.current_loc
    }

    fn advance(&mut self) -> FormatResult<Token> {
        let old = std::mem::replace(&mut self.current, Token::Eof);
        let _old_loc = self.current_loc;

        if let Some((token, loc)) = self.peeked.take() {
            self.current = token;
            self.current_loc = loc;
        } else {
            self.current_loc = self.lexer.location();
            self.current = self.lexer.next_token()?;
        }

        Ok(old)
    }

    fn check(&self, expected: &Token) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(expected)
    }

    fn skip_newlines_and_comments(&mut self) -> FormatResult<()> {
        loop {
            match &self.current {
                Token::Newline | Token::Comment(_) => {
                    self.advance()?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn skip_to_end_of_line(&mut self) -> FormatResult<()> {
        loop {
            match &self.current {
                Token::Newline | Token::Eof => break,
                _ => {
                    self.advance()?;
                }
            }
        }
        Ok(())
    }

    pub fn parse(&mut self) -> FormatResult<Value> {
        self.skip_newlines_and_comments()?;
        self.parse_object(0)
    }

    fn parse_object(&mut self, min_indent: usize) -> FormatResult<Value> {
        let mut map = IndexMap::new();

        loop {
            self.skip_newlines_and_comments()?;

            if matches!(self.current, Token::Eof) {
                break;
            }

            let current_indent = match &self.current {
                Token::Indent(n) => {
                    let indent = *n;
                    if indent < min_indent {
                        break; // نهاية الكائن الحالي
                    }
                    self.advance()?;
                    indent
                }
                Token::Identifier(_) if min_indent == 0 => 0,
                _ => break,
            };

            if current_indent < min_indent {
                break;
            }

            self.skip_newlines_and_comments()?;

            let key = match &self.current {
                Token::Identifier(s) => {
                    let k = s.clone();
                    self.advance()?;
                    k
                }
                Token::Eof => break,
                Token::Newline => continue,
                _ => {
                    return Err(FormatError::new(
                        FormatErrorKind::ExpectedKey,
                        self.location(),
                    ))
                }
            };

            if !self.check(&Token::Colon) {
                return Err(FormatError::new(
                    FormatErrorKind::ExpectedColon,
                    self.location(),
                ));
            }
            self.advance()?;

            self.skip_newlines_and_comments()?;

            let value = self.parse_value(current_indent)?;

            if map.contains_key(&key) {
                return Err(FormatError::new(
                    FormatErrorKind::DuplicateKey(key),
                    self.location(),
                ));
            }

            map.insert(key, value);
        }

        Ok(Value::Object(map))
    }

    fn parse_value(&mut self, current_indent: usize) -> FormatResult<Value> {
        match &self.current {
            Token::String(s) => {
                let value = Value::String(s.clone());
                self.advance()?;
                self.skip_to_end_of_line()?;
                Ok(value)
            }
            Token::Number(n) => {
                let value = Value::Number(*n);
                self.advance()?;
                self.skip_to_end_of_line()?;
                Ok(value)
            }
            Token::Bool(b) => {
                let value = Value::Bool(*b);
                self.advance()?;
                self.skip_to_end_of_line()?;
                Ok(value)
            }
            Token::Null => {
                self.advance()?;
                self.skip_to_end_of_line()?;
                Ok(Value::Null)
            }
            Token::Identifier(s) => {
                let value = Value::String(s.clone());
                self.advance()?;
                self.skip_to_end_of_line()?;
                Ok(value)
            }
            Token::Dash => self.parse_array(current_indent),
            Token::Newline => {
                self.advance()?;
                match &self.current {
                    Token::Indent(n) if *n > current_indent => {
                        let child_indent = *n;
                        self.advance()?;
                        match &self.current {
                            Token::Dash => self.parse_array_at_indent(child_indent),
                            _ => {
                                self.peeked = Some((self.current.clone(), self.current_loc));
                                self.current = Token::Indent(child_indent);
                                self.parse_object(child_indent)
                            }
                        }
                    }
                    Token::Dash => {
                        self.parse_array(current_indent)
                    }
                    _ => Ok(Value::Null),
                }
            }
            Token::Indent(n) => {
                let child_indent = *n;
                if child_indent > current_indent {
                    self.advance()?;
                    match &self.current {
                        Token::Dash => self.parse_array_at_indent(child_indent),
                        _ => {
                            self.peeked = Some((self.current.clone(), self.current_loc));
                            self.current = Token::Indent(child_indent);
                            self.parse_object(child_indent)
                        }
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            Token::Eof | Token::Comment(_) => Ok(Value::Null),
            Token::Colon => Err(FormatError::new(
                FormatErrorKind::ExpectedValue,
                self.location(),
            )),
        }
    }

    fn parse_array(&mut self, min_indent: usize) -> FormatResult<Value> {
        let mut items = Vec::new();

        loop {
            self.skip_newlines_and_comments()?;

            match &self.current {
                Token::Eof => break,
                Token::Dash => {
                    self.advance()?;
                    let value = self.parse_array_item(min_indent)?;
                    items.push(value);
                }
                Token::Indent(n) => {
                    if *n < min_indent {
                        break;
                    }
                    let indent = *n;
                    self.advance()?;
                    if !self.check(&Token::Dash) {
                        self.peeked = Some((self.current.clone(), self.current_loc));
                        self.current = Token::Indent(indent);
                        break;
                    }
                    self.advance()?;
                    let value = self.parse_array_item(indent)?;
                    items.push(value);
                }
                _ => break,
            }
        }

        Ok(Value::Array(items))
    }

    fn parse_array_at_indent(&mut self, indent: usize) -> FormatResult<Value> {
        let mut items = Vec::new();

        self.advance()?;
        let value = self.parse_array_item(indent)?;
        items.push(value);

        loop {
            self.skip_newlines_and_comments()?;

            match &self.current {
                Token::Eof => break,
                Token::Indent(n) if *n == indent => {
                    self.advance()?;
                    if self.check(&Token::Dash) {
                        self.advance()?;
                        let value = self.parse_array_item(indent)?;
                        items.push(value);
                    } else {
                        break;
                    }
                }
                Token::Indent(n) if *n < indent => break,
                _ => break,
            }
        }

        Ok(Value::Array(items))
    }

    fn parse_array_item(&mut self, _indent: usize) -> FormatResult<Value> {
        match &self.current {
            Token::String(s) => {
                let value = Value::String(s.clone());
                self.advance()?;
                Ok(value)
            }
            Token::Number(n) => {
                let value = Value::Number(*n);
                self.advance()?;
                Ok(value)
            }
            Token::Bool(b) => {
                let value = Value::Bool(*b);
                self.advance()?;
                Ok(value)
            }
            Token::Null => {
                self.advance()?;
                Ok(Value::Null)
            }
            Token::Identifier(first) => {
                let mut result = first.clone();
                self.advance()?;

                loop {
                    match &self.current {
                        Token::Identifier(s) => {
                            result.push(' ');
                            result.push_str(s);
                            self.advance()?;
                        }
                        Token::Number(n) => {
                            result.push(' ');
                            result.push_str(&n.to_string());
                            self.advance()?;
                        }
                        Token::Newline | Token::Eof | Token::Comment(_) | Token::Indent(_) => {
                            break;
                        }
                        _ => break,
                    }
                }

                Ok(Value::String(result))
            }
            Token::Newline => {
                self.advance()?;
                Ok(Value::Null)
            }
            _ => Ok(Value::Null),
        }
    }
}

pub fn parse(source: &str) -> FormatResult<Value> {
    let mut parser = Parser::new(source)?;
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_object() {
        let source = r#"
اسم: مكتبتي
نسخة: 1.0.0
"#;
        let value = parse(source).unwrap();
        assert!(value.is_object());
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("اسم").unwrap().as_str(), Some("مكتبتي"));
        assert_eq!(obj.get("نسخة").unwrap().as_str(), Some("1.0.0"));
    }

    #[test]
    fn test_parse_nested_object() {
        let source = r#"
حزمة:
    اسم: مكتبتي
    نسخة: 1.0.0
"#;
        let value = parse(source).unwrap();
        let obj = value.as_object().unwrap();
        let pkg = obj.get("حزمة").unwrap().as_object().unwrap();
        assert_eq!(pkg.get("اسم").unwrap().as_str(), Some("مكتبتي"));
    }

    #[test]
    fn test_parse_array() {
        let source = r#"
مؤلفون:
    - أحمد
    - محمد
    - فاطمة
"#;
        let value = parse(source).unwrap();
        let obj = value.as_object().unwrap();
        let authors = obj.get("مؤلفون").unwrap().as_array().unwrap();
        assert_eq!(authors.len(), 3);
        assert_eq!(authors[0].as_str(), Some("أحمد"));
    }

    #[test]
    fn test_parse_arabic_numbers() {
        let source = r#"
عدد: ١٢٣
عشري: ٣٫١٤
"#;
        let value = parse(source).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("عدد").unwrap().as_number(), Some(123.0));
        assert!((obj.get("عشري").unwrap().as_number().unwrap() - 3.14).abs() < 0.01);
    }

    #[test]
    fn test_parse_booleans() {
        let source = r#"
نشط: نعم
موقف: لا
"#;
        let value = parse(source).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("نشط").unwrap().as_bool(), Some(true));
        assert_eq!(obj.get("موقف").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn test_parse_quoted_strings() {
        let source = r#"
وصف: "مكتبة رائعة للتعامل مع النصوص"
"#;
        let value = parse(source).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(
            obj.get("وصف").unwrap().as_str(),
            Some("مكتبة رائعة للتعامل مع النصوص")
        );
    }

    #[test]
    fn test_parse_comments() {
        let source = r#"
# هذا تعليق
اسم: مكتبتي  # تعليق آخر
نسخة: 1.0.0
"#;
        let value = parse(source).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("اسم").unwrap().as_str(), Some("مكتبتي"));
    }

    #[test]
    fn test_parse_full_package() {
        let source = r#"
# ملف تهيئة حزمة ترقيم

حزمة:
    اسم: مكتبتي
    نسخة: ١.٠.٠
    وصف: "مكتبة رائعة"
    مؤلفون:
        - أحمد محمد
        - فاطمة علي
    رخصة: MIT
    مدخل: مصدر/رئيسي.ترقيم

اعتماديات:
    json: 2.0.0
    أدوات:
        نسخة: 1.0.0
        اختياري: نعم
"#;
        let value = parse(source).unwrap();
        assert!(value.is_object());

        let obj = value.as_object().unwrap();

        let pkg = obj.get("حزمة").unwrap().as_object().unwrap();
        assert_eq!(pkg.get("اسم").unwrap().as_str(), Some("مكتبتي"));
        assert_eq!(pkg.get("نسخة").unwrap().as_str(), Some("1.0.0"));
        assert_eq!(pkg.get("رخصة").unwrap().as_str(), Some("MIT"));

        let authors = pkg.get("مؤلفون").unwrap().as_array().unwrap();
        assert_eq!(authors.len(), 2);

        let deps = obj.get("اعتماديات").unwrap().as_object().unwrap();
        assert!(deps.contains_key("json"));
        assert!(deps.contains_key("أدوات"));
    }
}
