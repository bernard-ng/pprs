//! Explicit file/comment states around the regular Logos byte scanner.
use super::{LexStop, keywords::keyword, regular::Regular, strings::Mode};
use crate::{
    ParseOptions, PhpVersion, SourceText,
    cursor::TokenSource,
    diagnostic::{Diagnostic, DiagnosticBag, DiagnosticCode},
    token::{Token, TokenKind},
};
use logos::Logos;

pub(crate) struct Scanner {
    pub(super) source: SourceText,
    pub(super) options: ParseOptions,
    pub(super) offset: usize,
    php: bool,
    pub(super) property: bool,
    tail: bool,
    pub(super) modes: Vec<Mode>,
    diagnostics: DiagnosticBag,
}
impl Scanner {
    pub(crate) fn new(source: SourceText, options: ParseOptions) -> Self {
        Self {
            source,
            options,
            offset: 0,
            php: false,
            property: false,
            tail: false,
            modes: Vec::new(),
            diagnostics: DiagnosticBag::new(options.limits),
        }
    }
    pub(super) fn token(&mut self, kind: TokenKind, end: usize) -> Token {
        let span = self
            .source
            .span(self.offset..end)
            .expect("scanner ranges are checked against input");
        self.offset = end;
        if !kind.is_trivia() {
            self.property = matches!(kind, TokenKind::ObjectArrow | TokenKind::NullsafeArrow);
        }
        Token::new(kind, span)
    }
    pub(super) fn report(
        &mut self,
        code: DiagnosticCode,
        start: usize,
        end: usize,
        message: String,
    ) -> Result<(), LexStop> {
        let span = self
            .source
            .span(start..end)
            .expect("diagnostic lies inside the scanned input");
        self.diagnostics
            .push(Diagnostic::error(code, span, message))
            .map_err(LexStop::Limit)
    }
    fn html(&mut self) -> Token {
        let bytes = self.source.as_bytes();
        let start = self.offset;
        let mut next = start;
        while next < bytes.len() {
            if let Some((kind, width)) = open_tag(&bytes[next..], self.options.short_open_tag) {
                if next != start {
                    return self.token(TokenKind::InlineHtml, next);
                }
                self.php = true;
                return self.token(kind, next + width);
            }
            next += 1;
        }
        self.token(TokenKind::InlineHtml, next)
    }
    pub(super) fn php(&mut self) -> Result<Token, LexStop> {
        let start = self.offset;
        let rest = &self.source.as_bytes()[start..];
        if rest.starts_with(b"?>") {
            let end = start + 2 + newline_width(&rest[2..]);
            self.php = false;
            return Ok(self.token(TokenKind::CloseTag, end));
        }
        if rest.starts_with(b"/*") {
            let end = rest[2..]
                .windows(2)
                .position(|pair| pair == b"*/")
                .map(|at| start + at + 4);
            let kind = if rest.starts_with(b"/**") && rest.get(3).is_some_and(|b| whitespace(*b)) {
                TokenKind::DocComment
            } else {
                TokenKind::Comment
            };
            let end = if let Some(end) = end {
                end
            } else {
                let end = self.source.len();
                self.report(
                    DiagnosticCode::Unterminated,
                    start,
                    end,
                    "Unterminated comment".into(),
                )?;
                end
            };
            return Ok(self.token(kind, end));
        }
        if rest.starts_with(b"//")
            || (rest[0] == b'#' && (self.property || !rest.starts_with(b"#[")))
        {
            let mut end = start;
            let bytes = self.source.as_bytes();
            while end < bytes.len()
                && !matches!(bytes[end], b'\r' | b'\n')
                && !bytes[end..].starts_with(b"?>")
            {
                end += 1;
            }
            return Ok(self.token(TokenKind::Comment, end));
        }
        if let Some(token) = self.start_string()? {
            return Ok(token);
        }
        let rest = &self.source.as_bytes()[start..];
        let mut lexer = Regular::lexer(rest);
        let regular = lexer.next().expect("nonempty input produces a Logos token");
        let mut end = start + lexer.span().end;
        let mut kind = match regular {
            Ok(Regular::Word) => {
                let word = &self.source.as_bytes()[start..end];
                if self.property {
                    TokenKind::Identifier
                } else {
                    keyword(word, self.options.version)
                }
            }
            Ok(Regular::Name) if self.property && rest[0] != b'\\' => {
                end = start
                    + rest
                        .iter()
                        .position(|byte| *byte == b'\\')
                        .expect("qualified name has a separator");
                TokenKind::Identifier
            }
            Ok(Regular::Name) => {
                let name = &self.source.as_bytes()[start..end];
                if name.starts_with(b"\\") {
                    TokenKind::NameFullyQualified
                } else if name
                    .get(..10)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"namespace\\"))
                {
                    TokenKind::NameRelative
                } else {
                    TokenKind::NameQualified
                }
            }
            Ok(Regular::Integer) => number_kind(&self.source.as_bytes()[start..end]),
            Ok(regular) => regular.kind(),
            Err(()) => {
                // Every otherwise unrecognized byte gets its own source-preserving error.
                end = start + 1;
                let byte = self.source.as_bytes()[start];
                let message = if byte == 0 {
                    "Unexpected null byte".into()
                } else {
                    format!("Unexpected byte 0x{byte:02x} (ASCII {byte})")
                };
                self.report(DiagnosticCode::InvalidByte, start, end, message)?;
                TokenKind::Invalid
            }
        };
        let following = &self.source.as_bytes()[end..];
        if kind == TokenKind::Enum {
            let skipped = skip_trivia(following);
            if skipped == 0 || !following.get(skipped).is_some_and(|b| identifier_start(*b)) {
                kind = TokenKind::Identifier;
            }
        }
        if kind == TokenKind::Yield {
            let spaces = skip_trivia(following);
            let suffix = &following[spaces..];
            if spaces > 0
                && suffix
                    .get(..4)
                    .is_some_and(|part| part.eq_ignore_ascii_case(b"from"))
                && !suffix.get(4).is_some_and(|b| identifier_continue(*b))
            {
                kind = TokenKind::YieldFrom;
                end += spaces + 4;
            }
        }
        if self.options.version >= PhpVersion::Php84
            && following
                .get(..5)
                .is_some_and(|part| part.eq_ignore_ascii_case(b"(set)"))
        {
            let set = match kind {
                TokenKind::Public => Some(TokenKind::PublicSet),
                TokenKind::Protected => Some(TokenKind::ProtectedSet),
                TokenKind::Private => Some(TokenKind::PrivateSet),
                _ => None,
            };
            if let Some(set) = set {
                kind = set;
                end += 5;
            }
        }
        Ok(self.token(kind, end))
    }
}
impl TokenSource for Scanner {
    fn next_token(&mut self) -> Result<Token, LexStop> {
        if self.offset == self.source.len() {
            self.end_modes()?;
            return Ok(Token::new(TokenKind::Eof, self.source.eof()));
        }
        if self.tail {
            return Ok(self.token(TokenKind::HaltTail, self.source.len()));
        }
        if !self.php {
            Ok(self.html())
        } else if !self.modes.is_empty() {
            self.string_token()
        } else {
            self.php()
        }
    }
    fn enter_halt_tail(&mut self) {
        self.tail = true;
    }
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::replace(
            &mut self.diagnostics,
            DiagnosticBag::new(self.options.limits),
        )
        .into_vec()
    }
}

fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}
pub(super) fn identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}
pub(super) fn newline_width(bytes: &[u8]) -> usize {
    if bytes.starts_with(b"\r\n") {
        2
    } else {
        usize::from(bytes.first().is_some_and(|b| matches!(b, b'\n' | b'\r')))
    }
}
fn open_tag(bytes: &[u8], short: bool) -> Option<(TokenKind, usize)> {
    if bytes.starts_with(b"<?=") {
        return Some((TokenKind::OpenEchoTag, 3));
    }
    if bytes
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"<?php"))
    {
        if bytes.len() == 5 {
            return Some((TokenKind::OpenTag, 5));
        }
        if whitespace(bytes[5]) {
            return Some((TokenKind::OpenTag, 5 + newline_width(&bytes[5..]).max(1)));
        }
    }
    if short && bytes.starts_with(b"<?") {
        return Some((TokenKind::OpenTag, 2));
    }
    None
}

// Token classification only, not scalar interpretation. The reference's 64-bit
// tokenizer promotes overflow to T_DNUMBER. Conversion stops at an invalid octal
// digit, unless overflow has already promoted the token.
fn number_kind(bytes: &[u8]) -> TokenKind {
    let (radix, digits) = if bytes.starts_with(b"0x") || bytes.starts_with(b"0X") {
        (16_u32, &bytes[2..])
    } else if bytes.starts_with(b"0b") || bytes.starts_with(b"0B") {
        (2, &bytes[2..])
    } else if bytes.starts_with(b"0o") || bytes.starts_with(b"0O") {
        (8, &bytes[2..])
    } else if bytes.starts_with(b"0") {
        (8, bytes)
    } else {
        (10, bytes)
    };
    let mut value = 0_i64;
    for &byte in digits {
        if byte == b'_' {
            continue;
        }
        let Some(digit) = char::from(byte).to_digit(radix) else {
            break;
        };
        let Some(next) = value
            .checked_mul(i64::from(radix))
            .and_then(|value| value.checked_add(i64::from(digit)))
        else {
            return TokenKind::Float;
        };
        value = next;
    }
    TokenKind::Integer
}

pub(super) fn identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

// PHP 8.5 host-tokenizer lookahead retained by the pinned reference for every
// target: enum names and yield-from may have intervening comments.
fn skip_trivia(bytes: &[u8]) -> usize {
    let mut offset = 0;
    loop {
        offset += bytes[offset..]
            .iter()
            .take_while(|b| whitespace(**b))
            .count();
        let rest = &bytes[offset..];
        if rest.starts_with(b"/*") {
            let Some(end) = rest[2..].windows(2).position(|pair| pair == b"*/") else {
                return bytes.len();
            };
            offset += end + 4;
        } else if rest.starts_with(b"//") || (rest.starts_with(b"#") && !rest.starts_with(b"#[")) {
            while offset < bytes.len()
                && !matches!(bytes[offset], b'\n' | b'\r')
                && !bytes[offset..].starts_with(b"?>")
            {
                offset += 1;
            }
            if bytes[offset..].starts_with(b"?>") {
                return offset;
            }
        } else {
            return offset;
        }
    }
}
