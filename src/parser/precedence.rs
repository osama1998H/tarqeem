//! Operator precedence for expression parsing

use crate::lexer::TokenKind;

/// Operator precedence levels (higher = binds tighter)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None = 0,
    Assignment = 1, // =, +=, -=, etc.
    Ternary = 2,    // ?:
    Or = 3,         // ||, أو
    And = 4,        // &&, و
    Equality = 5,   // ==, !=
    Comparison = 6, // <, >, <=, >=
    Term = 7,       // +, -
    Factor = 8,     // *, /, %
    Power = 9,      // **
    Unary = 10,     // -, !, ليس
    Call = 11,      // (), [], .
    Primary = 12,
}

impl Precedence {
    /// Get the precedence of a token
    pub fn of(kind: &TokenKind) -> Precedence {
        match kind {
            TokenKind::Equal
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual
            | TokenKind::PercentEqual => Precedence::Assignment,
            // Note: FatArrow (=>) is NOT an infix operator - it's handled by
            // parse_match_statement and try_parse_arrow_function directly
            TokenKind::Question => Precedence::Ternary,

            TokenKind::Or => Precedence::Or,

            TokenKind::And => Precedence::And,

            TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equality,

            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Precedence::Comparison,

            TokenKind::Plus | TokenKind::Minus => Precedence::Term,

            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,

            TokenKind::StarStar => Precedence::Power,

            TokenKind::Bang => Precedence::Unary,

            // Postfix operators (++, --) have high precedence
            TokenKind::PlusPlus | TokenKind::MinusMinus => Precedence::Call,

            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot => Precedence::Call,

            _ => Precedence::None,
        }
    }

    /// Get the next higher precedence level
    pub fn next(self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Ternary,
            Precedence::Ternary => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Power,
            Precedence::Power => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }

    /// Check if this operator is right-associative
    pub fn is_right_associative(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Equal
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::PercentEqual
                | TokenKind::StarStar
        )
    }
}
