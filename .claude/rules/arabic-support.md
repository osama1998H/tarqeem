# Arabic Language Support Rules

This file defines requirements for Arabic language support in Tarqeem.

## Core Principle

Tarqeem is **Arabic-first, but bilingual**. Every user-facing feature must work in Arabic and have an English equivalent.

## Keyword Mapping

### All Keywords Need Both Forms

```rust
// In src/lexer/keywords.rs
pub fn get_keyword(text: &str) -> Option<TokenKind> {
    match text {
        // Arabic primary, English alias
        "متغير" | "let" => Some(TokenKind::Let),
        "ثابت" | "const" => Some(TokenKind::Const),
        "دالة" | "function" | "fn" => Some(TokenKind::Function),
        "إذا" | "if" => Some(TokenKind::If),
        "وإلا" | "else" => Some(TokenKind::Else),
        // ... etc
        _ => None,
    }
}
```

### When Adding a New Keyword

1. Choose a clear Arabic term
2. Add English alias(es)
3. Document the mapping in both CLAUDE.md and README.md
4. Test both forms

## Error Messages

### Bilingual Error Format (REQUIRED)

```rust
pub struct Diagnostic {
    pub message: String,      // English (required)
    pub message_ar: String,   // Arabic (required)
    pub span: Span,
    pub level: DiagnosticLevel,
}
```

### Error Message Guidelines

```rust
// GOOD: Both languages, clear message
Diagnostic {
    message: "Variable 'x' is not defined",
    message_ar: "المتغير 'x' غير معرّف",
    // ...
}

// BAD: Missing Arabic
Diagnostic {
    message: "Variable 'x' is not defined",
    message_ar: "",  // WRONG!
    // ...
}

// BAD: Machine-translated Arabic
Diagnostic {
    message: "Cannot assign to constant",
    message_ar: "لا يمكن التخصيص إلى ثابت",  // Awkward translation
    // ...
}
```

### Common Error Messages Reference

| English | Arabic |
|---------|--------|
| Undefined variable '{name}' | المتغير '{name}' غير معرّف |
| Type mismatch: expected {a}, found {b} | عدم تطابق الأنماط: متوقع {a}، وُجد {b} |
| Cannot assign to constant | لا يمكن تعيين قيمة لمتغير ثابت |
| Missing semicolon | فاصلة منقوطة مفقودة |
| Unexpected token | رمز غير متوقع |
| Function '{name}' not found | الدالة '{name}' غير موجودة |
| Invalid number of arguments | عدد غير صحيح من المعاملات |

## Unicode Handling

### NFC Normalization (REQUIRED)

Arabic text must be NFC-normalized before comparison:

```rust
use unicode_normalization::UnicodeNormalization;

fn normalize_identifier(s: &str) -> String {
    s.nfc().collect()
}

// ALWAYS normalize before:
// - Identifier comparison
// - Symbol table lookup
// - Error message formatting
```

### Arabic Character Recognition

```rust
fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' |  // Arabic
                '\u{0750}'..='\u{077F}' |  // Arabic Supplement
                '\u{08A0}'..='\u{08FF}' |  // Arabic Extended-A
                '\u{FB50}'..='\u{FDFF}' |  // Arabic Presentation Forms-A
                '\u{FE70}'..='\u{FEFF}')   // Arabic Presentation Forms-B
}
```

### Arabic Punctuation

The lexer must accept both Arabic and ASCII punctuation:

| ASCII | Arabic | Name |
|-------|--------|------|
| `,` | `،` (U+060C) | Comma |
| `;` | `؛` (U+061B) | Semicolon |
| `?` | `؟` (U+061F) | Question mark |
| `"` | `«` `»` | Quotation marks |

```rust
fn is_comma(c: char) -> bool {
    c == ',' || c == '،'
}

fn is_semicolon(c: char) -> bool {
    c == ';' || c == '؛'
}
```

## RTL Text Handling

### String Literals

String literals preserve their content exactly:

```rust
// The lexer should NOT modify RTL ordering inside strings
let source = r#"اطبع("مرحباً")"#;
// The string content is exactly: مرحباً
```

### Comments

Comments can be in any language, any direction:

```rust
// This is an English comment
// هذا تعليق بالعربية
```

### Mixed Identifiers

Identifiers can mix Arabic and ASCII (numbers, underscores):

```rust
// Valid identifiers
متغير
متغير1
متغير_اختبار
_متغير
```

## Testing Arabic Support

### Required Tests

```rust
#[test]
fn test_arabic_identifier() {
    let source = "متغير اسم = \"أحمد\"";
    assert!(parse(source).is_ok());
}

#[test]
fn test_arabic_comma() {
    let source = "دالة(أ، ب)";  // Arabic comma
    assert!(parse(source).is_ok());
}

#[test]
fn test_normalization() {
    // These should be equal after NFC normalization
    let a = "متغير";
    let b = "متغير";  // May have different byte sequence
    assert_eq!(normalize(a), normalize(b));
}

#[test]
fn test_mixed_direction() {
    let source = r#"متغير x = 5"#;  // Mixed Arabic/English
    assert!(parse(source).is_ok());
}
```

## Documentation

### Code Comments

Write code comments in English (for broader accessibility), but document Arabic behavior:

```rust
// Handles Arabic keyword "متغير" (mutable variable declaration)
// Equivalent to JavaScript's "let" or Python's variable without type hint
fn parse_let(&mut self) -> ParseResult<Stmt> { }
```

### User Documentation

User documentation (README, language spec) should have:
- Arabic examples with Arabic keywords
- English examples with English keywords
- Clear mapping tables between the two
