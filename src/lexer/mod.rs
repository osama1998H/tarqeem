//! Lexical analysis for Tarqeem
//!
//! This module provides the lexer (tokenizer) which converts source code
//! into a stream of tokens. It handles:
//! - Full Unicode/UTF-8 support for Arabic characters
//! - Bidirectional text (Arabic RTL mixed with English/numbers LTR)
//! - Dual Arabic/English keyword recognition

mod keywords;
mod lexer;
mod token;

#[cfg(test)]
mod token_tests;

pub use keywords::KEYWORDS;
pub use lexer::Lexer;
pub use token::{Token, TokenKind};
