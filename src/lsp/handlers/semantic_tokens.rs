//! Semantic tokens handler
//!
//! Provides semantic syntax highlighting tokens.

use crate::error::Language;
use crate::lexer::TokenKind;
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::offset_to_position;
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensLegend,
    SemanticTokensResult,
};

pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,      // 0
    SemanticTokenType::TYPE,           // 1
    SemanticTokenType::CLASS,          // 2
    SemanticTokenType::ENUM,           // 3
    SemanticTokenType::INTERFACE,      // 4
    SemanticTokenType::STRUCT,         // 5
    SemanticTokenType::TYPE_PARAMETER, // 6
    SemanticTokenType::PARAMETER,      // 7
    SemanticTokenType::VARIABLE,       // 8
    SemanticTokenType::PROPERTY,       // 9
    SemanticTokenType::ENUM_MEMBER,    // 10
    SemanticTokenType::EVENT,          // 11
    SemanticTokenType::FUNCTION,       // 12
    SemanticTokenType::METHOD,         // 13
    SemanticTokenType::MACRO,          // 14
    SemanticTokenType::KEYWORD,        // 15
    SemanticTokenType::MODIFIER,       // 16
    SemanticTokenType::COMMENT,        // 17
    SemanticTokenType::STRING,         // 18
    SemanticTokenType::NUMBER,         // 19
    SemanticTokenType::REGEXP,         // 20
    SemanticTokenType::OPERATOR,       // 21
];

pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,     // 0
    SemanticTokenModifier::DEFINITION,      // 1
    SemanticTokenModifier::READONLY,        // 2
    SemanticTokenModifier::STATIC,          // 3
    SemanticTokenModifier::DEPRECATED,      // 4
    SemanticTokenModifier::ABSTRACT,        // 5
    SemanticTokenModifier::ASYNC,           // 6
    SemanticTokenModifier::MODIFICATION,    // 7
    SemanticTokenModifier::DOCUMENTATION,   // 8
    SemanticTokenModifier::DEFAULT_LIBRARY, // 9
];

pub fn get_semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

pub fn handle_semantic_tokens_full(
    doc: &mut DocumentState,
    language: Language,
) -> Option<SemanticTokensResult> {
    let content = doc.content.clone();
    let analysis = doc.get_analysis(language);

    let mut tokens: Vec<SemanticTokenData> = Vec::new();

    // Process all lexer tokens
    for token in &analysis.tokens {
        if let Some((token_type, modifiers)) = classify_token(&token.kind, &analysis.symbols) {
            let position = offset_to_position(&content, token.span.start);
            let length = token.lexeme.chars().count() as u32;

            tokens.push(SemanticTokenData {
                line: position.line,
                start_char: position.character,
                length,
                token_type,
                modifiers,
            });
        }
    }

    // Sort tokens by position (line, then character)
    tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));

    // Convert to LSP delta-encoded format
    let semantic_tokens = encode_tokens(&tokens);

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: semantic_tokens,
    }))
}

#[derive(Debug)]
struct SemanticTokenData {
    line: u32,
    start_char: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

fn classify_token(
    kind: &TokenKind,
    symbols: &std::collections::HashMap<String, crate::lsp::analysis::SymbolInfo>,
) -> Option<(u32, u32)> {
    use crate::lsp::analysis::SymbolKind;

    match kind {
        // Keywords
        TokenKind::Let
        | TokenKind::Const
        | TokenKind::Function
        | TokenKind::Return
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::While
        | TokenKind::For
        | TokenKind::In
        | TokenKind::Match
        | TokenKind::Case
        | TokenKind::Default
        | TokenKind::Do
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Class
        | TokenKind::Interface
        | TokenKind::Extends
        | TokenKind::Implements
        | TokenKind::Constructor
        | TokenKind::This
        | TokenKind::Super
        | TokenKind::New
        | TokenKind::Try
        | TokenKind::Catch
        | TokenKind::Finally
        | TokenKind::Throw
        | TokenKind::Import
        | TokenKind::Export
        | TokenKind::From
        | TokenKind::As
        | TokenKind::Async
        | TokenKind::Await => Some((15, 0)), // KEYWORD

        // Access modifiers
        TokenKind::Public | TokenKind::Private | TokenKind::Protected | TokenKind::Static => {
            Some((16, 0)) // MODIFIER
        }

        // Type keywords
        TokenKind::TypeInt
        | TokenKind::TypeFloat
        | TokenKind::TypeString
        | TokenKind::TypeBool
        | TokenKind::TypeArray
        | TokenKind::TypeMap
        | TokenKind::TypeVoid
        | TokenKind::TypeAny => Some((1, 0)), // TYPE

        // Literals
        TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_) => Some((19, 0)), // NUMBER
        TokenKind::StringLiteral(_) => Some((18, 0)),                           // STRING
        TokenKind::True | TokenKind::False | TokenKind::Null => Some((15, 0)), // KEYWORD (boolean/null literals)

        // Identifiers - classify based on symbol table
        TokenKind::Identifier(name) => {
            if let Some(info) = symbols.get(name) {
                let modifiers = compute_modifiers(info);
                match info.kind {
                    SymbolKind::Variable => Some((8, modifiers)), // VARIABLE
                    SymbolKind::Function => Some((12, modifiers)), // FUNCTION
                    SymbolKind::Class => Some((2, modifiers)),    // CLASS
                    SymbolKind::Interface => Some((4, modifiers)), // INTERFACE
                    SymbolKind::Parameter => Some((7, modifiers)), // PARAMETER
                    SymbolKind::Field => Some((9, modifiers)),    // PROPERTY
                    SymbolKind::Method => Some((13, modifiers)),  // METHOD
                    SymbolKind::Property => Some((9, modifiers)), // PROPERTY (same as Field)
                }
            } else if is_builtin_function(name) {
                Some((12, 1 << 9)) // FUNCTION with DEFAULT_LIBRARY modifier
            } else {
                Some((8, 0)) // Default to VARIABLE
            }
        }

        // Operators
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::StarStar
        | TokenKind::EqualEqual
        | TokenKind::BangEqual
        | TokenKind::Less
        | TokenKind::LessEqual
        | TokenKind::Greater
        | TokenKind::GreaterEqual
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Bang
        | TokenKind::Equal
        | TokenKind::PlusEqual
        | TokenKind::MinusEqual
        | TokenKind::StarEqual
        | TokenKind::SlashEqual
        | TokenKind::PercentEqual
        | TokenKind::PlusPlus
        | TokenKind::MinusMinus => Some((21, 0)), // OPERATOR

        // Skip delimiters, newlines, errors, EOF
        _ => None,
    }
}

fn compute_modifiers(info: &crate::lsp::analysis::SymbolInfo) -> u32 {
    let mut modifiers = 0u32;

    // DECLARATION (bit 0)
    modifiers |= 1 << 0;

    // READONLY (bit 2) for immutable variables
    if !info.mutable {
        modifiers |= 1 << 2;
    }

    modifiers
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "اطبع"
            | "print"
            | "println"
            | "اطبع_سطر"
            | "ادخل"
            | "input"
            | "طول"
            | "len"
            | "length"
            | "نوع"
            | "type"
            | "typeof"
            | "عدد"
            | "int"
            | "عدد_عشري"
            | "float"
            | "نص"
            | "str"
            | "string"
            | "منطقي"
            | "bool"
            | "مطلق"
            | "abs"
            | "جذر"
            | "sqrt"
            | "قوة"
            | "pow"
            | "اقرأ_ملف"
            | "read_file"
            | "اكتب_ملف"
            | "write_file"
    )
}

fn encode_tokens(tokens: &[SemanticTokenData]) -> Vec<SemanticToken> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for token in tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start_char - prev_char
        } else {
            token.start_char
        };

        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: token.modifiers,
        });

        prev_line = token.line;
        prev_char = token.start_char;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_semantic_tokens_legend() {
        let legend = get_semantic_tokens_legend();
        assert!(!legend.token_types.is_empty());
        assert!(!legend.token_modifiers.is_empty());
        assert!(legend.token_types.contains(&SemanticTokenType::KEYWORD));
        assert!(legend.token_types.contains(&SemanticTokenType::VARIABLE));
    }

    #[test]
    fn test_classify_keyword() {
        let symbols = std::collections::HashMap::new();
        assert_eq!(classify_token(&TokenKind::Let, &symbols), Some((15, 0)));
        assert_eq!(
            classify_token(&TokenKind::Function, &symbols),
            Some((15, 0))
        );
        assert_eq!(classify_token(&TokenKind::If, &symbols), Some((15, 0)));
    }

    #[test]
    fn test_classify_type() {
        let symbols = std::collections::HashMap::new();
        assert_eq!(classify_token(&TokenKind::TypeInt, &symbols), Some((1, 0)));
        assert_eq!(
            classify_token(&TokenKind::TypeString, &symbols),
            Some((1, 0))
        );
    }

    #[test]
    fn test_classify_literal() {
        let symbols = std::collections::HashMap::new();
        assert_eq!(
            classify_token(&TokenKind::IntLiteral(42), &symbols),
            Some((19, 0))
        );
        assert_eq!(
            classify_token(&TokenKind::StringLiteral("test".into()), &symbols),
            Some((18, 0))
        );
    }

    #[test]
    fn test_is_builtin_function() {
        assert!(is_builtin_function("اطبع"));
        assert!(is_builtin_function("print"));
        assert!(is_builtin_function("طول"));
        assert!(is_builtin_function("len"));
        assert!(!is_builtin_function("custom_func"));
    }

    #[test]
    fn test_encode_tokens() {
        let tokens = vec![
            SemanticTokenData {
                line: 0,
                start_char: 0,
                length: 5,
                token_type: 15,
                modifiers: 0,
            },
            SemanticTokenData {
                line: 0,
                start_char: 6,
                length: 1,
                token_type: 8,
                modifiers: 0,
            },
            SemanticTokenData {
                line: 1,
                start_char: 0,
                length: 3,
                token_type: 15,
                modifiers: 0,
            },
        ];

        let encoded = encode_tokens(&tokens);
        assert_eq!(encoded.len(), 3);

        // First token: delta_line=0, delta_start=0
        assert_eq!(encoded[0].delta_line, 0);
        assert_eq!(encoded[0].delta_start, 0);

        // Second token: same line, delta_start=6
        assert_eq!(encoded[1].delta_line, 0);
        assert_eq!(encoded[1].delta_start, 6);

        // Third token: new line, delta_line=1, delta_start=0
        assert_eq!(encoded[2].delta_line, 1);
        assert_eq!(encoded[2].delta_start, 0);
    }

    #[test]
    fn test_semantic_tokens_full() {
        let content = "متغير س = 5".to_string();
        let mut doc = DocumentState::new(uri, 1, content);

        let result = handle_semantic_tokens_full(&mut doc, Language::Arabic);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            assert!(!tokens.data.is_empty());
        }
    }
}
