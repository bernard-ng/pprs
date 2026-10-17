//! Owned results and strict/recovering conversion for public parsing entry points.

use crate::{
    SourceText,
    ast::Program,
    diagnostic::{Category, Diagnostic, Severity},
    token::TokenStream,
};

/// Completion status, independent of whether a useful root exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseStatus {
    /// Completed without errors or recovery. Warnings are permitted.
    Clean,
    /// Completed with error diagnostics/recovery; the root is available.
    Recovered,
    /// Processing stopped; any available root is only a prefix/partial tree.
    Aborted,
}

/// Recovering result, owning source, tokens, any root and ordered diagnostics.
/// Immutable access prevents callers changing a tree without updating its status.
#[derive(Debug)]
pub struct ParseResult {
    tokens: TokenStream,
    program: Option<Program>,
    diagnostics: Vec<Diagnostic>,
    status: ParseStatus,
}

// Only the parser may assert completion. No public constructor can manufacture
// successful parsing from arbitrary caller-supplied parts.
#[allow(dead_code)]
pub(crate) enum Completion {
    Complete(Program),
    Recovered(Program),
    Aborted(Option<Program>),
}

impl ParseResult {
    #[allow(dead_code)]
    pub(crate) fn assemble(
        tokens: TokenStream,
        completion: Completion,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        let (program, mut status) = match completion {
            Completion::Complete(program) => (Some(program), ParseStatus::Clean),
            Completion::Recovered(program) => (Some(program), ParseStatus::Recovered),
            Completion::Aborted(program) => (program, ParseStatus::Aborted),
        };
        if !tokens.is_complete()
            || diagnostics
                .iter()
                .any(|item| item.code.category() == Category::Resource)
        {
            status = ParseStatus::Aborted;
        } else if status == ParseStatus::Clean
            && diagnostics
                .iter()
                .any(|item| item.severity == Severity::Error)
        {
            status = ParseStatus::Recovered;
        }
        Self {
            tokens,
            program,
            diagnostics,
            status,
        }
    }
    /// Original immutable source, independent of caller buffer lifetimes.
    #[must_use]
    pub const fn source(&self) -> &SourceText {
        self.tokens.source()
    }
    /// Lossless retained token stream; unscanned suffixes are explicit on abort.
    #[must_use]
    pub const fn tokens(&self) -> &TokenStream {
        &self.tokens
    }
    /// Complete or partial root. Check status before treating it as complete.
    #[must_use]
    pub const fn program(&self) -> Option<&Program> {
        self.program.as_ref()
    }
    /// Diagnostics in emission order, including lexical errors.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Whether parsing completed cleanly, recovered, or aborted.
    #[must_use]
    pub const fn status(&self) -> ParseStatus {
        self.status
    }
    /// Require clean completion. This does not perform strict PHP-version validation.
    ///
    /// # Errors
    /// Returns the entire original result on recovery, errors, or abort.
    pub fn into_strict(self) -> Result<ParsedFile, ParseError> {
        if self.status == ParseStatus::Clean {
            Ok(ParsedFile(self))
        } else {
            Err(ParseError(Box::new(self)))
        }
    }
}

/// A file known to have completed without recovery or error diagnostics.
#[derive(Debug)]
pub struct ParsedFile(ParseResult);
impl ParsedFile {
    /// The complete root guaranteed by strict success, including an empty file.
    #[must_use]
    #[allow(
        clippy::missing_panics_doc,
        reason = "only private assembly can violate the clean-root invariant"
    )]
    pub fn program(&self) -> &Program {
        self.0
            .program
            .as_ref()
            .expect("clean completion retains a program")
    }

    /// Inspect source, tokens, diagnostics and root without changing clean status.
    #[must_use]
    pub const fn result(&self) -> &ParseResult {
        &self.0
    }
    /// Recover the owned general result.
    #[must_use]
    pub fn into_result(self) -> ParseResult {
        self.0
    }
}

/// Strict failure retaining source, diagnostics and any partial/recovered tree.
#[derive(Debug, thiserror::Error)]
#[error("PHP parsing did not complete cleanly ({:?})", .0.status())]
pub struct ParseError(Box<ParseResult>);
impl ParseError {
    /// Inspect the original recovering result.
    #[must_use]
    pub fn result(&self) -> &ParseResult {
        &self.0
    }
    /// Recover ownership of the original result.
    #[must_use]
    pub fn into_result(self) -> ParseResult {
        *self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostic::{DiagnosticCode, Severity},
        options::Resource,
        token::{Token, TokenKind},
    };

    fn empty() -> TokenStream {
        let source = SourceText::from("");
        let eof = Token::new(TokenKind::Eof, source.eof());
        TokenStream::new(source, vec![eof]).unwrap()
    }
    #[test]
    fn clean_empty_root_is_not_abort_and_warnings_are_allowed() {
        let mut warning = Diagnostic::error(
            DiagnosticCode::InvalidContext,
            empty().source().eof(),
            "warning",
        );
        warning.severity = Severity::Warning;
        let clean = ParseResult::assemble(
            empty(),
            Completion::Complete(Program::default()),
            vec![warning],
        );
        let strict = clean.into_strict().unwrap();
        assert_eq!(strict.result().program().unwrap().statements.len(), 0);
        assert_eq!(strict.into_result().status(), ParseStatus::Clean);
        let aborted = ParseResult::assemble(empty(), Completion::Aborted(None), vec![]);
        let failure = aborted.into_strict().unwrap_err().into_result();
        assert_eq!(failure.status(), ParseStatus::Aborted);
        assert!(failure.program().is_none());
    }
    #[test]
    fn lexical_errors_recovery_and_limits_cannot_be_strict_success() {
        for (code, expected) in [
            (DiagnosticCode::InvalidByte, ParseStatus::Recovered),
            (
                DiagnosticCode::ResourceLimit(Resource::Nodes),
                ParseStatus::Aborted,
            ),
        ] {
            let error = Diagnostic::error(code, empty().source().eof(), "problem");
            let result = ParseResult::assemble(
                empty(),
                Completion::Complete(Program::default()),
                vec![error.clone()],
            );
            let failure = result.into_strict().unwrap_err();
            assert_eq!(failure.result().status(), expected);
            assert_eq!(failure.result().diagnostics(), &[error]);
            assert!(failure.result().program().is_some());
        }
        let result =
            ParseResult::assemble(empty(), Completion::Recovered(Program::default()), vec![]);
        assert_eq!(
            result.into_strict().unwrap_err().result().status(),
            ParseStatus::Recovered
        );
    }
    #[test]
    fn unscanned_suffix_overrides_a_claim_of_complete_parsing() {
        let source = SourceText::from("unscanned");
        let tokens = vec![
            Token::new(TokenKind::Unscanned, source.span(0..source.len()).unwrap()),
            Token::new(TokenKind::Eof, source.eof()),
        ];
        let result = ParseResult::assemble(
            TokenStream::new(source, tokens).unwrap(),
            Completion::Complete(Program::default()),
            vec![],
        );
        assert_eq!(result.status(), ParseStatus::Aborted);
        assert!(result.into_strict().is_err());
    }
    #[test]
    fn aborted_result_owns_caller_bytes_and_does_not_scan_suffix() {
        let result = {
            let input = vec![0xff, b'\r', b'\n', 0];
            let source = SourceText::from(input.as_slice());
            let tokens = vec![
                Token::new(TokenKind::Unscanned, source.span(0..4).unwrap()),
                Token::new(TokenKind::Eof, source.eof()),
            ];
            ParseResult::assemble(
                TokenStream::new(source, tokens).unwrap(),
                Completion::Aborted(Some(Program::default())),
                vec![],
            )
        };
        assert_eq!(result.source().as_bytes(), &[0xff, b'\r', b'\n', 0]);
        assert_eq!(result.status(), ParseStatus::Aborted);
        assert!(!result.tokens().is_complete());
        assert_eq!(
            result
                .into_strict()
                .unwrap_err()
                .into_result()
                .source()
                .location(3)
                .unwrap()
                .line,
            2
        );
    }
}
