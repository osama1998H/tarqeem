//! Comprehensive tests for Tokens

use super::token::*;
use crate::error::Span;

#[test]
fn test_token_new() {
    let span = Span::new(0, 5, 1, 1);
    let token = Token::new(TokenKind::Let, span.clone(), "let");

    assert_eq!(token.kind, TokenKind::Let);
    assert_eq!(token.span, span);
    assert_eq!(token.lexeme, "let");
}

#[test]
fn test_token_new_arabic() {
    let span = Span::new(0, 10, 1, 1);
    let token = Token::new(TokenKind::Let, span, "متغير");

    assert_eq!(token.lexeme, "متغير");
}

#[test]
fn test_token_display() {
    let span = Span::new(0, 3, 1, 1);
    let token = Token::new(TokenKind::IntLiteral(42), span, "42");

    let display = format!("{}", token);
    assert!(display.contains("IntLiteral"));
    assert!(display.contains("42"));
}

#[test]
fn test_token_kind_int_literal() {
    let kind = TokenKind::IntLiteral(123);
    assert!(kind.is_literal());
    assert!(!kind.is_keyword());
    assert!(!kind.is_operator());
}

#[test]
fn test_token_kind_float_literal() {
    let kind = TokenKind::FloatLiteral(3.14);
    assert!(kind.is_literal());
    assert!(!kind.is_keyword());
}

#[test]
fn test_token_kind_string_literal() {
    let kind = TokenKind::StringLiteral("hello".to_string());
    assert!(kind.is_literal());
    assert!(!kind.is_keyword());
}

#[test]
fn test_token_kind_string_literal_arabic() {
    let kind = TokenKind::StringLiteral("مرحبا".to_string());
    assert!(kind.is_literal());
}

#[test]
fn test_token_kind_true() {
    let kind = TokenKind::True;
    assert!(kind.is_literal());
    assert!(kind.is_keyword()); // True is both literal and keyword
}

#[test]
fn test_token_kind_false() {
    let kind = TokenKind::False;
    assert!(kind.is_literal());
    assert!(kind.is_keyword());
}

#[test]
fn test_token_kind_null() {
    let kind = TokenKind::Null;
    assert!(kind.is_literal());
    assert!(kind.is_keyword());
}

#[test]
fn test_token_kind_variable_keywords() {
    assert!(TokenKind::Let.is_keyword());
    assert!(TokenKind::Const.is_keyword());
}

#[test]
fn test_token_kind_function_keywords() {
    assert!(TokenKind::Function.is_keyword());
    assert!(TokenKind::Return.is_keyword());
    assert!(TokenKind::Async.is_keyword());
    assert!(TokenKind::Await.is_keyword());
}

#[test]
fn test_token_kind_control_flow_keywords() {
    assert!(TokenKind::If.is_keyword());
    assert!(TokenKind::Else.is_keyword());
    assert!(TokenKind::Match.is_keyword());
    assert!(TokenKind::Case.is_keyword());
    assert!(TokenKind::Default.is_keyword());
}

#[test]
fn test_token_kind_loop_keywords() {
    assert!(TokenKind::While.is_keyword());
    assert!(TokenKind::For.is_keyword());
    assert!(TokenKind::In.is_keyword());
    assert!(TokenKind::Do.is_keyword());
    assert!(TokenKind::Break.is_keyword());
    assert!(TokenKind::Continue.is_keyword());
}

#[test]
fn test_token_kind_oop_keywords() {
    assert!(TokenKind::Class.is_keyword());
    assert!(TokenKind::Interface.is_keyword());
    assert!(TokenKind::Extends.is_keyword());
    assert!(TokenKind::Implements.is_keyword());
    assert!(TokenKind::Public.is_keyword());
    assert!(TokenKind::Private.is_keyword());
    assert!(TokenKind::Protected.is_keyword());
    assert!(TokenKind::Static.is_keyword());
    assert!(TokenKind::Constructor.is_keyword());
    assert!(TokenKind::This.is_keyword());
    assert!(TokenKind::Super.is_keyword());
    assert!(TokenKind::New.is_keyword());
}

#[test]
fn test_token_kind_error_handling_keywords() {
    assert!(TokenKind::Try.is_keyword());
    assert!(TokenKind::Catch.is_keyword());
    assert!(TokenKind::Finally.is_keyword());
    assert!(TokenKind::Throw.is_keyword());
}

#[test]
fn test_token_kind_module_keywords() {
    assert!(TokenKind::Import.is_keyword());
    assert!(TokenKind::Export.is_keyword());
    assert!(TokenKind::From.is_keyword());
    assert!(TokenKind::As.is_keyword());
}

#[test]
fn test_token_kind_type_keywords() {
    assert!(TokenKind::TypeInt.is_type_keyword());
    assert!(TokenKind::TypeFloat.is_type_keyword());
    assert!(TokenKind::TypeString.is_type_keyword());
    assert!(TokenKind::TypeBool.is_type_keyword());
    assert!(TokenKind::TypeArray.is_type_keyword());
    assert!(TokenKind::TypeMap.is_type_keyword());
    assert!(TokenKind::TypeVoid.is_type_keyword());
    assert!(TokenKind::TypeAny.is_type_keyword());
}

#[test]
fn test_token_kind_type_not_regular_keyword() {
    assert!(!TokenKind::TypeInt.is_keyword());
    assert!(!TokenKind::TypeFloat.is_keyword());
}

#[test]
fn test_token_kind_arithmetic_operators() {
    assert!(TokenKind::Plus.is_operator());
    assert!(TokenKind::Minus.is_operator());
    assert!(TokenKind::Star.is_operator());
    assert!(TokenKind::Slash.is_operator());
    assert!(TokenKind::Percent.is_operator());
    assert!(TokenKind::StarStar.is_operator());
}

#[test]
fn test_token_kind_comparison_operators() {
    assert!(TokenKind::EqualEqual.is_operator());
    assert!(TokenKind::BangEqual.is_operator());
    assert!(TokenKind::Less.is_operator());
    assert!(TokenKind::LessEqual.is_operator());
    assert!(TokenKind::Greater.is_operator());
    assert!(TokenKind::GreaterEqual.is_operator());
}

#[test]
fn test_token_kind_logical_operators() {
    assert!(TokenKind::And.is_operator());
    assert!(TokenKind::Or.is_operator());
    assert!(TokenKind::Bang.is_operator());
}

#[test]
fn test_token_kind_assignment_operators() {
    assert!(TokenKind::Equal.is_operator());
    assert!(TokenKind::PlusEqual.is_operator());
    assert!(TokenKind::MinusEqual.is_operator());
    assert!(TokenKind::StarEqual.is_operator());
    assert!(TokenKind::SlashEqual.is_operator());
    assert!(TokenKind::PercentEqual.is_operator());
}

#[test]
fn test_token_kind_increment_decrement() {
    assert!(TokenKind::PlusPlus.is_operator());
    assert!(TokenKind::MinusMinus.is_operator());
}

#[test]
fn test_token_kind_doc_comment() {
    let kind = TokenKind::DocComment("This is a doc comment".to_string());
    assert!(kind.is_doc_comment());
    assert!(!kind.is_keyword());
    assert!(!kind.is_literal());
}

#[test]
fn test_token_kind_block_doc_comment() {
    let kind = TokenKind::BlockDocComment("Block doc comment".to_string());
    assert!(kind.is_doc_comment());
}

#[test]
fn test_token_kind_doc_comment_arabic() {
    let kind = TokenKind::DocComment("هذا تعليق توثيقي".to_string());
    assert!(kind.is_doc_comment());
}

#[test]
fn test_token_kind_identifier_not_keyword() {
    let kind = TokenKind::Identifier("myVar".to_string());
    assert!(!kind.is_keyword());
    assert!(!kind.is_literal());
    assert!(!kind.is_operator());
    assert!(!kind.is_type_keyword());
    assert!(!kind.is_doc_comment());
}

#[test]
fn test_token_kind_identifier_arabic() {
    let kind = TokenKind::Identifier("متغيري".to_string());
    assert!(!kind.is_keyword());
}

#[test]
fn test_token_kind_delimiters_not_operators() {
    assert!(!TokenKind::LeftParen.is_operator());
    assert!(!TokenKind::RightParen.is_operator());
    assert!(!TokenKind::LeftBrace.is_operator());
    assert!(!TokenKind::RightBrace.is_operator());
    assert!(!TokenKind::LeftBracket.is_operator());
    assert!(!TokenKind::RightBracket.is_operator());
    assert!(!TokenKind::Comma.is_operator());
    assert!(!TokenKind::Dot.is_operator());
    assert!(!TokenKind::Colon.is_operator());
    assert!(!TokenKind::Semicolon.is_operator());
    assert!(!TokenKind::Arrow.is_operator());
    assert!(!TokenKind::FatArrow.is_operator());
    assert!(!TokenKind::Question.is_operator());
}

#[test]
fn test_token_kind_arabic_punctuation_not_operators() {
    assert!(!TokenKind::ArabicComma.is_operator());
    assert!(!TokenKind::ArabicSemicolon.is_operator());
}

#[test]
fn test_token_kind_special_not_keyword() {
    assert!(!TokenKind::Eof.is_keyword());
    assert!(!TokenKind::Newline.is_keyword());
    assert!(!TokenKind::Error("test error".to_string()).is_keyword());
}

#[test]
fn test_token_kind_display_int() {
    let kind = TokenKind::IntLiteral(42);
    assert_eq!(format!("{}", kind), "42");
}

#[test]
fn test_token_kind_display_float() {
    let kind = TokenKind::FloatLiteral(3.14);
    assert_eq!(format!("{}", kind), "3.14");
}

#[test]
fn test_token_kind_display_string() {
    let kind = TokenKind::StringLiteral("hello".to_string());
    assert_eq!(format!("{}", kind), "\"hello\"");
}

#[test]
fn test_token_kind_display_identifier() {
    let kind = TokenKind::Identifier("myVar".to_string());
    assert_eq!(format!("{}", kind), "myVar");
}

#[test]
fn test_token_kind_display_error() {
    let kind = TokenKind::Error("unexpected character".to_string());
    assert!(format!("{}", kind).contains("unexpected character"));
}

#[test]
fn test_token_kind_display_keyword() {
    let kind = TokenKind::Let;
    let display = format!("{}", kind);
    assert!(display.contains("Let"));
}

#[test]
fn test_token_kind_clone() {
    let kind = TokenKind::StringLiteral("test".to_string());
    let cloned = kind.clone();
    assert_eq!(kind, cloned);
}

#[test]
fn test_token_kind_equality() {
    assert_eq!(TokenKind::Let, TokenKind::Let);
    assert_eq!(TokenKind::IntLiteral(5), TokenKind::IntLiteral(5));
    assert_ne!(TokenKind::IntLiteral(5), TokenKind::IntLiteral(6));
    assert_ne!(TokenKind::Let, TokenKind::Const);
}

#[test]
fn test_token_kind_equality_strings() {
    let s1 = TokenKind::StringLiteral("hello".to_string());
    let s2 = TokenKind::StringLiteral("hello".to_string());
    let s3 = TokenKind::StringLiteral("world".to_string());

    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
}

#[test]
fn test_token_kind_equality_identifiers() {
    let id1 = TokenKind::Identifier("foo".to_string());
    let id2 = TokenKind::Identifier("foo".to_string());
    let id3 = TokenKind::Identifier("bar".to_string());

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn test_token_clone() {
    let span = Span::new(0, 5, 1, 1);
    let token = Token::new(TokenKind::IntLiteral(100), span, "100");
    let cloned = token.clone();

    assert_eq!(token.kind, cloned.kind);
    assert_eq!(token.span, cloned.span);
    assert_eq!(token.lexeme, cloned.lexeme);
}

#[test]
fn test_token_equality() {
    let span1 = Span::new(0, 3, 1, 1);
    let span2 = Span::new(0, 3, 1, 1);

    let token1 = Token::new(TokenKind::Let, span1, "let");
    let token2 = Token::new(TokenKind::Let, span2, "let");

    assert_eq!(token1, token2);
}

#[test]
fn test_token_inequality_kind() {
    let span = Span::new(0, 3, 1, 1);
    let token1 = Token::new(TokenKind::Let, span.clone(), "let");
    let token2 = Token::new(TokenKind::Const, span, "const");

    assert_ne!(token1, token2);
}

#[test]
fn test_token_inequality_span() {
    let span1 = Span::new(0, 3, 1, 1);
    let span2 = Span::new(10, 13, 2, 5);

    let token1 = Token::new(TokenKind::Let, span1, "let");
    let token2 = Token::new(TokenKind::Let, span2, "let");

    assert_ne!(token1, token2);
}

#[test]
fn test_token_kind_large_int() {
    let kind = TokenKind::IntLiteral(i64::MAX);
    assert!(kind.is_literal());
    assert_eq!(format!("{}", kind), i64::MAX.to_string());
}

#[test]
fn test_token_kind_negative_int() {
    let kind = TokenKind::IntLiteral(-42);
    assert_eq!(format!("{}", kind), "-42");
}

#[test]
fn test_token_kind_float_precision() {
    let kind = TokenKind::FloatLiteral(0.123456789);
    let display = format!("{}", kind);
    assert!(display.starts_with("0.123"));
}

#[test]
fn test_token_kind_empty_string() {
    let kind = TokenKind::StringLiteral("".to_string());
    assert!(kind.is_literal());
    assert_eq!(format!("{}", kind), "\"\"");
}

#[test]
fn test_token_kind_string_with_quotes() {
    let kind = TokenKind::StringLiteral("say \"hello\"".to_string());
    assert!(kind.is_literal());
}

#[test]
fn test_token_kind_multiline_doc_comment() {
    let kind = TokenKind::BlockDocComment("Line 1\nLine 2\nLine 3".to_string());
    assert!(kind.is_doc_comment());
}

#[test]
fn test_token_empty_lexeme() {
    let span = Span::new(0, 0, 1, 1);
    let token = Token::new(TokenKind::Eof, span, "");

    assert_eq!(token.lexeme, "");
}

#[test]
fn test_token_debug() {
    let span = Span::new(0, 5, 1, 1);
    let token = Token::new(TokenKind::IntLiteral(42), span, "42");

    let debug = format!("{:?}", token);
    assert!(debug.contains("IntLiteral"));
    assert!(debug.contains("42"));
}
