//! Typed AST for the first vertical slice. These types do not imply parser coverage.
//!
//! Children are owned. An absent optional child differs from an empty list and
//! from an explicit error expression. Raw spelling is obtained from node origins.

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
        })
    }
    /// Metadata for a constructed/transformed node without fabricated offsets.
    #[must_use]
    pub const fn synthetic() -> Self {
        Self {
            origin: Origin::Synthetic,
            comments: Vec::new(),
        }
    }
    /// Source origin, including explicit empty or synthetic cases.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        self.origin
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
/// Binary operators needed by the first expression slice.
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
}

/// Expression node and its origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Origin and comments.
    pub meta: NodeMeta,
    /// Typed expression structure.
    pub kind: ExprKind,
}
/// Initial expression family; remaining reference nodes are tracked in the ledger.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExprKind {
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
    /// Simple variable; dynamic variable expressions arrive in P07.
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
}
/// File root. An empty statement list is a valid empty program; a partial root is
/// distinguished by the containing [`crate::ParseResult`]'s status.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Program {
    /// Top-level statements in source order.
    pub statements: Vec<Stmt>,
}
