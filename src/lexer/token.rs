//! Token definitions for Tarqeem

use crate::error::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            lexeme: lexeme.into(),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({})", self.kind, self.lexeme)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    True,
    False,
    Null,

    Identifier(String),

    Let,
    Const,

    Function,
    Return,
    Async,
    Await,

    If,
    Else,
    Match,
    Case,
    Default,

    While,
    For,
    In,
    Do,
    Break,
    Continue,

    Class,
    Interface,
    Extends,
    Implements,
    Public,
    Private,
    Protected,
    Static,
    Constructor,
    This,
    Super,
    New,
    Enum,

    Property,
    Get,
    Set,

    Try,
    Catch,
    Finally,
    Throw,

    Import,
    Export,
    From,
    As,

    TypeInt,
    TypeFloat,
    TypeString,
    TypeBool,
    TypeArray,
    TypeMap,
    TypeVoid,
    TypeAny,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,

    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    And,
    Or,
    Bang,

    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,

    PlusPlus,
    MinusMinus,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Arrow,
    FatArrow,
    Question,

    ArabicComma,
    ArabicSemicolon,

    /// Line comment (// ...)
    LineComment(String),
    DocComment(String),
    BlockDocComment(String),

    Bismillah,
    Alhamdulillah,

    Eof,
    Newline,
    Error(String),
}

impl TokenKind {
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
        )
    }

    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Let
                | TokenKind::Const
                | TokenKind::Function
                | TokenKind::Return
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::For
                | TokenKind::In
                | TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Extends
                | TokenKind::Implements
                | TokenKind::Public
                | TokenKind::Private
                | TokenKind::Protected
                | TokenKind::Static
                | TokenKind::Constructor
                | TokenKind::This
                | TokenKind::Super
                | TokenKind::New
                | TokenKind::Enum
                | TokenKind::Property
                | TokenKind::Get
                | TokenKind::Set
                | TokenKind::Try
                | TokenKind::Catch
                | TokenKind::Finally
                | TokenKind::Throw
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::From
                | TokenKind::As
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Match
                | TokenKind::Case
                | TokenKind::Default
                | TokenKind::Do
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Bismillah
                | TokenKind::Alhamdulillah
        )
    }

    pub fn is_type_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::TypeInt
                | TokenKind::TypeFloat
                | TokenKind::TypeString
                | TokenKind::TypeBool
                | TokenKind::TypeArray
                | TokenKind::TypeMap
                | TokenKind::TypeVoid
                | TokenKind::TypeAny
        )
    }

    pub fn is_doc_comment(&self) -> bool {
        matches!(
            self,
            TokenKind::DocComment(_) | TokenKind::BlockDocComment(_)
        )
    }

    pub fn is_line_comment(&self) -> bool {
        matches!(self, TokenKind::LineComment(_))
    }

    pub fn is_comment(&self) -> bool {
        matches!(
            self,
            TokenKind::LineComment(_) | TokenKind::DocComment(_) | TokenKind::BlockDocComment(_)
        )
    }

    pub fn is_operator(&self) -> bool {
        matches!(
            self,
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
                | TokenKind::MinusMinus
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::IntLiteral(n) => write!(f, "{}", n),
            TokenKind::FloatLiteral(n) => write!(f, "{}", n),
            TokenKind::StringLiteral(s) => write!(f, "\"{}\"", s),
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::Error(s) => write!(f, "Error: {}", s),
            _ => write!(f, "{:?}", self),
        }
    }
}
