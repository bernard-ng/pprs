#![doc = include_str!("../docs/guide.md")]

pub mod lexer;
pub mod literal;
pub mod parser;
pub mod visit;
pub use diagnostic::{Category, Diagnostic, DiagnosticCode, Label, Severity};
pub use lexer::{InputTooLarge, LexResult, LexStop, lex, lex_source};
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
