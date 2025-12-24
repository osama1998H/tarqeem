//! Documentation generators for Tarqeem
//!
//! This module provides generators that convert documentation models
//! into various output formats (HTML, Markdown, JSON).

mod html;
mod json;
mod markdown;

pub use html::HtmlGenerator;
pub use json::JsonGenerator;
pub use markdown::MarkdownGenerator;

use super::model::Documentation;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Html,
    Markdown,
    Json,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Html => "html",
            OutputFormat::Markdown => "md",
            OutputFormat::Json => "json",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "html" | "htm" => Some(OutputFormat::Html),
            "markdown" | "md" => Some(OutputFormat::Markdown),
            "json" => Some(OutputFormat::Json),
            _ => None,
        }
    }
}

pub trait DocGenerator {
    fn generate(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()>;
}

pub fn generate_docs(
    doc: &Documentation,
    format: OutputFormat,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Html => HtmlGenerator::new().generate(doc, writer),
        OutputFormat::Markdown => MarkdownGenerator::new().generate(doc, writer),
        OutputFormat::Json => JsonGenerator::new().generate(doc, writer),
    }
}
