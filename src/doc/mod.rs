//! Documentation Generator for Tarqeem (trqdoc)
//!
//! This module provides functionality to generate documentation from Tarqeem source files.
//! It supports extracting doc comments and generating output in HTML, Markdown, and JSON formats.

pub mod comment;
pub mod extractor;
pub mod generator;
pub mod model;

pub use comment::DocCommentParser;
pub use extractor::DocExtractor;
pub use generator::{generate_docs, HtmlGenerator, JsonGenerator, MarkdownGenerator, OutputFormat};
pub use model::{
    ClassDoc, DocItem, Documentation, FieldDoc, FunctionDoc, InterfaceDoc, MethodDoc, ParamDoc,
};
