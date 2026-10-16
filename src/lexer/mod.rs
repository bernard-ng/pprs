//! PHP lexical modes and file boundaries. Raw halt tails require parser authorization.
mod keywords;
pub(crate) mod regular;
pub(crate) mod scanner;
mod strings;

use crate::{
    Limits, ParseOptions, SourceText,
    cursor::{CursorError, TokenCursor},
    diagnostic::{Diagnostic, DiagnosticCode},
    options::Resource,
    token::TokenStream,
};
use scanner::Scanner;

/// Why scanning stopped before completing the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LexStop {
    /// Finite resource budget exhausted; a corresponding diagnostic is retained.
    #[error("{0} budget exhausted")]
    Limit(Resource),
}

/// Input rejected before copying source bytes or building a line index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("input length {length} exceeds the configured limit {limit}")]
pub struct InputTooLarge {
    /// Supplied byte length.
    pub length: usize,
    /// Configured maximum byte length.
    pub limit: usize,
}

/// Owned lexical output, including errors and any explicit unscanned suffix.
#[derive(Debug)]
pub struct LexResult {
    tokens: TokenStream,
    diagnostics: Vec<Diagnostic>,
    stop: Option<LexStop>,
}
impl LexResult {
    /// Original immutable bytes.
    #[must_use]
    pub const fn source(&self) -> &SourceText {
        self.tokens.source()
    }
    /// Every scanned token, all trivia, any unscanned suffix and EOF.
    #[must_use]
    pub const fn tokens(&self) -> &TokenStream {
        &self.tokens
    }
    /// Lexical diagnostics in emission order; no syntax/parser diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Stop reason, if scanning did not complete.
    #[must_use]
    pub const fn stop(&self) -> Option<LexStop> {
        self.stop
    }
    /// Whether scanning and EOF checks finished without a resource stop.
    /// This does not imply error-free or valid PHP; EOF checks can exhaust the
    /// diagnostic budget even when every source byte has already been scanned.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.stop.is_none()
    }
}

/// Lex caller-supplied bytes, copying them into owned storage after the input
/// budget check. Short tags and version interpretation are explicit options.
///
/// ```
/// let result = pprs::lex("<?php echo $value + 1; ?>tail", pprs::ParseOptions::default())?;
/// assert!(result.is_complete());
/// assert!(result.diagnostics().is_empty());
/// # Ok::<(), pprs::lexer::InputTooLarge>(())
/// ```
///
/// # Errors
/// Returns [`InputTooLarge`] before copying or indexing an oversized input.
pub fn lex(input: impl AsRef<[u8]>, options: ParseOptions) -> Result<LexResult, InputTooLarge> {
    check_input(input.as_ref().len(), options.limits)?;
    lex_source(SourceText::from(input.as_ref()), options)
}

/// Lex an already owned source without copying its bytes or line index.
///
/// # Errors
/// Returns [`InputTooLarge`] if the supplied source exceeds the input budget.
#[allow(
    clippy::missing_panics_doc,
    reason = "panics indicate private scanner invariant bugs, not input failures"
)]
pub fn lex_source(source: SourceText, options: ParseOptions) -> Result<LexResult, InputTooLarge> {
    check_input(source.len(), options.limits)?;
    let scanner = Scanner::new(source.clone(), options);
    let diagnostic_source = source.clone();
    let mut cursor = TokenCursor::new(source, scanner, options.limits);
    let stop = loop {
        match cursor.bump() {
            Ok((_, token)) if token.kind() == crate::token::TokenKind::Eof => break None,
            Ok(_) => {}
            Err(CursorError::TokenLimit) => break Some(LexStop::Limit(Resource::Tokens)),
            Err(CursorError::Lexical(stop)) => break Some(stop),
            Err(error) => panic!("internal lexer/cursor invariant: {error}"),
        }
    };
    let mut diagnostics = cursor.take_diagnostics();
    if stop == Some(LexStop::Limit(Resource::Tokens)) {
        let offset = cursor.scanned_end();
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::ResourceLimit(Resource::Tokens),
            diagnostic_source
                .span(offset..offset)
                .expect("cursor offset within source"),
            "token budget exhausted",
        ));
    }
    let tokens = cursor
        .abort()
        .expect("production lexer emits contiguous tokens");
    Ok(LexResult {
        tokens,
        diagnostics,
        stop,
    })
}

fn check_input(length: usize, limits: Limits) -> Result<(), InputTooLarge> {
    let limit = limits.get(Resource::InputBytes);
    if length > limit {
        Err(InputTooLarge { length, limit })
    } else {
        Ok(())
    }
}
