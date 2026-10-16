//! Explicit, bounded string/interpolation states. No input-driven recursion.
use super::{
    LexStop,
    regular::Regular,
    scanner::{Scanner, identifier_continue, identifier_start, newline_width},
};
use crate::{
    diagnostic::DiagnosticCode,
    options::Resource,
    token::{Token, TokenKind as K},
};
use logos::Logos;

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Text { delimiter: Delimiter, start: usize },
    Expression { braces: usize, varname: bool },
    AfterVariable,
    Property,
    Offset,
}
#[derive(Clone, Copy)]
pub(super) enum Delimiter {
    Quote(u8),
    Doc {
        label_start: usize,
        label_end: usize,
        body_start: usize,
        nowdoc: bool,
    },
}
impl Scanner {
    fn nesting_depth(&self) -> usize {
        self.modes
            .iter()
            .map(|mode| match mode {
                Mode::Expression { braces, .. } => braces + 1,
                _ => 1,
            })
            .sum()
    }

    fn push_mode(&mut self, mode: Mode) -> Result<(), LexStop> {
        if self.nesting_depth() >= self.options.limits.get(Resource::Depth) {
            self.report(
                DiagnosticCode::ResourceLimit(Resource::Depth),
                self.offset,
                self.offset,
                "string nesting depth exhausted".into(),
            )?;
            return Err(LexStop::Limit(Resource::Depth));
        }
        self.modes.push(mode);
        Ok(())
    }
    pub(super) fn end_modes(&mut self) -> Result<(), LexStop> {
        if let Some(start) = self.modes.iter().find_map(|mode| match mode {
            Mode::Text { start, .. } => Some(*start),
            _ => None,
        }) {
            self.modes.clear();
            self.report(
                DiagnosticCode::UnterminatedString,
                start,
                self.source.len(),
                "Unterminated string or interpolation".into(),
            )?;
        }
        Ok(())
    }
    pub(super) fn start_string(&mut self) -> Result<Option<Token>, LexStop> {
        let start = self.offset;
        let bytes = self.source.as_bytes();
        let binary = usize::from(matches!(bytes[start], b'b' | b'B'));
        let quote_at = start + binary;
        let Some(&quote) = bytes.get(quote_at) else {
            return Ok(None);
        };
        if bytes[quote_at..].starts_with(b"<<<") {
            if let Some((label_start, label_end, end, nowdoc)) = doc_opener(bytes, quote_at) {
                self.push_mode(Mode::Text {
                    delimiter: Delimiter::Doc {
                        label_start,
                        label_end,
                        body_start: end,
                        nowdoc,
                    },
                    start,
                })?;
                return Ok(Some(self.token(K::StartHeredoc, end)));
            }
        }
        if !matches!(quote, b'\'' | b'"' | b'`') || (binary == 1 && quote == b'`') {
            return Ok(None);
        }
        if quote != b'`' {
            let mut end = quote_at + 1;
            while end < bytes.len() {
                if bytes[end] == b'\\' {
                    end = (end + 2).min(bytes.len());
                    continue;
                }
                if bytes[end] == quote {
                    return Ok(Some(self.token(K::QuotedString, end + 1)));
                }
                if quote == b'"' && interpolation(bytes, end) {
                    break;
                }
                end += 1;
            }
            if quote == b'\'' {
                let end = bytes.len();
                self.report(
                    DiagnosticCode::UnterminatedString,
                    start,
                    end,
                    "Unterminated single-quoted string".into(),
                )?;
                return Ok(Some(self.token(K::StringPart, end)));
            }
        }
        self.push_mode(Mode::Text {
            delimiter: Delimiter::Quote(quote),
            start,
        })?;
        Ok(Some(self.token(
            if quote == b'`' {
                K::Backtick
            } else {
                K::DoubleQuote
            },
            quote_at + 1,
        )))
    }
    pub(super) fn string_token(&mut self) -> Result<Token, LexStop> {
        loop {
            match *self.modes.last().expect("string state present") {
                Mode::Text { delimiter, .. } => return self.text_token(delimiter),
                Mode::AfterVariable => {
                    self.modes.pop();
                    let rest = &self.source.as_bytes()[self.offset..];
                    if rest[0] == b'[' {
                        self.push_mode(Mode::Offset)?;
                        return Ok(self.token(K::LeftBracket, self.offset + 1));
                    }
                    let width = if rest.starts_with(b"?->") {
                        3
                    } else if rest.starts_with(b"->") {
                        2
                    } else {
                        0
                    };
                    if width > 0 && rest.get(width).is_some_and(|b| identifier_start(*b)) {
                        self.push_mode(Mode::Property)?;
                        return Ok(self.token(
                            if width == 3 {
                                K::NullsafeArrow
                            } else {
                                K::ObjectArrow
                            },
                            self.offset + width,
                        ));
                    }
                }
                Mode::Property => {
                    self.modes.pop();
                    let end = word_end(self.source.as_bytes(), self.offset);
                    return Ok(self.token(K::Identifier, end));
                }
                Mode::Offset => {
                    if let Some(token) = self.offset_token()? {
                        return Ok(token);
                    }
                    self.modes.pop();
                }
                Mode::Expression { braces, varname } => {
                    let bytes = self.source.as_bytes();
                    if varname {
                        *self.modes.last_mut().unwrap() = Mode::Expression {
                            braces,
                            varname: false,
                        };
                        if identifier_start(bytes[self.offset]) {
                            let end = word_end(bytes, self.offset);
                            if bytes.get(end).is_some_and(|b| matches!(b, b'}' | b'[')) {
                                return Ok(self.token(K::StringVarname, end));
                            }
                        }
                    }
                    match bytes[self.offset] {
                        b'}' => {
                            if braces == 0 {
                                self.modes.pop();
                            } else {
                                *self.modes.last_mut().unwrap() = Mode::Expression {
                                    braces: braces - 1,
                                    varname: false,
                                };
                            }
                            return Ok(self.token(K::RightBrace, self.offset + 1));
                        }
                        b'{' => {
                            if self.nesting_depth() >= self.options.limits.get(Resource::Depth) {
                                self.report(
                                    DiagnosticCode::ResourceLimit(Resource::Depth),
                                    self.offset,
                                    self.offset,
                                    "interpolation nesting depth exhausted".into(),
                                )?;
                                return Err(LexStop::Limit(Resource::Depth));
                            }
                            *self.modes.last_mut().unwrap() = Mode::Expression {
                                braces: braces + 1,
                                varname: false,
                            };
                            return Ok(self.token(K::LeftBrace, self.offset + 1));
                        }
                        _ => return self.php(),
                    }
                }
            }
        }
    }
    fn text_token(&mut self, delimiter: Delimiter) -> Result<Token, LexStop> {
        let start = self.offset;
        let bytes = self.source.as_bytes();
        let nowdoc = matches!(delimiter, Delimiter::Doc { nowdoc: true, .. });
        let mut end = start;
        while end < bytes.len() {
            if let Some(close_end) = closing(bytes, end, delimiter) {
                if end != start {
                    break;
                }
                self.modes.pop();
                let kind = match delimiter {
                    Delimiter::Quote(b'`') => K::Backtick,
                    Delimiter::Quote(_) => K::DoubleQuote,
                    Delimiter::Doc { .. } => K::EndHeredoc,
                };
                return Ok(self.token(kind, close_end));
            }
            if !nowdoc && interpolation(bytes, end) {
                if end != start {
                    break;
                }
                if bytes[end..].starts_with(b"${") {
                    self.push_mode(Mode::Expression {
                        braces: 0,
                        varname: true,
                    })?;
                    self.property = false;
                    return Ok(self.token(K::DollarOpenCurly, end + 2));
                }
                if bytes[end] == b'{' {
                    self.push_mode(Mode::Expression {
                        braces: 0,
                        varname: false,
                    })?;
                    self.property = false;
                    return Ok(self.token(K::CurlyOpen, end + 1));
                }
                let end = word_end(bytes, end + 1);
                self.push_mode(Mode::AfterVariable)?;
                return Ok(self.token(K::Variable, end));
            }
            // A backslash quotes the next byte for boundary detection, but must
            // not hide a heredoc label after an escaped newline.
            if !nowdoc
                && bytes[end] == b'\\'
                && bytes
                    .get(end + 1)
                    .is_some_and(|b| !matches!(b, b'\r' | b'\n'))
            {
                end += 1;
            }
            end += 1;
        }
        Ok(self.token(K::StringPart, end))
    }
    fn offset_token(&mut self) -> Result<Option<Token>, LexStop> {
        let start = self.offset;
        let bytes = self.source.as_bytes();
        let byte = bytes[start];
        let (kind, end) = if byte == b']' {
            self.modes.pop();
            (K::RightBracket, start + 1)
        } else if identifier_start(byte) {
            (K::Identifier, word_end(bytes, start))
        } else if byte == b'$' && bytes.get(start + 1).is_some_and(|b| identifier_start(*b)) {
            (K::Variable, word_end(bytes, start + 1))
        } else if byte.is_ascii_digit() {
            (K::NumString, offset_number_end(bytes, start))
        } else {
            if matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'\\' | b'\'') || byte >= 0x80 {
                return Ok(None);
            }
            let kind = match byte {
                b'"' => K::DoubleQuote,
                b'`' => K::Backtick,
                _ => {
                    let mut lexer = Regular::lexer(&bytes[start..=start]);
                    match lexer.next().expect("nonempty offset byte") {
                        Ok(regular) => regular.kind(),
                        Err(()) => K::Invalid,
                    }
                }
            };
            (kind, start + 1)
        };
        if kind == K::Invalid {
            self.report(
                DiagnosticCode::InvalidByte,
                start,
                end,
                format!("Unexpected byte 0x{byte:02x}"),
            )?;
        }
        Ok(Some(self.token(kind, end)))
    }
}
fn word_end(bytes: &[u8], mut offset: usize) -> usize {
    while offset < bytes.len() && identifier_continue(bytes[offset]) {
        offset += 1;
    }
    offset
}
fn interpolation(bytes: &[u8], at: usize) -> bool {
    (bytes[at] == b'$'
        && bytes
            .get(at + 1)
            .is_some_and(|b| *b == b'{' || identifier_start(*b)))
        || (bytes[at] == b'{' && bytes.get(at + 1) == Some(&b'$'))
}
fn doc_opener(bytes: &[u8], at: usize) -> Option<(usize, usize, usize, bool)> {
    let mut pos = at + 3;
    while bytes.get(pos).is_some_and(|b| matches!(b, b' ' | b'\t')) {
        pos += 1;
    }
    let quote = bytes
        .get(pos)
        .copied()
        .filter(|b| matches!(b, b'\'' | b'"'));
    pos += usize::from(quote.is_some());
    if !bytes.get(pos).is_some_and(|b| identifier_start(*b)) {
        return None;
    }
    let start = pos;
    pos = word_end(bytes, pos);
    let end = pos;
    if let Some(quote) = quote {
        if bytes.get(pos) != Some(&quote) {
            return None;
        }
        pos += 1;
    }
    let newline = newline_width(&bytes[pos..]);
    (newline > 0).then_some((start, end, pos + newline, quote == Some(b'\'')))
}
fn closing(bytes: &[u8], at: usize, delimiter: Delimiter) -> Option<usize> {
    match delimiter {
        Delimiter::Quote(quote) => (bytes[at] == quote).then_some(at + 1),
        Delimiter::Doc {
            label_start,
            label_end,
            body_start,
            ..
        } => {
            if at != body_start && (at == 0 || !matches!(bytes[at - 1], b'\r' | b'\n')) {
                return None;
            }
            let mut pos = at;
            while bytes.get(pos).is_some_and(|b| matches!(b, b' ' | b'\t')) {
                pos += 1;
            }
            let label = &bytes[label_start..label_end];
            let end = pos + label.len();
            (bytes[pos..].starts_with(label)
                && !bytes.get(end).is_some_and(|b| identifier_continue(*b)))
            .then_some(end)
        }
    }
}

fn offset_number_end(bytes: &[u8], start: usize) -> usize {
    let (radix, mut pos) = match bytes.get(start..start + 2) {
        Some(b"0x" | b"0X") if bytes.get(start + 2).is_some_and(u8::is_ascii_hexdigit) => {
            (16, start + 2)
        }
        Some(b"0b" | b"0B")
            if bytes
                .get(start + 2)
                .is_some_and(|b| matches!(b, b'0' | b'1')) =>
        {
            (2, start + 2)
        }
        Some(b"0o" | b"0O")
            if bytes
                .get(start + 2)
                .is_some_and(|b| matches!(b, b'0'..=b'7')) =>
        {
            (8, start + 2)
        }
        _ => (10, start),
    };
    loop {
        while bytes
            .get(pos)
            .is_some_and(|b| char::from(*b).is_digit(radix))
        {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'_')
            && bytes
                .get(pos + 1)
                .is_some_and(|b| char::from(*b).is_digit(radix))
        {
            pos += 1;
        } else {
            break;
        }
    }
    pos
}
