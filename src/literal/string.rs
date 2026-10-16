use super::{LiteralError, LiteralErrorKind as E};
use crate::ByteString;

/// Escape rules for a raw string segment; no delimiters are included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeMode {
    /// Only escaped single quotes and backslashes are interpreted.
    SingleQuoted,
    /// PHP double-quoted escapes, including escaped double quotes.
    DoubleQuoted,
    /// PHP shell-command escapes, including escaped backticks.
    Backtick,
    /// PHP heredoc escapes; quotes have no special treatment.
    Heredoc,
    /// No escape interpretation.
    Nowdoc,
}

/// Decode a complete single/double-quoted token with an optional `b`/`B` prefix.
/// Interpolation is not evaluated; use this only for a `QuotedString` token.
///
/// # Errors
/// Returns an error for unmatched delimiters, unescaped interior quotes or invalid Unicode escapes.
pub fn decode_quoted(raw: &[u8]) -> Result<ByteString, LiteralError> {
    let prefix = usize::from(raw.first().is_some_and(|b| matches!(b, b'b' | b'B')));
    let Some(&quote) = raw.get(prefix) else {
        return Err(LiteralError::new(E::InvalidSpelling, 0..raw.len()));
    };
    if !matches!(quote, b'\'' | b'"') || raw.len() < prefix + 2 || raw.last() != Some(&quote) {
        return Err(LiteralError::new(E::InvalidSpelling, 0..raw.len()));
    }
    let body = &raw[prefix + 1..raw.len() - 1];
    let mut at = 0;
    while at < body.len() {
        if body[at] == quote {
            return Err(LiteralError::new(
                E::InvalidSpelling,
                prefix + 1 + at..prefix + 2 + at,
            ));
        }
        if body[at] == b'\\' {
            at += 1;
        }
        at += 1;
    }
    if at > body.len() {
        return Err(LiteralError::new(
            E::InvalidSpelling,
            raw.len() - 2..raw.len(),
        ));
    }
    decode_escapes(
        body,
        if quote == b'\'' {
            EscapeMode::SingleQuoted
        } else {
            EscapeMode::DoubleQuoted
        },
    )
    .map_err(|mut error| {
        error.range.start += prefix + 1;
        error.range.end += prefix + 1;
        error
    })
}

/// Decode raw bytes under the selected PHP escape rules. Unknown escapes remain
/// unchanged. The frozen reference accepts surrogate code points and encodes them
/// as three bytes; output is therefore bytes, not necessarily valid UTF-8.
///
/// # Errors
/// Returns the first malformed/out-of-range Unicode escape with an input-relative range.
pub fn decode_escapes(bytes: &[u8], mode: EscapeMode) -> Result<ByteString, LiteralError> {
    if mode == EscapeMode::Nowdoc {
        return Ok(bytes.into());
    }
    if mode == EscapeMode::SingleQuoted {
        return Ok(single_quoted(bytes).into());
    }
    let (prepared, origins) = prepare_quote(bytes, mode);
    decode_prepared(bytes, &prepared, &origins)
}
fn single_quoted(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'\\'
            && bytes
                .get(at + 1)
                .is_some_and(|b| matches!(b, b'\\' | b'\''))
        {
            at += 1;
        }
        out.push(bytes[at]);
        at += 1;
    }
    out
}
fn prepare_quote(bytes: &[u8], mode: EscapeMode) -> (Vec<u8>, Vec<usize>) {
    // Match the reference's delimiter replacement before escape processing.
    let quote = match mode {
        EscapeMode::DoubleQuoted => Some(b'"'),
        EscapeMode::Backtick => Some(b'`'),
        _ => None,
    };
    let mut prepared = Vec::with_capacity(bytes.len());
    let mut origins = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let origin = at;
        if bytes[at] == b'\\' && quote.is_some() && bytes.get(at + 1) == quote.as_ref() {
            at += 1;
        }
        prepared.push(bytes[at]);
        origins.push(origin);
        at += 1;
    }
    (prepared, origins)
}
fn decode_prepared(
    bytes: &[u8],
    prepared: &[u8],
    origins: &[usize],
) -> Result<ByteString, LiteralError> {
    let mut at = 0;
    let mut out = Vec::with_capacity(prepared.len());
    while at < prepared.len() {
        if prepared[at] != b'\\' || at + 1 == prepared.len() {
            out.push(prepared[at]);
            at += 1;
            continue;
        }
        let start = at;
        let next = prepared[at + 1];
        let replacement = match next {
            b'\\' => Some(b'\\'),
            b'$' => Some(b'$'),
            b'n' => Some(b'\n'),
            b'r' => Some(b'\r'),
            b't' => Some(b'\t'),
            b'f' => Some(12),
            b'v' => Some(11),
            b'e' => Some(27),
            _ => None,
        };
        if let Some(byte) = replacement {
            out.push(byte);
            at += 2;
            continue;
        }
        if matches!(next, b'x' | b'X') && prepared.get(at + 2).is_some_and(u8::is_ascii_hexdigit) {
            at += 2;
            let mut value = 0;
            for _ in 0..2 {
                if let Some(digit) = prepared.get(at).and_then(|b| char::from(*b).to_digit(16)) {
                    value = value * 16 + digit;
                    at += 1;
                } else {
                    break;
                }
            }
            out.push(u8::try_from(value).expect("two hex digits fit a byte"));
            continue;
        }
        if matches!(next, b'0'..=b'7') {
            at += 1;
            let mut value = 0_u16;
            for _ in 0..3 {
                if let Some(&digit @ b'0'..=b'7') = prepared.get(at) {
                    value = value * 8 + u16::from(digit - b'0');
                    at += 1;
                } else {
                    break;
                }
            }
            out.push(u8::try_from(value & 255).expect("masked to one byte"));
            continue;
        }
        if next == b'u' && prepared.get(at + 2) == Some(&b'{') {
            at += 3;
            let digits_start = at;
            let mut value = Some(0_u32);
            while let Some(digit) = prepared.get(at).and_then(|b| char::from(*b).to_digit(16)) {
                value = value
                    .and_then(|v| v.checked_mul(16))
                    .and_then(|v| v.checked_add(digit));
                at += 1;
            }
            if at == digits_start
                || prepared.get(at) != Some(&b'}')
                || value.is_none_or(|v| v > 0x0010_ffff)
            {
                return Err(LiteralError::new(
                    E::UnicodeEscape,
                    origins[start]..origins.get(at + 1).copied().unwrap_or(bytes.len()),
                ));
            }
            encode_codepoint(value.unwrap(), &mut out);
            at += 1;
            continue;
        }
        out.push(b'\\');
        at += 1;
    }
    Ok(out.into())
}
fn encode_codepoint(value: u32, out: &mut Vec<u8>) {
    // Deliberately includes surrogate code points, matching the byte-oriented reference.
    if value <= 0x7f {
        out.push(u8::try_from(value).unwrap());
    } else {
        let (count, head) = if value <= 0x7ff {
            (1, 0xc0)
        } else if value <= 0xffff {
            (2, 0xe0)
        } else {
            (3, 0xf0)
        };
        out.push(head | u8::try_from(value >> (count * 6)).unwrap());
        for shift in (0..count).rev() {
            out.push(0x80 | u8::try_from((value >> (shift * 6)) & 63).unwrap());
        }
    }
}

/// Strip flexible doc-string indentation, remove the final body newline when
/// `last` is true, and decode escapes. `first`/`last` refer to the complete body,
/// allowing a parser to decode text segments separated by interpolated expressions.
/// Empty first segments validate interpolation occurring before required indentation.
///
/// # Errors
/// Returns the first indentation or escape error, relative to `body`. A mixed or
/// non-whitespace closing indentation is rejected with an empty range.
pub fn decode_doc_segment(
    body: &[u8],
    indentation: &[u8],
    first: bool,
    last: bool,
    nowdoc: bool,
) -> Result<ByteString, LiteralError> {
    let indent = indentation.first().copied().unwrap_or(b' ');
    if !matches!(indent, b' ' | b'\t') || indentation.iter().any(|b| *b != indent) {
        return Err(LiteralError::new(E::MixedIndentation, 0..0));
    }
    let mut stripped = Vec::with_capacity(body.len());
    let mut origins = Vec::with_capacity(body.len());
    let mut at = 0;
    loop {
        if !indentation.is_empty() && ((at == 0 && first) || (at > 0 && body[at - 1] == b'\n')) {
            let start = at;
            while at < body.len()
                && at - start < indentation.len()
                && matches!(body[at], b' ' | b'\t')
            {
                if body[at] != indent {
                    return Err(LiteralError::new(E::MixedIndentation, start..at + 1));
                }
                at += 1;
            }
            let blank = body.get(at).is_some_and(|b| matches!(b, b'\r' | b'\n'))
                || (at == body.len() && last);
            if at - start < indentation.len() && !blank {
                return Err(LiteralError::new(E::InsufficientIndentation, start..at));
            }
        }
        if at == body.len() {
            break;
        }
        stripped.push(body[at]);
        origins.push(at);
        at += 1;
    }
    if last {
        if stripped.ends_with(b"\r\n") {
            stripped.truncate(stripped.len() - 2);
        } else if stripped.last().is_some_and(|b| matches!(b, b'\r' | b'\n')) {
            stripped.pop();
        }
    }
    decode_escapes(
        &stripped,
        if nowdoc {
            EscapeMode::Nowdoc
        } else {
            EscapeMode::Heredoc
        },
    )
    .map_err(|mut error| {
        error.range = origins
            .get(error.range.start)
            .copied()
            .unwrap_or(body.len())
            ..error
                .range
                .end
                .checked_sub(1)
                .and_then(|end| origins.get(end))
                .map_or(body.len(), |end| end + 1);
        error
    })
}
