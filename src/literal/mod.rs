//! Byte-oriented literal interpretation, separate from lexing and AST construction.
//!
//! Offsets in errors are relative to the supplied spelling/segment. Callers retain
//! source spans and translate these offsets when constructing parser diagnostics.
// Literal semantics adapted from nikic/PHP-Parser, Copyright (c) 2011 Nikita Popov.
// See THIRD_PARTY_NOTICES.md and reference/LICENSE.nikic-PHP-Parser (BSD-3-Clause).
mod number;
mod string;
pub use number::{Number, NumericOffset, decode_number, decode_numeric_offset};
use std::ops::Range;
pub use string::{EscapeMode, decode_doc_segment, decode_escapes, decode_quoted};

/// Kind of literal interpretation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LiteralErrorKind {
    /// The argument is not one complete spelling of the requested literal kind.
    #[error("invalid literal spelling")]
    InvalidSpelling,
    /// Invalid digit in an integer octal literal.
    #[error("invalid numeric literal")]
    InvalidOctal,
    /// A malformed Unicode escape or code point above U+10FFFF.
    #[error("invalid UTF-8 codepoint escape sequence")]
    UnicodeEscape,
    /// Mixed tabs/spaces in the required indentation prefix.
    #[error("tabs and spaces cannot be mixed in doc-string indentation")]
    MixedIndentation,
    /// A nonblank line has fewer indentation bytes than the closing label.
    #[error("insufficient doc-string body indentation")]
    InsufficientIndentation,
}
/// A literal error with a half-open byte range relative to its input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind} at bytes {range:?}")]
pub struct LiteralError {
    /// Stable failure classification.
    pub kind: LiteralErrorKind,
    /// Relative byte range. Indentation configuration errors use an empty range.
    pub range: Range<usize>,
}
impl LiteralError {
    fn new(kind: LiteralErrorKind, range: Range<usize>) -> Self {
        Self { kind, range }
    }
}
