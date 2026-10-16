//! Byte-preserving foundations for a PHP 7.0–8.4 parser library.
//!
//! Parsing and lexing entry points are not implemented yet. The source, token,
//! AST, options and result contracts are available for inspection and tooling.
//!
//! ```
//! use pprs::{SourceText, ParseOptions, PhpVersion};
//! let source = SourceText::from(&b"<?php\r\n\xff"[..]);
//! let span = source.span(7..8)?;
//! assert_eq!(source.slice(span), Some(&b"\xff"[..]));
//! assert_eq!(source.location(7).unwrap().line, 2);
//! assert_eq!(ParseOptions::default().version, PhpVersion::Php84);
//! # Ok::<(), pprs::source::RangeError>(())
//! ```

pub mod ast;
pub mod diagnostic;
pub mod options;
pub mod outcome;
pub mod source;
pub mod token;

// Exercised by unit tests; the production lexer/parser will connect these in P05–P07.
#[allow(dead_code)]
mod cursor;

pub use options::{Limits, ParseOptions, PhpVersion};
pub use outcome::{ParseError, ParseResult, ParseStatus, ParsedFile};
pub use source::{ByteString, SourceText, Span};
