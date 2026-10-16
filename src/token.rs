//! Project-owned token kinds and lossless retained token streams.

use crate::{
    SourceText, Span,
    source::{RangeError, check_range},
};
use std::ops::Range;

/// Token vocabulary for the first vertical slice; extended with later grammar phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenKind {
    /// Text outside PHP tags, including a BOM or hashbang.
    InlineHtml,
    /// `<?php` or enabled `<?`, including any lexically owned whitespace.
    OpenTag,
    /// `<?=` (significant implicit echo).
    OpenEchoTag,
    /// `?>`, including its lexically owned newline (significant termination).
    CloseTag,
    /// Whitespace.
    Whitespace,
    /// Ordinary line or block comment.
    Comment,
    /// Documentation comment.
    DocComment,
    /// An identifier, preserving case and high bytes in its source span.
    Identifier,
    /// Simple variable including its dollar sign.
    Variable,
    /// Integer-shaped spelling; overflow interpretation occurs in P06.
    Integer,
    /// Floating-point spelling.
    Float,
    /// A non-interpolated quoted string.
    QuotedString,
    /// `echo`.
    Echo,
    /// `return`.
    Return,
    /// `__halt_compiler`.
    HaltCompiler,
    /// `;`.
    Semicolon,
    /// `,`.
    Comma,
    /// `(`.
    LeftParen,
    /// `)`.
    RightParen,
    /// `{`.
    LeftBrace,
    /// `}`.
    RightBrace,
    /// `=`.
    Equal,
    /// `+`.
    Plus,
    /// `-`.
    Minus,
    /// `*`.
    Star,
    /// `/`.
    Slash,
    /// `.`.
    Dot,
    /// A byte sequence rejected by the lexer; always consumes bytes.
    Invalid,
    /// Bytes after a parser-validated top-level halt statement.
    HaltTail,
    /// A suffix deliberately not scanned because processing aborted.
    Unscanned,
    /// The single empty sentinel at source length; never contains a NUL byte.
    Eof,
}

impl TokenKind {
    /// Whether grammar lookahead skips this token. Tags with syntax are significant.
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace | Self::Comment | Self::DocComment | Self::OpenTag
        )
    }
    /// Whether a node may reference this token as a comment.
    #[must_use]
    pub const fn is_comment(self) -> bool {
        matches!(self, Self::Comment | Self::DocComment)
    }
}

/// A token stores its kind and source range, never a duplicate text allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    span: Span,
}

impl Token {
    /// Construct a token. Stream construction checks coverage and EOF invariants.
    #[must_use]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
    /// Project-owned kind.
    #[must_use]
    pub const fn kind(self) -> TokenKind {
        self.kind
    }
    /// Original byte range.
    #[must_use]
    pub const fn span(self) -> Span {
        self.span
    }
}

/// Half-open indices into a retained token stream, including trivia.
/// Like byte spans, these ranges have bounds but no stream identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRange {
    start: usize,
    end: usize,
}
impl TokenRange {
    /// First retained index.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }
    /// Exclusive retained end.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
    /// Standard index range.
    #[must_use]
    pub const fn range(self) -> Range<usize> {
        self.start..self.end
    }
    /// Whether the origin is an insertion point.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Violated lossless stream invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TokenStreamError {
    /// Empty streams and streams without a final EOF are invalid.
    #[error("missing final EOF token")]
    MissingEof,
    /// Tokens must cover the source once, with exactly one empty EOF at the end.
    #[error("invalid coverage or token placement at retained index {0}")]
    InvalidToken(usize),
}

/// Source and a checked lossless token sequence. An aborted sequence still covers
/// all bytes, using a final [`TokenKind::Unscanned`] suffix before EOF.
#[derive(Debug, Clone)]
pub struct TokenStream {
    source: SourceText,
    tokens: Vec<Token>,
}

impl TokenStream {
    /// Validate exact contiguous coverage, positive token widths, EOF, and tails.
    /// This checks structure, not whether kinds correctly classify PHP text.
    ///
    /// # Errors
    /// Rejects gaps, overlaps, misplaced/duplicate sentinels, and out-of-bounds spans.
    pub fn new(source: SourceText, tokens: Vec<Token>) -> Result<Self, TokenStreamError> {
        if tokens.last().map(|token| token.kind) != Some(TokenKind::Eof) {
            return Err(TokenStreamError::MissingEof);
        }
        let mut end = 0;
        for (index, token) in tokens.iter().enumerate() {
            let eof = token.kind == TokenKind::Eof;
            let tail = matches!(token.kind, TokenKind::HaltTail | TokenKind::Unscanned);
            if token.span.start() != end
                || token.span.end() > source.len()
                || (eof && (index != tokens.len() - 1 || token.span != source.eof()))
                || (!eof && token.span.is_empty())
                || (tail && index != tokens.len() - 2)
            {
                return Err(TokenStreamError::InvalidToken(index));
            }
            end = token.span.end();
        }
        Ok(Self { source, tokens })
    }
    /// Retained source storage.
    #[must_use]
    pub const fn source(&self) -> &SourceText {
        &self.source
    }
    /// All tokens, including trivia and EOF.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
    /// Whether every byte was scanned, irrespective of lexical validity.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Unscanned)
    }
    /// Exact text of a retained token, including an empty slice for EOF.
    #[must_use]
    pub fn text(&self, index: usize) -> Option<&[u8]> {
        self.tokens
            .get(index)
            .and_then(|token| self.source.slice(token.span))
    }
    /// Check a range of retained token indices. Empty ranges through `tokens.len()`
    /// are allowed; the end insertion point maps to source EOF.
    ///
    /// # Errors
    /// Rejects reversed ranges and indices beyond the retained stream.
    pub fn range(&self, range: Range<usize>) -> Result<TokenRange, RangeError> {
        check_range(&range, self.tokens.len())?;
        Ok(TokenRange {
            start: range.start,
            end: range.end,
        })
    }
    /// Convert token coverage to bytes, rechecking bounds for foreign ranges.
    ///
    /// # Errors
    /// Rejects ranges that exceed this stream.
    pub fn span(&self, range: TokenRange) -> Result<Span, RangeError> {
        check_range(&range.range(), self.tokens.len())?;
        let start = self
            .tokens
            .get(range.start)
            .map_or(self.source.len(), |token| token.span.start());
        let end = if range.is_empty() {
            start
        } else {
            self.tokens[range.end - 1].span.end()
        };
        self.source.span(start..end)
    }
}
