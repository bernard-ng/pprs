//! Byte-preserving foundations for a PHP 7.0–8.4 parser library.
//!
//! [`lex`] and [`lex_source`] implement regular tokens and file boundaries. String,
//! heredoc and halt modes stop explicitly pending P06. Parsing APIs are not yet
//! implemented; AST and parse-result types are foundational contracts.
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

pub mod lexer;
pub use lexer::{LexResult, lex, lex_source};

pub mod ast;
pub mod diagnostic;
pub mod options;
pub mod outcome;
pub mod source;
pub mod token;

// Production lexing uses the cursor; parser lookahead/halt methods connect in P06–P07.
#[allow(dead_code)]
mod cursor;

pub use options::{Limits, ParseOptions, PhpVersion};
pub use outcome::{ParseError, ParseResult, ParseStatus, ParsedFile};
pub use source::{ByteString, SourceText, Span};
