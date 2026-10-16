//! Immutable source bytes and checked, half-open byte positions.

use std::{fmt, ops::Range, sync::Arc};

/// Owned semantic bytes. Debug output escapes invalid UTF-8 without losing bytes.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct ByteString(bstr::BString);

impl ByteString {
    /// View the original bytes without decoding them.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl From<Vec<u8>> for ByteString {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into())
    }
}
impl From<&[u8]> for ByteString {
    fn from(value: &[u8]) -> Self {
        Self(value.into())
    }
}
impl From<&str> for ByteString {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}
impl fmt::Debug for ByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A reversed or out-of-bounds byte/token range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("range {start}..{end} is invalid for length {len}")]
pub struct RangeError {
    /// Requested start.
    pub start: usize,
    /// Requested exclusive end.
    pub end: usize,
    /// Length against which the range was checked.
    pub len: usize,
}

pub(crate) fn check_range(range: &Range<usize>, len: usize) -> Result<(), RangeError> {
    if range.start <= range.end && range.end <= len {
        Ok(())
    } else {
        Err(RangeError {
            start: range.start,
            end: range.end,
            len,
        })
    }
}

/// A checked half-open byte range. It does not carry a source identity.
///
/// Construct through [`SourceText::span`]. Slicing another source rechecks bounds;
/// callers must keep origins paired with their original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// First byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }
    /// Exclusive byte end.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
    /// Number of bytes.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }
    /// Whether this is a point, including an EOF or missing-node origin.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
    /// Standard half-open range.
    #[must_use]
    pub const fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

/// One-based line and byte column, not a Unicode display coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// One-based line.
    pub line: usize,
    /// One-based byte column.
    pub column: usize,
}

/// Shared immutable bytes and a shared line index, built once on construction.
#[derive(Clone)]
pub struct SourceText {
    bytes: Arc<[u8]>,
    line_starts: Arc<[usize]>,
}

impl SourceText {
    /// Share existing storage without copying source bytes. LF, CRLF and lone CR
    /// are line breaks. An offset inside CRLF belongs to the preceding line.
    #[must_use]
    pub fn from_shared(bytes: Arc<[u8]>) -> Self {
        let mut starts = vec![0];
        let mut offset = 0;
        while offset < bytes.len() {
            match bytes[offset] {
                b'\r' => {
                    offset += 1;
                    if bytes.get(offset) == Some(&b'\n') {
                        offset += 1;
                    }
                    starts.push(offset);
                }
                b'\n' => {
                    offset += 1;
                    starts.push(offset);
                }
                _ => offset += 1,
            }
        }
        Self {
            bytes,
            line_starts: starts.into(),
        }
    }
    /// Source bytes, including BOM, tags, comments and any raw tail.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    /// Whether the source has no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    /// Check a range, permitting empty spans through EOF.
    ///
    /// # Errors
    /// Returns [`RangeError`] for reversed ranges or ends beyond EOF.
    pub fn span(&self, range: Range<usize>) -> Result<Span, RangeError> {
        check_range(&range, self.len())?;
        Ok(Span {
            start: range.start,
            end: range.end,
        })
    }
    /// Empty span at EOF.
    #[must_use]
    pub fn eof(&self) -> Span {
        Span {
            start: self.len(),
            end: self.len(),
        }
    }
    /// Slice bytes. Returns `None` if a span from a different source exceeds EOF.
    #[must_use]
    pub fn slice(&self, span: Span) -> Option<&[u8]> {
        self.bytes.get(span.range())
    }
    /// Resolve a byte offset, including EOF. Returns `None` beyond EOF.
    #[must_use]
    pub fn location(&self, offset: usize) -> Option<Location> {
        if offset > self.len() {
            return None;
        }
        let index = self.line_starts.partition_point(|&start| start <= offset) - 1;
        Some(Location {
            line: index + 1,
            column: offset - self.line_starts[index] + 1,
        })
    }
    /// Line starts, including a final empty line after a trailing line break.
    #[must_use]
    pub fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }
}

impl From<Vec<u8>> for SourceText {
    fn from(value: Vec<u8>) -> Self {
        Self::from_shared(value.into())
    }
}
impl From<&[u8]> for SourceText {
    fn from(value: &[u8]) -> Self {
        Self::from_shared(value.into())
    }
}
impl From<&str> for SourceText {
    fn from(value: &str) -> Self {
        Self::from(value.as_bytes())
    }
}
impl fmt::Debug for SourceText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceText")
            .field("bytes", &bstr::BStr::new(&self.bytes))
            .finish_non_exhaustive()
    }
}
