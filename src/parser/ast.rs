//! Abstract Syntax Tree (AST) definitions for Tarqeem

use crate::error::Span;

/// A complete Tarqeem program
#[derive(Debug, Clone)]
pub struct Ast {
    pub statements: Vec<Stmt>,
    /// Span of the بسم_الله file start marker
    pub bismillah_span: Option<Span>,
    /// Span of the الحمد_لله file end marker
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

    /// Create an AST with file markers
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

    /// Check if this AST has proper file markers
    pub fn has_file_markers(&self) -> bool {
        self.bismillah_span.is_some() && self.alhamdulillah_span.is_some()
    }
}

/// A statement in the AST
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of statement
#[derive(Debug, Clone)]
pub enum StmtKind {
    /// Variable declaration: متغير x = 5
    VarDecl {
        name: String,
        mutable: bool,
        ty: Option<TypeAnnotation>,
        init: Option<Expr>,
        /// Documentation comment attached to this declaration
        doc_comment: Option<String>,
    },

    /// Function declaration: دالة foo(x: عدد) -> عدد { ... }
    FuncDecl {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Block,
        is_async: bool,
        /// Documentation comment attached to this declaration
        doc_comment: Option<String>,
    },

    /// Class declaration: صنف Person<T> { ... }
    ClassDecl {
        name: String,
        type_params: Vec<String>,
        extends: Option<String>,
        implements: Vec<String>,
        members: Vec<ClassMember>,
        /// Documentation comment attached to this declaration
        doc_comment: Option<String>,
    },

    /// Interface declaration: ميثاق Printable<T> { ... }
    InterfaceDecl {
        name: String,
        type_params: Vec<String>,
        methods: Vec<MethodSignature>,
        /// Documentation comment attached to this declaration
        doc_comment: Option<String>,
    },

    /// If statement: إذا (cond) { ... } وإلا { ... }
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },

    /// While loop: طالما (cond) { ... }
    While { condition: Expr, body: Block },

    /// Do-while loop: افعل { ... } طالما (cond)
    DoWhile { body: Block, condition: Expr },

    /// For loop: لكل (init; cond; update) { ... }
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        update: Option<Expr>,
        body: Block,
    },

    /// For-in loop: لكل item في collection { ... }
    ForIn {
        variable: String,
        iterable: Expr,
        body: Block,
    },

    /// Match statement: تطابق (expr) { ... }
    Match { expr: Expr, arms: Vec<MatchArm> },

    /// Return statement: أرجع expr
    Return(Option<Expr>),

    /// Break statement: أوقف
    Break,

    /// Continue statement: استمر
    Continue,

    /// Try-catch statement: حاول { ... } التقط { ... }
    Try {
        body: Block,
        catch: Option<CatchClause>,
        finally: Option<Block>,
    },

    /// Throw statement: ارمِ expr
    Throw(Expr),

    /// Import statement: استورد { x, y } من "module"
    Import { items: ImportItems, from: String },

    /// Export statement: صدّر ...
    Export(Box<Stmt>),

    /// Expression statement
    Expr(Expr),

    /// Block statement
    Block(Block),
}

/// An expression in the AST
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of expression
#[derive(Debug, Clone)]
pub enum ExprKind {
    /// Literal value
    Literal(Literal),

    /// Identifier
    Identifier(String),

    /// Binary operation: a + b
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Unary operation: -x, !x
    Unary { op: UnaryOp, operand: Box<Expr> },

    /// Function call: foo(a, b)
    Call { callee: Box<Expr>, args: Vec<Expr> },

    /// Member access: obj.property
    Member { object: Box<Expr>, property: String },

    /// Index access: arr[0]
    Index { object: Box<Expr>, index: Box<Expr> },

    /// Assignment: x = 5
    Assignment { target: Box<Expr>, value: Box<Expr> },

    /// Compound assignment: x += 5
    CompoundAssignment {
        target: Box<Expr>,
        op: BinaryOp,
        value: Box<Expr>,
    },

    /// Array literal: [1, 2, 3]
    Array(Vec<Expr>),

    /// Object/map literal: { key: value }
    Object(Vec<(String, Expr)>),

    /// Arrow function: (x) => x + 1
    Lambda {
        params: Vec<Param>,
        body: LambdaBody,
    },

    /// New expression: جديد Person<T>("name")
    New {
        class: Box<Expr>,
        type_args: Vec<TypeAnnotation>,
        args: Vec<Expr>,
    },

    /// Await expression: انتظر promise
    Await(Box<Expr>),

    /// Ternary expression: cond ? a : b
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Grouping: (expr)
    Grouping(Box<Expr>),

    /// This reference: هذا
    This,

    /// Super reference: أساس
    Super,
}

/// Literal values
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Logical
    And,
    Or,
}

impl BinaryOp {
    pub fn from_str(s: &str) -> Option<Self> {
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

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,     // -x
    Not,     // !x, ليس x
    PreInc,  // ++x
    PreDec,  // --x
    PostInc, // x++
    PostDec, // x--
}

/// A block of statements
#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn new(statements: Vec<Stmt>, span: Span) -> Self {
        Self { statements, span }
    }
}

/// Function/method parameter
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeAnnotation>,
    pub default: Option<Expr>,
    pub span: Span,
}

/// Type annotation
#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    pub kind: TypeKind,
    pub span: Span,
}

impl TypeAnnotation {
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Type kinds
#[derive(Debug, Clone)]
pub enum TypeKind {
    /// Simple type: عدد, int
    Simple(String),

    /// Array type: مصفوفة<عدد>
    Array(Box<TypeAnnotation>),

    /// Map type: قاموس<نص, عدد>
    Map(Box<TypeAnnotation>, Box<TypeAnnotation>),

    /// Function type: (عدد, عدد) -> عدد
    Function {
        params: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },

    /// Generic type: قائمة<ن>
    Generic {
        base: String,
        args: Vec<TypeAnnotation>,
    },

    /// Optional type: عدد?
    Optional(Box<TypeAnnotation>),
}

/// Lambda body (expression or block)
#[derive(Debug, Clone)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Block),
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub patterns: Vec<Expr>,
    pub body: Block,
    pub span: Span,
}

/// Catch clause
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: String,
    pub body: Block,
    pub span: Span,
}

/// Import items
#[derive(Debug, Clone)]
pub enum ImportItems {
    /// Named imports: { a, b, c }
    Named(Vec<ImportItem>),
    /// Wildcard: * كـ name
    Wildcard(String),
    /// Default import
    Default(String),
}

/// Single import item
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

/// Property accessor (getter or setter)
#[derive(Debug, Clone)]
pub enum PropertyAccessor {
    /// Get accessor: احصل { ... } or احصل => expr
    Get {
        visibility: Visibility,
        body: PropertyAccessorBody,
    },
    /// Set accessor: عيّن(param) { ... }
    Set {
        visibility: Visibility,
        /// Parameter name (defaults to "قيمة" if not specified)
        param_name: String,
        body: Block,
    },
}

/// Property accessor body (expression or block)
#[derive(Debug, Clone)]
pub enum PropertyAccessorBody {
    /// Expression body: احصل => expr
    Expr(Box<Expr>),
    /// Block body: احصل { ... }
    Block(Block),
}

/// Class member
#[derive(Debug, Clone)]
pub enum ClassMember {
    /// Field: خاص name: type
    Field {
        visibility: Visibility,
        name: String,
        ty: Option<TypeAnnotation>,
        init: Option<Expr>,
        is_static: bool,
        /// Documentation comment attached to this field
        doc_comment: Option<String>,
    },

    /// Method
    Method {
        visibility: Visibility,
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Block,
        is_static: bool,
        is_async: bool,
        /// Documentation comment attached to this method
        doc_comment: Option<String>,
    },

    /// Constructor
    Constructor {
        params: Vec<Param>,
        body: Block,
        /// Documentation comment attached to this constructor
        doc_comment: Option<String>,
    },

    /// Property: خاصية name: type { احصل { ... } عيّن { ... } }
    Property {
        visibility: Visibility,
        name: String,
        ty: TypeAnnotation,
        accessors: Vec<PropertyAccessor>,
        /// Default value for auto-properties
        default_value: Option<Expr>,
        is_static: bool,
        /// Documentation comment attached to this property
        doc_comment: Option<String>,
    },
}

/// Method signature (for interfaces)
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    /// Documentation comment attached to this method signature
    pub doc_comment: Option<String>,
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
