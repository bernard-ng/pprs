//! Byte-preserving foundations for a PHP 7.0–8.4 parser library.
//!
//! [`lex`] and [`lex_source`] implement PHP lexical modes and file boundaries.
//! [`literal`] decodes byte strings and numeric values. Raw halt tails require
//! parser authorization; standalone lexing keeps scanning ordinary PHP tokens.
//! Parsing APIs remain pending; AST and parse-result types are foundational contracts.
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
pub mod literal;
pub use lexer::{LexResult, lex, lex_source};

pub mod ast;
pub mod diagnostic;
pub mod options;
pub mod outcome;
pub mod source;
pub mod token;

// Production lexing uses the cursor; statement parsing connects in P07.
#[allow(dead_code)]
mod cursor;

// P07 supplies statement scope; P06 tests this boundary against the production scanner.
#[allow(dead_code)]
mod halt;

pub use options::{Limits, ParseOptions, PhpVersion};
pub use outcome::{ParseError, ParseResult, ParseStatus, ParsedFile};
pub use source::{ByteString, SourceText, Span};
