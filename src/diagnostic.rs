//! Structured diagnostics; the library does not print or invoke global callbacks.

use crate::{Span, options::Resource, token::TokenKind};

/// Severity affects strict-result conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational warning; by itself it does not prevent strict success.
    Warning,
    /// An input or resource failure.
    Error,
}

/// Broad origin of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Tokenization or literal decoding.
    Lexical,
    /// Grammar mismatch.
    Syntax,
    /// Parser-local contextual restriction.
    Context,
    /// Finite resource budget.
    Resource,
}

/// Stable machine-readable codes. More specific codes arrive with their features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// Byte not recognized in the current lexical state.
    InvalidByte,
    /// Unterminated lexical construct.
    Unterminated,
    /// Unexpected token or missing syntax.
    UnexpectedToken,
    /// Syntax invalid in its surrounding parser context.
    InvalidContext,
    /// Processing stopped at a finite limit.
    ResourceLimit(Resource),
}
impl DiagnosticCode {
    /// Stable identifier, independent of the human-readable message.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidByte => "PPRS-L001",
            Self::Unterminated => "PPRS-L002",
            Self::UnexpectedToken => "PPRS-S001",
            Self::InvalidContext => "PPRS-C001",
            Self::ResourceLimit(Resource::InputBytes) => "PPRS-R001",
            Self::ResourceLimit(Resource::Depth) => "PPRS-R002",
            Self::ResourceLimit(Resource::Tokens) => "PPRS-R003",
            Self::ResourceLimit(Resource::Nodes) => "PPRS-R004",
            Self::ResourceLimit(Resource::Diagnostics) => "PPRS-R005",
        }
    }
    /// Category determined by the code, so it cannot contradict it.
    #[must_use]
    pub const fn category(self) -> Category {
        match self {
            Self::InvalidByte | Self::Unterminated => Category::Lexical,
            Self::UnexpectedToken => Category::Syntax,
            Self::InvalidContext => Category::Context,
            Self::ResourceLimit(_) => Category::Resource,
        }
    }
}

/// A related location and its explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Related source range.
    pub span: Span,
    /// Human-readable explanation; not a stable compatibility key.
    pub message: String,
}

/// Input diagnostic. Source ranges belong to the containing parse result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code, also determining the category.
    pub code: DiagnosticCode,
    /// Warning or error.
    pub severity: Severity,
    /// Primary byte range, possibly empty at EOF.
    pub primary: Span,
    /// Human-readable message; exact wording is not part of compatibility.
    pub message: String,
    /// Related locations, in emission order.
    pub secondary: Vec<Label>,
    /// Expected kinds, if the parser can enumerate them usefully.
    pub expected: Vec<TokenKind>,
    /// Actual kind; `None` means not applicable, whereas EOF is explicit.
    pub found: Option<TokenKind>,
}

impl Diagnostic {
    /// Construct an error without secondary locations or token expectations.
    #[must_use]
    pub fn error(code: DiagnosticCode, primary: Span, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            primary,
            message: message.into(),
            secondary: Vec::new(),
            expected: Vec::new(),
            found: None,
        }
    }
}

// Used by tests now and production diagnostic emission starting in P05.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct DiagnosticBag {
    entries: Vec<Diagnostic>,
    limit: usize,
    stopped: bool,
}
#[allow(dead_code)]
impl DiagnosticBag {
    pub(crate) fn new(limits: crate::Limits) -> Self {
        Self {
            entries: Vec::new(),
            limit: limits.get(Resource::Diagnostics),
            stopped: false,
        }
    }
    /// Reserve one entry for the terminal limit error; never grow past the bound.
    pub(crate) fn push(&mut self, diagnostic: Diagnostic) -> Result<(), Resource> {
        if self.stopped {
            return Err(Resource::Diagnostics);
        }
        if self.entries.len() == self.limit - 1 {
            self.entries.push(Diagnostic::error(
                DiagnosticCode::ResourceLimit(Resource::Diagnostics),
                diagnostic.primary,
                "diagnostic budget exhausted",
            ));
            self.stopped = true;
            return Err(Resource::Diagnostics);
        }
        self.entries.push(diagnostic);
        Ok(())
    }
    pub(crate) fn into_vec(self) -> Vec<Diagnostic> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_limit_reserves_a_terminal_error() {
        let span = crate::SourceText::from("").eof();
        for limit in [1, 2, 10] {
            let mut bag = DiagnosticBag::new(crate::Limits::new(0, 1, 2, 1, limit).unwrap());
            for index in 0..limit + 5 {
                let outcome = bag.push(Diagnostic::error(
                    DiagnosticCode::UnexpectedToken,
                    span,
                    "missing expression",
                ));
                assert_eq!(outcome.is_err(), index >= limit - 1);
            }
            let entries = bag.into_vec();
            assert_eq!(entries.len(), limit);
            assert_eq!(
                entries.last().unwrap().code,
                DiagnosticCode::ResourceLimit(Resource::Diagnostics)
            );
        }
    }
}
