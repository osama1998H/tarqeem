//! Abstract Syntax Tree (AST) definitions for Tarqeem

use crate::error::Span;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Ast {
    pub statements: Vec<Stmt>,
    pub bismillah_span: Option<Span>,
    pub alhamdulillah_span: Option<Span>,
}

impl Ast {
    pub fn new(statements: Vec<Stmt>) -> Self {
        Self {
            statements,
            bismillah_span: None,
            alhamdulillah_span: None,
        }
    }

    pub fn with_markers(
        statements: Vec<Stmt>,
        bismillah_span: Span,
        alhamdulillah_span: Span,
    ) -> Self {
        Self {
            statements,
            bismillah_span: Some(bismillah_span),
            alhamdulillah_span: Some(alhamdulillah_span),
        }
    }

    pub fn has_file_markers(&self) -> bool {
        self.bismillah_span.is_some() && self.alhamdulillah_span.is_some()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
    /// Line comments that appear before this statement
    pub leading_comments: Vec<String>,
    /// Line comment that appears after this statement on the same line
    pub trailing_comment: Option<String>,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self {
            kind,
            span,
            leading_comments: Vec::new(),
            trailing_comment: None,
        }
    }

    pub fn with_comments(kind: StmtKind, span: Span, comments: Vec<String>) -> Self {
        Self {
            kind,
            span,
            leading_comments: comments,
            trailing_comment: None,
        }
    }

    pub fn with_trailing_comment(mut self, comment: Option<String>) -> Self {
        self.trailing_comment = comment;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum StmtKind {
    VarDecl {
        name: String,
        mutable: bool,
        ty: Option<TypeAnnotation>,
        init: Option<Expr>,
        doc_comment: Option<String>,
    },

    FuncDecl {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Block,
        is_async: bool,
        doc_comment: Option<String>,
    },

    ClassDecl {
        name: String,
        type_params: Vec<String>,
        extends: Option<String>,
        implements: Vec<String>,
        members: Vec<ClassMember>,
        doc_comment: Option<String>,
    },

    InterfaceDecl {
        name: String,
        type_params: Vec<String>,
        methods: Vec<MethodSignature>,
        doc_comment: Option<String>,
    },

    EnumDecl {
        name: String,
        type_params: Vec<String>,
        variants: Vec<EnumVariant>,
        doc_comment: Option<String>,
    },

    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },

    While {
        condition: Expr,
        body: Block,
    },

    DoWhile {
        body: Block,
        condition: Expr,
    },

    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        update: Option<Expr>,
        body: Block,
    },

    ForIn {
        variable: String,
        iterable: Expr,
        body: Block,
    },

    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },

    Return(Option<Expr>),

    Break,

    Continue,

    Try {
        body: Block,
        catch: Option<CatchClause>,
        finally: Option<Block>,
    },

    Throw(Expr),

    Import {
        items: ImportItems,
        from: String,
    },

    Export(Box<Stmt>),

    Expr(Expr),

    Block(Block),
}

#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ExprKind {
    Literal(Literal),

    Identifier(String),

    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    Member {
        object: Box<Expr>,
        property: String,
    },

    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    CompoundAssignment {
        target: Box<Expr>,
        op: BinaryOp,
        value: Box<Expr>,
    },

    Array(Vec<Expr>),

    Object(Vec<(String, Expr)>),

    Lambda {
        params: Vec<Param>,
        body: LambdaBody,
    },

    New {
        class: Box<Expr>,
        type_args: Vec<TypeAnnotation>,
        args: Vec<Expr>,
    },

    Await(Box<Expr>),

    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    Grouping(Box<Expr>),

    This,

    Super,

    /// Enum variant instantiation: `لون::أحمر` or `نتيجة::نجاح(42)`
    EnumVariant {
        /// The enum type name (e.g., "لون", "نتيجة")
        enum_name: String,
        /// Optional type arguments for generic enums (e.g., `اختياري<عدد>`)
        type_args: Vec<TypeAnnotation>,
        /// The variant name (e.g., "أحمر", "نجاح")
        variant_name: String,
        /// Associated data arguments (empty for unit variants)
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    And,
    Or,
}

impl BinaryOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Self::Add),
            "-" => Some(Self::Sub),
            "*" => Some(Self::Mul),
            "/" => Some(Self::Div),
            "%" => Some(Self::Mod),
            "**" => Some(Self::Pow),
            "==" => Some(Self::Eq),
            "!=" => Some(Self::NotEq),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::LtEq),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::GtEq),
            "&&" | "و" => Some(Self::And),
            "||" | "أو" | "او" => Some(Self::Or),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UnaryOp {
    Neg,     // -x
    Not,     // !x, ليس x
    PreInc,  // ++x
    PreDec,  // --x
    PostInc, // x++
    PostDec, // x--
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn new(statements: Vec<Stmt>, span: Span) -> Self {
        Self { statements, span }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeAnnotation>,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeAnnotation {
    pub kind: TypeKind,
    pub span: Span,
}

impl TypeAnnotation {
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum TypeKind {
    Simple(String),

    Array(Box<TypeAnnotation>),

    Map(Box<TypeAnnotation>, Box<TypeAnnotation>),

    Function {
        params: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },

    Generic {
        base: String,
        args: Vec<TypeAnnotation>,
    },

    Optional(Box<TypeAnnotation>),
}

#[derive(Debug, Clone, Serialize)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchArm {
    pub patterns: Vec<Pattern>,
    pub body: Block,
    pub span: Span,
}

/// Pattern for match expressions
/// نمط للمطابقة في تعبيرات تطابق
#[derive(Debug, Clone, Serialize)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Check if this pattern is a wildcard/default
    pub fn is_wildcard(&self) -> bool {
        matches!(self.kind, PatternKind::Wildcard)
    }

    /// Check if this pattern is an enum variant
    pub fn is_enum_variant(&self) -> bool {
        matches!(self.kind, PatternKind::EnumVariant { .. })
    }
}

/// Pattern kinds for match expressions
#[derive(Debug, Clone, Serialize)]
pub enum PatternKind {
    /// Literal value pattern (numbers, strings, booleans)
    /// نمط القيمة الحرفية (أرقام، نصوص، قيم منطقية)
    Literal(Expr),

    /// Identifier pattern - binds value to a name
    /// نمط المعرّف - يربط القيمة باسم
    Identifier(String),

    /// Wildcard pattern - matches anything (غير_ذلك / default)
    /// نمط البديل - يطابق أي شيء
    Wildcard,

    /// Enum variant pattern with optional bindings
    /// نمط حالة التعداد مع ربط اختياري
    EnumVariant {
        enum_name: String,
        variant_name: String,
        bindings: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct CatchClause {
    pub param: String,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub enum ImportItems {
    Named(Vec<ImportItem>),
    Wildcard(String),
    Default(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum PropertyAccessor {
    Get {
        visibility: Visibility,
        body: PropertyAccessorBody,
    },
    Set {
        visibility: Visibility,
        param_name: String,
        body: Block,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum PropertyAccessorBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone, Serialize)]
pub enum ClassMember {
    Field {
        visibility: Visibility,
        name: String,
        ty: Option<TypeAnnotation>,
        init: Option<Expr>,
        is_static: bool,
        doc_comment: Option<String>,
    },

    Method {
        visibility: Visibility,
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Block,
        is_static: bool,
        is_async: bool,
        doc_comment: Option<String>,
    },

    Constructor {
        params: Vec<Param>,
        body: Block,
        doc_comment: Option<String>,
    },

    Property {
        visibility: Visibility,
        name: String,
        ty: TypeAnnotation,
        accessors: Vec<PropertyAccessor>,
        default_value: Option<Expr>,
        is_static: bool,
        doc_comment: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumVariant {
    pub name: String,
    pub discriminant: Option<i64>,
    pub fields: Vec<EnumVariantField>,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumVariantField {
    pub name: Option<String>,
    pub ty: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_creation() {
        let ast = Ast::new(vec![Stmt::new(
            StmtKind::VarDecl {
                name: "x".to_string(),
                mutable: true,
                ty: None,
                init: Some(Expr::new(ExprKind::Literal(Literal::Int(5)), Span::empty())),
                doc_comment: None,
            },
            Span::empty(),
        )]);

        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_ast_with_doc_comment() {
        let ast = Ast::new(vec![Stmt::new(
            StmtKind::VarDecl {
                name: "س".to_string(),
                mutable: true,
                ty: None,
                init: Some(Expr::new(ExprKind::Literal(Literal::Int(5)), Span::empty())),
                doc_comment: Some("متغير للاختبار".to_string()),
            },
            Span::empty(),
        )]);

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl { doc_comment, .. } => {
                assert_eq!(doc_comment.as_deref(), Some("متغير للاختبار"));
            }
            _ => panic!("Expected VarDecl"),
        }
    }
}
