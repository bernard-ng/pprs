//! Byte-preserving PHP 7.0–8.4 parsing, with expressions and file statements.
//!
//! [`lex`] and [`lex_source`] implement PHP lexical modes and file boundaries.
//! [`literal`] decodes byte strings and numeric values. Raw halt tails require
//! parser authorization; standalone lexing keeps scanning ordinary PHP tokens.
//! [`parse`], [`parse_source`] and [`parse_strict`] return owned syntax and diagnostics.
//! Functions/control flow and declarations remain later implementation phases.
//! Strict completion rejects recovery, errors and resource aborts.
//!
//! ```
//! use pprs::{parse_strict, visit, ParseOptions};
//! let file = parse_strict("<?php echo 1 + 2;", ParseOptions::default())?;
//! let program = file.result().program().unwrap();
//! assert_eq!(program.statements.len(), 1);
//! assert!(visit::walk(program).count() > 1);
//! # Ok::<(), pprs::StrictError>(())
//! ```
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
pub mod parser;
pub mod visit;
pub use lexer::{LexResult, lex, lex_source};
pub use parser::{StrictError, parse, parse_source, parse_strict};

pub mod ast;
pub mod diagnostic;
pub mod options;
pub mod outcome;
pub mod source;
pub mod token;

// Shared lazy production lexer/parser cursor.
#[allow(dead_code)]
mod cursor;

// Statement parsing supplies scope before authorizing a raw halt tail.
#[allow(dead_code)]
mod halt;

pub use options::{Limits, ParseOptions, PhpVersion};
pub use outcome::{ParseError, ParseResult, ParseStatus, ParsedFile};
pub use source::{ByteString, SourceText, Span};
