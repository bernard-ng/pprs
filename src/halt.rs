//! Parser-owned halt grammar boundary. Never enabled by standalone lexical recognition.
use crate::{
    cursor::{CursorError, TokenCursor, TokenSource},
    token::TokenKind as K,
};

/// The statement parser must resolve its enclosing grammar scope before calling.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HaltScope {
    Outermost,
    Nested,
}
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum HaltError {
    #[error("__halt_compiler must appear in outermost scope")]
    Scope,
    #[error("invalid __halt_compiler syntax")]
    Syntax(K),
    #[error(transparent)]
    Cursor(#[from] CursorError),
}
/// Consume `__halt_compiler ( ) ;` (or a closing-tag terminator), then authorize
/// the terminal raw-tail state before requesting any token from that tail.
pub(crate) fn consume<L: TokenSource>(
    cursor: &mut TokenCursor<L>,
    scope: HaltScope,
) -> Result<(), HaltError> {
    if scope != HaltScope::Outermost {
        return Err(HaltError::Scope);
    }
    for expected in [K::HaltCompiler, K::LeftParen, K::RightParen] {
        if cursor.peek(0)?.1.kind() != expected {
            return Err(HaltError::Syntax(expected));
        }
        cursor.bump()?;
    }
    if !matches!(cursor.peek(0)?.1.kind(), K::Semicolon | K::CloseTag) {
        return Err(HaltError::Syntax(K::Semicolon));
    }
    cursor.bump()?;
    cursor.enter_halt_tail()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParseOptions, SourceText, lexer::scanner::Scanner};
    #[test]
    fn production_scanner_enters_raw_tail_only_after_validated_boundary() {
        for (source, tail) in [
            (
                &b"<?php __halt_compiler /*x*/ ( ) ;\0\xff'<?=raw"[..],
                &b"\0\xff'<?=raw"[..],
            ),
            (&b"<?php __halt_compiler() ?>\r\nraw"[..], &b"raw"[..]),
            (&b"<?php __halt_compiler();"[..], &b""[..]),
        ] {
            let source = SourceText::from(source);
            let options = ParseOptions::default();
            let scanner = Scanner::new(source.clone(), options);
            let mut cursor = TokenCursor::new(source, scanner, options.limits);
            // Open tags are trivia, so the cursor starts at the halt keyword.
            consume(&mut cursor, HaltScope::Outermost).unwrap();
            let (stream, stop) = cursor.finish();
            assert!(stop.is_none());
            let stream = stream.unwrap();
            let raw = stream
                .tokens()
                .iter()
                .position(|token| token.kind() == K::HaltTail);
            assert_eq!(
                raw.and_then(|index| stream.text(index)).unwrap_or_default(),
                tail
            );
        }
    }
    #[test]
    fn malformed_or_nested_halts_never_authorize_tail() {
        for source in [
            "<?php __halt_compiler x y z ; $after;",
            "<?php __halt_compiler(1); $after;",
            "<?php __halt_compiler() $after;",
        ] {
            let source = SourceText::from(source);
            let options = ParseOptions::default();
            let scanner = Scanner::new(source.clone(), options);
            let mut cursor = TokenCursor::new(source, scanner, options.limits);
            assert!(matches!(
                consume(&mut cursor, HaltScope::Outermost),
                Err(HaltError::Syntax(_))
            ));
            let (stream, stop) = cursor.finish();
            assert!(stop.is_none());
            let stream = stream.unwrap();
            assert!(!stream.tokens().iter().any(|t| t.kind() == K::HaltTail));
            assert!(
                stream
                    .tokens()
                    .iter()
                    .enumerate()
                    .any(|(i, _)| stream.text(i) == Some(&b"$after"[..]))
            );
        }
        let source = SourceText::from("<?php __halt_compiler(); $after;");
        let options = ParseOptions::default();
        let scanner = Scanner::new(source.clone(), options);
        let mut cursor = TokenCursor::new(source, scanner, options.limits);
        assert_eq!(
            consume(&mut cursor, HaltScope::Nested),
            Err(HaltError::Scope)
        );
        assert_eq!(cursor.position(), 0);
        assert!(
            !cursor
                .finish()
                .0
                .unwrap()
                .tokens()
                .iter()
                .any(|t| t.kind() == K::HaltTail)
        );
    }
}
