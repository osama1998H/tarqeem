//! Token definitions for Tarqeem

use crate::error::Span;
use std::fmt;

/// A token produced by the lexer
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

/// The kind of token
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ============ Literals ============
    /// Integer literal: 42, ٤٢
    IntLiteral(i64),
    /// Float literal: 3.14, ٣.١٤
    FloatLiteral(f64),
    /// String literal: "مرحبا", 'hello'
    StringLiteral(String),
    /// Boolean true: صحيح, true
    True,
    /// Boolean false: خطأ, false
    False,
    /// Null: عدم, null
    Null,

    // ============ Identifiers ============
    /// Identifier (variable/function name)
    Identifier(String),

    // ============ Keywords - Variables ============
    /// متغير / let - mutable variable
    Let,
    /// ثابت / const - immutable constant
    Const,

    // ============ Keywords - Functions ============
    /// دالة / function
    Function,
    /// أرجع / return
    Return,
    /// غير_متزامن / async
    Async,
    /// انتظر / await
    Await,

    // ============ Keywords - Control Flow ============
    /// إذا / if
    If,
    /// وإلا / else
    Else,
    /// تطابق / match
    Match,
    /// حالة / case
    Case,
    /// غير_ذلك / default
    Default,

    // ============ Keywords - Loops ============
    /// طالما / while
    While,
    /// لكل / for
    For,
    /// في / in
    In,
    /// افعل / do
    Do,
    /// أوقف / break
    Break,
    /// استمر / continue
    Continue,

    // ============ Keywords - OOP ============
    /// صنف / class
    Class,
    /// ميثاق / interface
    Interface,
    /// يرث / extends
    Extends,
    /// يلتزم / implements
    Implements,
    /// عام / public
    Public,
    /// خاص / private
    Private,
    /// محمي / protected
    Protected,
    /// مشترك / static
    Static,
    /// منشئ / constructor
    Constructor,
    /// هذا / this
    This,
    /// أساس / super
    Super,
    /// جديد / new
    New,

    // ============ Keywords - Error Handling ============
    /// حاول / try
    Try,
    /// التقط / catch
    Catch,
    /// أخيراً / finally
    Finally,
    /// ارمِ / throw
    Throw,

    // ============ Keywords - Modules ============
    /// استورد / import
    Import,
    /// صدّر / export
    Export,
    /// من / from
    From,
    /// كـ / as
    As,

    // ============ Type Keywords ============
    /// عدد / int
    TypeInt,
    /// عدد_عشري / float
    TypeFloat,
    /// نص / string
    TypeString,
    /// منطقي / bool
    TypeBool,
    /// مصفوفة / array
    TypeArray,
    /// قاموس / map
    TypeMap,
    /// Internal void type (no keyword - functions default to no return)
    TypeVoid,
    /// أي / any
    TypeAny,

    // ============ Operators - Arithmetic ============
    /// +
    Plus,
    /// -
    Minus,
    /// *
    Star,
    /// /
    Slash,
    /// %
    Percent,
    /// **
    StarStar,

    // ============ Operators - Comparison ============
    /// ==
    EqualEqual,
    /// !=
    BangEqual,
    /// <
    Less,
    /// <=
    LessEqual,
    /// >
    Greater,
    /// >=
    GreaterEqual,

    // ============ Operators - Logical ============
    /// && / و
    And,
    /// || / أو
    Or,
    /// ! / ليس
    Bang,

    // ============ Operators - Assignment ============
    /// =
    Equal,
    /// +=
    PlusEqual,
    /// -=
    MinusEqual,
    /// *=
    StarEqual,
    /// /=
    SlashEqual,
    /// %=
    PercentEqual,

    // ============ Operators - Increment/Decrement ============
    /// ++
    PlusPlus,
    /// --
    MinusMinus,

    // ============ Delimiters ============
    /// (
    LeftParen,
    /// )
    RightParen,
    /// {
    LeftBrace,
    /// }
    RightBrace,
    /// [
    LeftBracket,
    /// ]
    RightBracket,
    /// ,
    Comma,
    /// . (dot)
    Dot,
    /// :
    Colon,
    /// ;
    Semicolon,
    /// ->
    Arrow,
    /// =>
    FatArrow,
    /// ?
    Question,

    // ============ Arabic Punctuation ============
    /// ، (Arabic comma)
    ArabicComma,
    /// ؛ (Arabic semicolon)
    ArabicSemicolon,

    // ============ Documentation Comments ============
    /// Single-line doc comment: /// comment
    DocComment(String),
    /// Block doc comment: /** comment */
    BlockDocComment(String),

    // ============ File Markers ============
    /// بسم_الله / bismillah - File start marker (In the name of God)
    Bismillah,
    /// الحمد_لله / alhamdulillah - File end marker (Praise be to God)
    Alhamdulillah,

    // ============ Special ============
    /// End of file
    Eof,
    /// Newline (may be significant in some contexts)
    Newline,
    /// Error token
    Error(String),
}

impl TokenKind {
    /// Check if this token is a literal
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

    /// Check if this token is a keyword
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

    /// Check if this token is a type keyword
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

    /// Check if this token is a documentation comment
    pub fn is_doc_comment(&self) -> bool {
        matches!(
            self,
            TokenKind::DocComment(_) | TokenKind::BlockDocComment(_)
        )
    }

    /// Check if this token is an operator
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
