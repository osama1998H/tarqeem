//! Parser for Tarqeem
//!
//! This module provides a recursive descent parser that converts a stream
//! of tokens into an Abstract Syntax Tree (AST).

mod ast;
mod parser;
mod precedence;

#[cfg(test)]
mod parser_tests;

pub use ast::*;
pub use parser::Parser;
pub use precedence::Precedence;
