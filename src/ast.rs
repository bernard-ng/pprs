//! Typed PHP syntax for expressions, statements, callables and declarations.
//!
//! Children are owned. An absent optional child differs from an empty list and
//! from an explicit error expression. Raw spelling is obtained from node origins.

mod declarations;
mod metadata_walk;
pub use declarations::*;
mod procedural;
pub use procedural::*;

use crate::{
    ByteString, Span,
    source::RangeError,
    token::{TokenRange, TokenStream},
};

/// Original source location or an explicitly synthetic/transformed origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Original byte and retained-token coverage; empty ranges represent missing nodes.
    Parsed {
        /// Byte span.
        span: Span,
        /// Retained token range, including intervening trivia.
        tokens: TokenRange,
    },
    /// No claim to represent original source text.
    Synthetic,
}

/// Invalid node metadata construction.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MetadataError {
    /// Invalid token range.
    #[error(transparent)]
    Range(#[from] RangeError),
    /// Index is absent, is not a comment, or occurs more than once.
    #[error("invalid or duplicate comment token index {0}")]
    Comment(usize),
    /// Unscanned bytes cannot be presented as parsed node coverage.
    #[error("node range includes an unscanned suffix")]
    Unscanned,
}

/// Shared node metadata. Comment references use original retained token indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMeta {
    origin: Origin,
    comments: Vec<usize>,
    parenthesized: Option<Box<Origin>>,
}

impl NodeMeta {
    /// Build source metadata from a checked stream; byte coverage is derived from
    /// token coverage. Empty ranges point at the next token's start (or EOF).
    ///
    /// # Errors
    /// Rejects foreign out-of-bounds ranges, unscanned coverage, and invalid comments.
    pub fn parsed(
        stream: &TokenStream,
        tokens: TokenRange,
        comments: Vec<usize>,
    ) -> Result<Self, MetadataError> {
        let span = stream.span(tokens)?;
        if stream.tokens()[tokens.range()]
            .iter()
            .any(|token| token.kind() == crate::token::TokenKind::Unscanned)
        {
            return Err(MetadataError::Unscanned);
        }
        let mut seen = std::collections::HashSet::new();
        for &index in &comments {
            if !stream
                .tokens()
                .get(index)
                .is_some_and(|token| token.kind().is_comment())
                || !seen.insert(index)
            {
                return Err(MetadataError::Comment(index));
            }
        }
        Ok(Self {
            origin: Origin::Parsed { span, tokens },
            comments,
            parenthesized: None,
        })
    }
    pub(crate) fn prefix(
        source: &crate::SourceText,
        tokens: &[crate::token::Token],
        start: usize,
        end: usize,
    ) -> Self {
        let begin = tokens.get(start).map_or(source.len(), |t| t.span().start());
        let finish = if end > start {
            tokens[end - 1].span().end()
        } else {
            begin
        };
        let mut comments = Vec::new();
        for index in (0..start).rev() {
            if !tokens[index].kind().is_trivia() {
                break;
            }
            if tokens[index].kind().is_comment() {
                comments.push(index);
            }
        }
        comments.reverse();
        Self {
            origin: Origin::Parsed {
                span: source.span(begin..finish).expect("parser token bounds"),
                tokens: TokenRange::prefix(start, end, tokens.len()),
            },
            comments,
            parenthesized: None,
        }
    }
    /// Metadata for a constructed/transformed node without fabricated offsets.
    #[must_use]
    pub const fn synthetic() -> Self {
        Self {
            origin: Origin::Synthetic,
            comments: Vec::new(),
            parenthesized: None,
        }
    }
    /// Source origin, including explicit empty or synthetic cases.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        self.origin
    }
    /// Coverage including enclosing grouping parentheses, when present.
    /// `origin()` continues to describe the semantic node itself.
    #[must_use]
    pub const fn enclosing_origin(&self) -> Origin {
        match &self.parenthesized {
            Some(origin) => **origin,
            None => self.origin,
        }
    }
    pub(crate) fn parenthesize(&mut self, outer: &Self) {
        self.parenthesized = Some(Box::new(outer.origin));
    }
    /// Comment indices in attachment order; comment bytes stay in the token stream.
    #[must_use]
    pub fn comments(&self) -> &[usize] {
        &self.comments
    }
}

/// Invalid semantic identifier/name components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("names require nonempty components without namespace separators")]
pub struct InvalidName;

/// A single nonempty identifier, distinct from a potentially qualified name.
/// Keyword acceptance and PHP lexical validity are checked by the parser context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    bytes: ByteString,
    meta: NodeMeta,
}
impl Identifier {
    /// Construct one semantic component, without a leading dollar sign for variables.
    ///
    /// # Errors
    /// Rejects empty bytes and namespace separators; does not enforce keyword rules.
    pub fn new(bytes: impl Into<ByteString>, meta: NodeMeta) -> Result<Self, InvalidName> {
        let bytes = bytes.into();
        validate_component(&bytes)?;
        Ok(Self { bytes, meta })
    }
    /// Exact semantic bytes.
    #[must_use]
    pub const fn bytes(&self) -> &ByteString {
        &self.bytes
    }
    /// Original spelling/position metadata.
    #[must_use]
    pub const fn meta(&self) -> &NodeMeta {
        &self.meta
    }
}
fn validate_component(bytes: &ByteString) -> Result<(), InvalidName> {
    if bytes.as_bytes().is_empty() || bytes.as_bytes().contains(&b'\\') {
        Err(InvalidName)
    } else {
        Ok(())
    }
}

/// Namespace qualification, separate from name component bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualification {
    /// `A` or `A\B`.
    Normal,
    /// `\A\B`.
    FullyQualified,
    /// `namespace\A`.
    Relative,
}

/// A namespace-aware name with one or more components (without prefix spelling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    parts: Vec<ByteString>,
    qualification: Qualification,
    meta: NodeMeta,
}
impl Name {
    /// Construct a name from semantic components.
    ///
    /// # Errors
    /// Rejects empty lists, empty components, and separators embedded in components.
    pub fn new(
        parts: Vec<ByteString>,
        qualification: Qualification,
        meta: NodeMeta,
    ) -> Result<Self, InvalidName> {
        if parts.is_empty() {
            return Err(InvalidName);
        }
        for part in &parts {
            validate_component(part)?;
        }
        Ok(Self {
            parts,
            qualification,
            meta,
        })
    }
    /// Nonempty semantic component list.
    #[must_use]
    pub fn parts(&self) -> &[ByteString] {
        &self.parts
    }
    /// Namespace prefix interpretation.
    #[must_use]
    pub const fn qualification(&self) -> Qualification {
        self.qualification
    }
    /// Raw spelling and position metadata.
    #[must_use]
    pub const fn meta(&self) -> &NodeMeta {
        &self.meta
    }
}

/// Original integer-literal base; spelling remains in the node's source span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerBase {
    /// Base 2.
    Binary,
    /// Base 8, implicit or explicit prefix retained in the source.
    Octal,
    /// Base 10.
    Decimal,
    /// Base 16.
    Hexadecimal,
}
/// Non-interpolated string delimiter kind. Binary prefix and exact quotes remain in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringKind {
    /// Single quotes.
    SingleQuoted,
    /// Double quotes without interpolation.
    DoubleQuoted,
    /// Heredoc without interpolation.
    Heredoc,
    /// Nowdoc.
    Nowdoc,
}
/// PHP binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BinaryOp {
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// String concatenation, with target-dependent precedence.
    Concat,
    /// Remainder.
    Modulo,
    /// Exponentiation.
    Power,
    /// Left shift.
    ShiftLeft,
    /// Right shift.
    ShiftRight,
    /// Bitwise AND.
    BitwiseAnd,
    /// Bitwise OR.
    BitwiseOr,
    /// Bitwise XOR.
    BitwiseXor,
    /// Short-circuit AND.
    BooleanAnd,
    /// Short-circuit OR.
    BooleanOr,
    /// Low-precedence AND.
    LogicalAnd,
    /// Low-precedence OR.
    LogicalOr,
    /// Low-precedence XOR.
    LogicalXor,
    /// Loose equality.
    Equal,
    /// Loose inequality.
    NotEqual,
    /// Strict equality.
    Identical,
    /// Strict inequality.
    NotIdentical,
    /// Less than.
    Less,
    /// Less than or equal.
    LessEqual,
    /// Greater than.
    Greater,
    /// Greater than or equal.
    GreaterEqual,
    /// Three-way comparison.
    Spaceship,
    /// Null coalescing.
    Coalesce,
}

/// Prefix/postfix expression operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Unary plus.
    Plus,
    /// Unary minus.
    Minus,
    /// Boolean negation.
    Not,
    /// Bitwise complement.
    BitwiseNot,
    /// Error suppression.
    Suppress,
    /// Prefix increment.
    PreIncrement,
    /// Postfix increment.
    PostIncrement,
    /// Prefix decrement.
    PreDecrement,
    /// Postfix decrement.
    PostDecrement,
    /// Object cloning.
    Clone,
    /// Print expression.
    Print,
    /// Throw expression.
    Throw,
    /// Yield delegation.
    YieldFrom,
    /// Eval construct.
    Eval,
    /// Empty construct.
    Empty,
}
/// PHP cast syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    /// Integer cast.
    Integer,
    /// Float cast.
    Float,
    /// String cast.
    String,
    /// Array cast.
    Array,
    /// Object cast.
    Object,
    /// Boolean cast.
    Boolean,
    /// Unset cast.
    Unset,
}
/// Include/require operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeKind {
    /// Include operation.
    Include,
    /// `IncludeOnce` operation.
    IncludeOnce,
    /// Require operation.
    Require,
    /// `RequireOnce` operation.
    RequireOnce,
}
/// Unevaluated PHP magic constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicConstant {
    /// Line magic constant.
    Line,
    /// File magic constant.
    File,
    /// Directory magic constant.
    Directory,
    /// Class magic constant.
    Class,
    /// Trait magic constant.
    Trait,
    /// Method magic constant.
    Method,
    /// Function magic constant.
    Function,
    /// Namespace magic constant.
    Namespace,
    /// Property magic constant.
    Property,
}
/// Original array or pattern delimiter syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayKind {
    /// Square brackets.
    Short,
    /// Array keyword and parentheses.
    Long,
    /// List keyword and parentheses.
    List,
}
/// A name or evaluated expression used as a class or function target.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// Syntactic class/function name.
    Name(Name),
    /// Dynamic target.
    Expr(Box<Expr>),
}
/// A literal member name or evaluated expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    /// Identifier; static properties interpret it as a variable-like identifier.
    Identifier(Identifier),
    /// Dynamic member expression.
    Expr(Box<Expr>),
}
/// Call argument, retaining named/reference/unpack distinctions.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    /// Source metadata.
    pub meta: NodeMeta,
    /// Argument name, if named.
    pub name: Option<Identifier>,
    /// Value; absent only for a first-class callable placeholder.
    pub value: Option<Expr>,
    /// Call-time reference syntax.
    pub by_ref: bool,
    /// Unpack syntax; the standalone placeholder instead has no value.
    pub unpack: bool,
}
/// Array element or destructuring entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayItem {
    /// Source metadata.
    pub meta: NodeMeta,
    /// Explicit key.
    pub key: Option<Expr>,
    /// Element or pattern value.
    pub value: Expr,
    /// Reference syntax.
    pub by_ref: bool,
    /// Unpacked element.
    pub unpack: bool,
}
/// Part of an interpolated string or shell command.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Decoded text segment, whose metadata retains raw spelling.
    Text {
        /// Source metadata.
        meta: NodeMeta,
        /// Decoded bytes.
        value: ByteString,
    },
    /// Embedded expression.
    Expr(Expr),
}
/// A match expression arm.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// Source metadata.
    pub meta: NodeMeta,
    /// Conditions, or `None` for default.
    pub conditions: Option<Vec<Expr>>,
    /// Result expression.
    pub body: Expr,
}
/// Expression node and its origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Origin and comments.
    pub meta: NodeMeta,
    /// Typed expression structure.
    pub kind: ExprKind,
}
/// PHP expression variants. Match with a fallback for future variants.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExprKind {
    /// Anonymous class construction.
    NewAnonymous(Box<AnonymousClass>),
    /// Signed 64-bit PHP integer model, independent of pointer width. A lexical
    /// sign remains a separate operator when the grammar treats it as syntax.
    Integer {
        /// Decoded semantic value.
        value: i64,
        /// Original base.
        base: IntegerBase,
    },
    /// Binary64 value, including infinity on overflow. Preserve spelling via metadata.
    Float(f64),
    /// Decoded, potentially non-UTF-8 bytes, distinct from original literal spelling.
    String {
        /// Decoded bytes.
        value: ByteString,
        /// Delimiter interpretation.
        kind: StringKind,
    },
    /// Simple variable.
    Variable(Identifier),
    /// Constant/name lookup; this is not an identifier expression.
    ConstFetch(Name),
    /// Simple assignment. Parser validation determines valid target expressions.
    Assign {
        /// Assignment target.
        target: Box<Expr>,
        /// Assigned value.
        value: Box<Expr>,
    },
    /// Binary operation.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Variable whose name is evaluated.
    DynamicVariable(Box<Expr>),
    /// Unevaluated magic constant.
    MagicConstant(MagicConstant),
    /// Unary operation.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        expr: Box<Expr>,
    },
    /// Compound assignment.
    AssignOp {
        /// Operator applied before assignment.
        op: BinaryOp,
        /// Target.
        target: Box<Expr>,
        /// Value.
        value: Box<Expr>,
    },
    /// Assignment by reference.
    AssignRef {
        /// Target.
        target: Box<Expr>,
        /// Referenced value.
        value: Box<Expr>,
    },
    /// Conditional expression, including shorthand ternary.
    Ternary {
        /// Condition.
        condition: Box<Expr>,
        /// True branch; absent for shorthand.
        then: Option<Box<Expr>>,
        /// False branch.
        otherwise: Box<Expr>,
    },
    /// Array literal; absent entries are retained as error expressions until pattern conversion.
    Array {
        /// Items.
        items: Vec<Option<ArrayItem>>,
        /// Delimiter syntax.
        kind: ArrayKind,
    },
    /// Destructuring pattern; omitted slots are absent items.
    List {
        /// Pattern slots.
        items: Vec<Option<ArrayItem>>,
        /// Delimiter syntax.
        kind: ArrayKind,
    },
    /// Array access; absent dimension means append syntax.
    ArrayDim {
        /// Base.
        array: Box<Expr>,
        /// Optional index.
        dim: Option<Box<Expr>>,
    },
    /// Function or dynamic call.
    Call {
        /// Callable.
        target: Target,
        /// Arguments or first-class placeholder.
        args: Vec<Argument>,
    },
    /// Property access.
    Property {
        /// Object.
        object: Box<Expr>,
        /// Property name.
        name: Member,
        /// Nullsafe access.
        nullsafe: bool,
    },
    /// Method call.
    MethodCall {
        /// Object.
        object: Box<Expr>,
        /// Method name.
        name: Member,
        /// Arguments.
        args: Vec<Argument>,
        /// Nullsafe call.
        nullsafe: bool,
    },
    /// Static property access.
    StaticProperty {
        /// Class.
        class: Target,
        /// Property name.
        name: Member,
    },
    /// Class constant access.
    ClassConst {
        /// Class.
        class: Target,
        /// Constant name.
        name: Member,
    },
    /// Static method call.
    StaticCall {
        /// Class.
        class: Target,
        /// Method name.
        name: Member,
        /// Arguments.
        args: Vec<Argument>,
    },
    /// Named or dynamic object construction. See also [`ExprKind::NewAnonymous`].
    New {
        /// Class expression/name.
        class: Target,
        /// Constructor arguments.
        args: Vec<Argument>,
    },
    /// Instance check.
    Instanceof {
        /// Tested expression.
        expr: Box<Expr>,
        /// Class expression/name.
        class: Target,
    },
    /// Cast syntax.
    Cast {
        /// Cast operation.
        kind: CastKind,
        /// Operand.
        expr: Box<Expr>,
    },
    /// Include/require syntax.
    Include {
        /// Include operation.
        kind: IncludeKind,
        /// Operand.
        expr: Box<Expr>,
    },
    /// Isset construct.
    Isset(Vec<Expr>),
    /// Simple exit/die syntax; complex argument forms are calls, matching the reference.
    Exit(Option<Box<Expr>>),
    /// Yield expression. The parser does not validate generator context.
    Yield {
        /// Optional key.
        key: Option<Box<Expr>>,
        /// Optional value.
        value: Option<Box<Expr>>,
    },
    /// Match expression.
    Match {
        /// Subject.
        condition: Box<Expr>,
        /// Ordered arms.
        arms: Vec<MatchArm>,
    },
    /// Double-quoted/heredoc interpolated string.
    Interpolated {
        /// Decoded text and expression parts.
        parts: Vec<StringPart>,
        /// Delimiter syntax.
        kind: StringKind,
    },
    /// Anonymous function.
    Closure(Box<Closure>),
    /// Arrow function.
    ArrowFunction(Box<ArrowFunction>),
    /// Shell command syntax, never executed by the library.
    ShellExec(Vec<StringPart>),
    /// Explicit missing/damaged expression. Metadata may be an empty insertion point.
    Error,
}

/// Statement node and its origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// Origin and comments.
    pub meta: NodeMeta,
    /// Typed statement structure.
    pub kind: StmtKind,
}
/// Initial statement family.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum StmtKind {
    /// Namespace declaration and grouped body.
    Namespace(Box<Namespace>),
    /// Simple or grouped imports.
    Use(Box<UseStatement>),
    /// Global constant declaration.
    Constants(Box<Constants>),
    /// Class, interface, trait or enum declaration.
    Class(Box<ClassDeclaration>),
    /// Class constant declaration.
    ClassConstants(Box<Constants>),
    /// Class method.
    Method(Box<Method>),
    /// Class property declaration.
    Property(Box<Property>),
    /// Trait imports and adaptations.
    TraitUse(Box<TraitUse>),
    /// Enum case.
    EnumCase(Box<EnumCase>),
    /// Expression followed by statement termination.
    Expression(Expr),
    /// Empty statement or comment-carrying no-op.
    Nop,
    /// Bytes outside PHP tags.
    InlineHtml(ByteString),
    /// Echo expressions in source order; parser enforces nonempty lists.
    Echo(Vec<Expr>),
    /// An absent return value is distinct from a damaged expression.
    Return(Option<Expr>),
    /// Braced statement block.
    Block(Vec<Stmt>),
    /// Named function declaration.
    Function(Box<Function>),
    /// Conditional branches.
    If(Box<IfStatement>),
    /// Precondition loop.
    While(Box<ConditionalLoop>),
    /// Postcondition loop.
    Do(Box<ConditionalLoop>),
    /// For loop.
    For(Box<ForStatement>),
    /// Foreach loop.
    Foreach(Box<ForeachStatement>),
    /// Switch with ordered cases.
    Switch(Box<SwitchStatement>),
    /// Break, optionally with a level expression.
    Break(Option<Expr>),
    /// Continue, optionally with a level expression.
    Continue(Option<Expr>),
    /// Jump to a label.
    Goto(Identifier),
    /// Label declaration.
    Label(Identifier),
    /// Global variable bindings.
    Global(Vec<Expr>),
    /// Static local bindings.
    Static(Vec<StaticVariable>),
    /// Unset variables.
    Unset(Vec<Expr>),
    /// Scoped or unscoped directives.
    Declare(Box<DeclareStatement>),
    /// Exception handling.
    Try(Box<TryStatement>),
    /// Validated outermost halt statement and uninterpreted tail bytes.
    HaltCompiler(ByteString),
}
/// File root. An empty statement list is a valid empty program; a partial root is
/// distinguished by the containing [`crate::ParseResult`]'s status.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Program {
    /// Top-level statements in source order.
    pub statements: Vec<Stmt>,
}

/// Give each retained comment one owner in outermost-first source traversal.
pub(crate) fn normalize_comments(program: &mut Program) {
    let mut owned = std::collections::HashSet::new();
    metadata_walk::metadata_mut(program, |meta| {
        meta.comments.retain(|index| owned.insert(*index));
    });
}
