//! Private lazy cursor contract. P05 supplies the production token source.

use crate::{
    Limits, SourceText,
    options::Resource,
    token::{Token, TokenKind, TokenStream, TokenStreamError},
};

/// Producers own their lexical state. They must emit contiguous positive-width
/// tokens and then EOF. They must not consume beyond the returned token;
/// read-only lookahead is allowed.
pub(crate) trait TokenSource {
    fn next_token(&mut self) -> Result<Token, crate::lexer::LexStop>;
    fn take_diagnostics(&mut self) -> Vec<crate::diagnostic::Diagnostic> {
        Vec::new()
    }
    /// Switch at the current byte position, before requesting any tail token.
    fn enter_halt_tail(&mut self);
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CursorError {
    #[error("lookahead exceeds the three-token grammar window")]
    Lookahead,
    #[error(transparent)]
    Lexical(crate::lexer::LexStop),
    #[error("token budget exhausted")]
    TokenLimit,
    #[error("token source violated coverage at retained index {0}")]
    InvalidProducer(usize),
    #[error("halt transition requires a consumed statement terminator and no buffered suffix")]
    HaltBoundary,
    #[error("cursor has stopped")]
    Stopped,
}

pub(crate) struct TokenCursor<L> {
    source: SourceText,
    producer: L,
    tokens: Vec<Token>,
    position: usize,
    last_consumed: Option<TokenKind>,
    limit: usize,
    stopped: bool,
}

impl<L: TokenSource> TokenCursor<L> {
    pub(crate) fn new(source: SourceText, producer: L, limits: Limits) -> Self {
        Self {
            source,
            producer,
            tokens: Vec::new(),
            position: 0,
            last_consumed: None,
            limit: limits.get(Resource::Tokens),
            stopped: false,
        }
    }
    pub(crate) fn retained(&self) -> &[Token] {
        &self.tokens
    }
    pub(crate) fn take_diagnostics(&mut self) -> Vec<crate::diagnostic::Diagnostic> {
        self.producer.take_diagnostics()
    }
    pub(crate) fn scanned_end(&self) -> usize {
        self.tokens.last().map_or(0, |token| token.span().end())
    }
    /// Retained index of the next unconsumed token; useful for progress guards.
    pub(crate) fn position(&self) -> usize {
        self.position
    }
    fn fill(&mut self) -> Result<(), CursorError> {
        if self.stopped {
            return Err(CursorError::Stopped);
        }
        let start = self.tokens.last().map_or(0, |token| token.span().end());
        // Reserve abort suffix + EOF. At source EOF only the sentinel is needed.
        if start < self.source.len() && self.tokens.len() >= self.limit - 2 {
            self.stopped = true;
            return Err(CursorError::TokenLimit);
        }
        let token = self.producer.next_token().map_err(|reason| {
            self.stopped = true;
            CursorError::Lexical(reason)
        })?;
        let eof = token.kind() == TokenKind::Eof;
        if token.span().start() != start
            || token.span().end() > self.source.len()
            || (eof && token.span() != self.source.eof())
            || (!eof && token.span().is_empty())
            || token.kind() == TokenKind::Unscanned
            || (token.kind() == TokenKind::HaltTail && token.span().end() != self.source.len())
        {
            self.stopped = true;
            return Err(CursorError::InvalidProducer(self.tokens.len()));
        }
        self.tokens.push(token);
        Ok(())
    }
    /// Look ahead at most three significant tokens, retaining every trivia token.
    /// EOF is sticky; asking beyond it returns the same sentinel.
    pub(crate) fn peek(&mut self, distance: usize) -> Result<(usize, Token), CursorError> {
        if distance >= 3 {
            return Err(CursorError::Lookahead);
        }
        let mut remaining = distance;
        let mut index = self.position;
        loop {
            if index == self.tokens.len() {
                self.fill()?;
            }
            let token = self.tokens[index];
            if token.kind() == TokenKind::Eof || (!token.kind().is_trivia() && remaining == 0) {
                return Ok((index, token));
            }
            if !token.kind().is_trivia() {
                remaining -= 1;
            }
            index += 1;
        }
    }
    /// Consume one significant token. EOF never advances; recovery must return there.
    pub(crate) fn bump(&mut self) -> Result<(usize, Token), CursorError> {
        let (index, token) = self.peek(0)?;
        self.position = if token.kind() == TokenKind::Eof {
            index
        } else {
            index + 1
        };
        self.last_consumed = Some(token.kind());
        Ok((index, token))
    }
    /// Parser-only authorization after validating outermost `__halt_compiler()` and its statement terminator.
    /// Lexical buffering cannot verify syntax or scope. It rejects speculative
    /// lookahead beyond the boundary, so such bytes can never be silently discarded.
    pub(crate) fn enter_halt_tail(&mut self) -> Result<(), CursorError> {
        if self.stopped
            || !matches!(
                self.last_consumed,
                Some(TokenKind::Semicolon | TokenKind::CloseTag)
            )
            || self.position != self.tokens.len()
        {
            return Err(CursorError::HaltBoundary);
        }
        self.producer.enter_halt_tail();
        self.last_consumed = None;
        Ok(())
    }
    /// Finish scanning after normal parser completion. A token budget or producer
    /// failure returns the retained source as an aborted stream plus its cause.
    pub(crate) fn finish(mut self) -> (Result<TokenStream, TokenStreamError>, Option<CursorError>) {
        loop {
            if self
                .tokens
                .last()
                .is_some_and(|token| token.kind() == TokenKind::Eof)
            {
                return (TokenStream::new(self.source, self.tokens), None);
            }
            if let Err(error) = self.fill() {
                return (self.abort(), Some(error));
            }
        }
    }
    /// Stop immediately; never scan additional bytes after a resource/parser abort.
    pub(crate) fn abort(mut self) -> Result<TokenStream, TokenStreamError> {
        if self
            .tokens
            .last()
            .is_none_or(|token| token.kind() != TokenKind::Eof)
        {
            let start = self.tokens.last().map_or(0, |token| token.span().end());
            if start < self.source.len() {
                let span = self
                    .source
                    .span(start..self.source.len())
                    .expect("cursor only retains validated source ranges");
                self.tokens.push(Token::new(TokenKind::Unscanned, span));
            }
            self.tokens
                .push(Token::new(TokenKind::Eof, self.source.eof()));
        }
        TokenStream::new(self.source, self.tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    // Deterministic producer fixture, not a second PHP lexer.
    struct Fixture {
        source: SourceText,
        offset: usize,
        calls: Rc<Cell<usize>>,
        tail: bool,
    }
    impl TokenSource for Fixture {
        fn next_token(&mut self) -> Result<Token, crate::lexer::LexStop> {
            self.calls.set(self.calls.get() + 1);
            let start = self.offset;
            let kind = match self.source.as_bytes().get(start) {
                None => TokenKind::Eof,
                Some(_) if self.tail => {
                    self.offset = self.source.len();
                    TokenKind::HaltTail
                }
                Some(byte) => {
                    self.offset += 1;
                    match byte {
                        b' ' => TokenKind::Whitespace,
                        b'#' => TokenKind::Comment,
                        b';' => TokenKind::Semicolon,
                        b'x' => TokenKind::Identifier,
                        _ => TokenKind::Invalid,
                    }
                }
            };
            Ok(Token::new(
                kind,
                self.source.span(start..self.offset).unwrap(),
            ))
        }
        fn enter_halt_tail(&mut self) {
            self.tail = true;
        }
    }
    fn cursor(input: &str, limit: usize) -> (TokenCursor<Fixture>, Rc<Cell<usize>>) {
        let source = SourceText::from(input);
        let calls = Rc::new(Cell::new(0));
        let producer = Fixture {
            source: source.clone(),
            offset: 0,
            calls: calls.clone(),
            tail: false,
        };
        (
            TokenCursor::new(
                source,
                producer,
                Limits::new(100, 10, limit, 100, 10).unwrap(),
            ),
            calls,
        )
    }
    #[test]
    fn lazy_lookahead_retains_trivia_indices_and_sticky_eof() {
        let (mut cursor, calls) = cursor(" #x ;", 20);
        assert_eq!(calls.get(), 0);
        assert_eq!(cursor.peek(0).unwrap().0, 2);
        assert_eq!(calls.get(), 3);
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.peek(1).unwrap().0, 4);
        assert_eq!(cursor.bump().unwrap().0, 2);
        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.bump().unwrap().0, 4);
        for distance in 0..3 {
            assert_eq!(cursor.peek(distance).unwrap().0, 5);
        }
        for _ in 0..5 {
            assert_eq!(cursor.bump().unwrap().1.kind(), TokenKind::Eof);
        }
        assert_eq!(cursor.position(), 5);
        assert_eq!(calls.get(), 6);
        assert_eq!(cursor.peek(3), Err(CursorError::Lookahead));
        let (stream, error) = cursor.finish();
        assert!(error.is_none());
        let stream = stream.unwrap();
        assert_eq!(stream.tokens().len(), 6);
        assert_eq!(stream.text(1), Some(&b"#"[..]));
    }
    #[test]
    fn halt_authorization_preserves_raw_suffix_without_speculation() {
        let (mut good, calls) = cursor("; #bad", 20);
        good.bump().unwrap();
        assert_eq!(calls.get(), 1);
        good.enter_halt_tail().unwrap();
        assert_eq!(good.enter_halt_tail(), Err(CursorError::HaltBoundary));
        let (stream, error) = good.finish();
        assert!(error.is_none());
        let stream = stream.unwrap();
        assert_eq!(stream.tokens()[1].kind(), TokenKind::HaltTail);
        assert_eq!(stream.text(1), Some(&b" #bad"[..]));

        let (mut bad, _) = cursor("; x", 20);
        bad.peek(1).unwrap();
        bad.bump().unwrap();
        assert_eq!(bad.enter_halt_tail(), Err(CursorError::HaltBoundary));
        assert_eq!(bad.bump().unwrap().1.kind(), TokenKind::Identifier);
    }
    #[test]
    fn token_limits_and_abort_do_not_scan_remaining_input() {
        for limit in 2..8 {
            let (cursor, calls) = cursor("xxxxxxxxxx", limit);
            let (stream, error) = cursor.finish();
            assert_eq!(error, Some(CursorError::TokenLimit));
            let stream = stream.unwrap();
            assert_eq!(stream.tokens().len(), limit);
            assert_eq!(calls.get(), limit - 2);
            assert!(!stream.is_complete());
            assert_eq!(stream.tokens()[limit - 2].kind(), TokenKind::Unscanned);
        }
        let (aborted, calls) = cursor("xxx", 20);
        assert!(!aborted.abort().unwrap().is_complete());
        assert_eq!(calls.get(), 0);
        let (empty, _) = cursor("", 2);
        assert!(empty.finish().0.unwrap().is_complete());
    }
    #[test]
    fn invalid_bytes_consume_and_broken_producers_stop() {
        struct Broken(Token);
        impl TokenSource for Broken {
            fn next_token(&mut self) -> Result<Token, crate::lexer::LexStop> {
                Ok(self.0)
            }
            fn enter_halt_tail(&mut self) {}
        }
        let (mut cursor, _) = cursor("?x", 10);
        assert_eq!(cursor.bump().unwrap().1.kind(), TokenKind::Invalid);
        assert_eq!(cursor.bump().unwrap().1.kind(), TokenKind::Identifier);
        let source = SourceText::from("xx");
        for token in [
            Token::new(TokenKind::Invalid, source.span(0..0).unwrap()),
            Token::new(TokenKind::Eof, source.span(0..0).unwrap()),
            Token::new(TokenKind::Identifier, source.span(1..2).unwrap()),
            Token::new(TokenKind::HaltTail, source.span(0..1).unwrap()),
            Token::new(TokenKind::Unscanned, source.span(0..2).unwrap()),
            Token::new(
                TokenKind::Identifier,
                SourceText::from("xxx").span(0..3).unwrap(),
            ),
        ] {
            let broken = TokenCursor::new(source.clone(), Broken(token), Limits::default());
            let (stream, error) = broken.finish();
            assert_eq!(error, Some(CursorError::InvalidProducer(0)));
            assert_eq!(stream.unwrap().text(0), Some(&b"xx"[..]));
        }
    }
}
