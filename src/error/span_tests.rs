//! Comprehensive tests for the Span module
//!
//! These tests verify source location tracking functionality.

use super::span::*;

#[test]
fn test_span_new() {
    let span = Span::new(10, 20, 5, 3);

    assert_eq!(span.start, 10);
    assert_eq!(span.end, 20);
    assert_eq!(span.line, 5);
    assert_eq!(span.column, 3);
}

#[test]
fn test_span_empty() {
    let span = Span::empty();

    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
    assert_eq!(span.line, 0);
    assert_eq!(span.column, 0);
}

#[test]
fn test_span_default() {
    let span = Span::default();

    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
    assert_eq!(span.line, 0);
    assert_eq!(span.column, 0);
}

#[test]
fn test_span_len() {
    let span = Span::new(10, 20, 1, 1);
    assert_eq!(span.len(), 10);
}

#[test]
fn test_span_len_zero() {
    let span = Span::new(5, 5, 1, 1);
    assert_eq!(span.len(), 0);
}

#[test]
fn test_span_len_single_char() {
    let span = Span::new(0, 1, 1, 1);
    assert_eq!(span.len(), 1);
}

#[test]
fn test_span_len_large() {
    let span = Span::new(0, 10000, 1, 1);
    assert_eq!(span.len(), 10000);
}

#[test]
fn test_span_is_empty_true() {
    let span = Span::new(5, 5, 1, 1);
    assert!(span.is_empty());
}

#[test]
fn test_span_is_empty_false() {
    let span = Span::new(5, 10, 1, 1);
    assert!(!span.is_empty());
}

#[test]
fn test_span_empty_is_empty() {
    let span = Span::empty();
    assert!(span.is_empty());
}

#[test]
fn test_span_merge_consecutive() {
    let span1 = Span::new(0, 10, 1, 1);
    let span2 = Span::new(10, 20, 1, 11);

    let merged = span1.merge(&span2);

    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 20);
    assert_eq!(merged.line, 1);
    assert_eq!(merged.column, 1);
}

#[test]
fn test_span_merge_overlapping() {
    let span1 = Span::new(5, 15, 1, 6);
    let span2 = Span::new(10, 25, 1, 11);

    let merged = span1.merge(&span2);

    assert_eq!(merged.start, 5);
    assert_eq!(merged.end, 25);
}

#[test]
fn test_span_merge_gap() {
    let span1 = Span::new(0, 10, 1, 1);
    let span2 = Span::new(20, 30, 1, 21);

    let merged = span1.merge(&span2);

    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 30);
}

#[test]
fn test_span_merge_contained() {
    let outer = Span::new(0, 100, 1, 1);
    let inner = Span::new(20, 30, 1, 21);

    let merged = outer.merge(&inner);

    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 100);
}

#[test]
fn test_span_merge_same_span() {
    let span = Span::new(10, 20, 2, 5);
    let merged = span.merge(&span);

    assert_eq!(merged.start, 10);
    assert_eq!(merged.end, 20);
    assert_eq!(merged.line, 2);
    assert_eq!(merged.column, 5);
}

#[test]
fn test_span_merge_different_lines() {
    let span1 = Span::new(0, 50, 1, 1);
    let span2 = Span::new(100, 150, 5, 10);

    let merged = span1.merge(&span2);

    assert_eq!(merged.line, 1);
    assert_eq!(merged.column, 1);
    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 150);
}

#[test]
fn test_span_merge_second_earlier() {
    let span1 = Span::new(50, 100, 5, 1);
    let span2 = Span::new(0, 30, 1, 1);

    let merged = span1.merge(&span2);

    assert_eq!(merged.line, 1);
    assert_eq!(merged.column, 1);
    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 100);
}

#[test]
fn test_span_merge_empty_with_non_empty() {
    let empty = Span::empty();
    let non_empty = Span::new(10, 20, 2, 5);

    let merged = empty.merge(&non_empty);

    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 20);
}

#[test]
fn test_span_display_simple() {
    let span = Span::new(0, 10, 1, 1);
    assert_eq!(format!("{}", span), "1:1");
}

#[test]
fn test_span_display_multidigit() {
    let span = Span::new(0, 100, 42, 15);
    assert_eq!(format!("{}", span), "42:15");
}

#[test]
fn test_span_display_zero() {
    let span = Span::empty();
    assert_eq!(format!("{}", span), "0:0");
}

#[test]
fn test_span_display_large_numbers() {
    let span = Span::new(0, 10000, 9999, 999);
    assert_eq!(format!("{}", span), "9999:999");
}

#[test]
fn test_span_equality() {
    let span1 = Span::new(10, 20, 5, 3);
    let span2 = Span::new(10, 20, 5, 3);

    assert_eq!(span1, span2);
}

#[test]
fn test_span_inequality_start() {
    let span1 = Span::new(10, 20, 5, 3);
    let span2 = Span::new(11, 20, 5, 3);

    assert_ne!(span1, span2);
}

#[test]
fn test_span_inequality_end() {
    let span1 = Span::new(10, 20, 5, 3);
    let span2 = Span::new(10, 21, 5, 3);

    assert_ne!(span1, span2);
}

#[test]
fn test_span_inequality_line() {
    let span1 = Span::new(10, 20, 5, 3);
    let span2 = Span::new(10, 20, 6, 3);

    assert_ne!(span1, span2);
}

#[test]
fn test_span_inequality_column() {
    let span1 = Span::new(10, 20, 5, 3);
    let span2 = Span::new(10, 20, 5, 4);

    assert_ne!(span1, span2);
}

#[test]
fn test_span_clone() {
    let original = Span::new(10, 20, 5, 3);
    let cloned = original; // Span is Copy

    assert_eq!(original, cloned);
}

#[test]
fn test_span_clone_independence() {
    let original = Span::new(10, 20, 5, 3);
    let cloned = original;

    assert_eq!(original.start, cloned.start);
}

#[test]
fn test_span_debug() {
    let span = Span::new(10, 20, 5, 3);
    let debug_str = format!("{:?}", span);

    assert!(debug_str.contains("Span"));
    assert!(debug_str.contains("10"));
    assert!(debug_str.contains("20"));
    assert!(debug_str.contains("5"));
    assert!(debug_str.contains("3"));
}

#[test]
fn test_span_max_values() {
    let span = Span::new(usize::MAX - 1, usize::MAX, usize::MAX, usize::MAX);

    assert_eq!(span.start, usize::MAX - 1);
    assert_eq!(span.end, usize::MAX);
    assert_eq!(span.len(), 1);
}

#[test]
fn test_span_single_byte() {
    let span = Span::new(42, 43, 10, 5);

    assert_eq!(span.len(), 1);
    assert!(!span.is_empty());
}

#[test]
fn test_span_first_position() {
    let span = Span::new(0, 5, 1, 1);

    assert_eq!(span.line, 1);
    assert_eq!(span.column, 1);
    assert_eq!(format!("{}", span), "1:1");
}

#[test]
fn test_span_arabic_identifier() {
    let span = Span::new(0, 12, 1, 1); // 6 chars * 2 bytes = 12 bytes

    assert_eq!(span.len(), 12);
    assert!(!span.is_empty());
}

#[test]
fn test_span_mixed_content() {
    let span = Span::new(0, 14, 1, 1);

    assert_eq!(span.len(), 14);
}

#[test]
fn test_span_token_positions() {
    let var_keyword_span = Span::new(0, 10, 1, 1); // "متغير"
    let var_name_span = Span::new(11, 13, 1, 7); // "س"
    let equals_span = Span::new(14, 15, 1, 9); // "="
    let number_span = Span::new(16, 17, 1, 11); // "5"

    let full_span = var_keyword_span
        .merge(&var_name_span)
        .merge(&equals_span)
        .merge(&number_span);

    assert_eq!(full_span.start, 0);
    assert_eq!(full_span.end, 17);
}

#[test]
fn test_span_multiline_expression() {
    let line1 = Span::new(0, 20, 1, 1);
    let line2 = Span::new(21, 40, 2, 1);
    let line3 = Span::new(41, 60, 3, 1);

    let full_span = line1.merge(&line2).merge(&line3);

    assert_eq!(full_span.line, 1);
    assert_eq!(full_span.start, 0);
    assert_eq!(full_span.end, 60);
}

#[test]
fn test_span_error_underline_calculation() {
    let span = Span::new(10, 25, 3, 5);

    let underline_start = span.column - 1; // 0-indexed for spacing
    let underline_length = span.len();

    assert_eq!(underline_start, 4);
    assert_eq!(underline_length, 15);
}
