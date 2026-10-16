use super::{LiteralError, LiteralErrorKind as E};
use crate::{ByteString, ast::IntegerBase, lexer::regular::Regular};
use logos::Logos;

/// PHP's signed 64-bit integer / IEEE-754 binary64 numeric model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    /// Integer and its original spelling base.
    Integer {
        /// Exact signed value; unary minus is a separate expression.
        value: i64,
        /// Spelling base.
        base: IntegerBase,
    },
    /// Floating-point literal or integer overflow, including positive infinity.
    Float(f64),
}
/// Decode one unsigned numeric token; unary signs belong to the parser. Malformed
/// octals report errors unless the valid prefix already overflowed to a float,
/// matching the reference's token-dependent conversion.
///
/// # Errors
/// Returns an error for incomplete/non-numeric spellings or invalid integer octals.
#[allow(
    clippy::cast_precision_loss,
    reason = "PHP explicitly promotes 64-bit integers to binary64 on overflow"
)]
pub fn decode_number(raw: &[u8]) -> Result<Number, LiteralError> {
    let invalid = || LiteralError::new(E::InvalidSpelling, 0..raw.len());
    let mut lexer = Regular::lexer(raw);
    let kind = lexer.next().and_then(Result::ok).ok_or_else(invalid)?;
    if lexer.span().end != raw.len() || !matches!(kind, Regular::Integer | Regular::Float) {
        return Err(invalid());
    }
    let clean: Vec<_> = raw.iter().copied().filter(|b| *b != b'_').collect();
    if matches!(kind, Regular::Float) {
        return std::str::from_utf8(&clean)
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .map(Number::Float)
            .ok_or_else(invalid);
    }
    let (radix, base, digits) = if clean.starts_with(b"0x") || clean.starts_with(b"0X") {
        (16, IntegerBase::Hexadecimal, &clean[2..])
    } else if clean.starts_with(b"0b") || clean.starts_with(b"0B") {
        (2, IntegerBase::Binary, &clean[2..])
    } else if clean.starts_with(b"0o") || clean.starts_with(b"0O") {
        (8, IntegerBase::Octal, &clean[2..])
    } else if clean.len() > 1 && clean[0] == b'0' {
        (8, IntegerBase::Octal, clean.as_slice())
    } else {
        (10, IntegerBase::Decimal, clean.as_slice())
    };
    let mut integer = Some(0_i64);
    let mut float = 0.0;
    for &byte in digits {
        let Some(digit) = char::from(byte).to_digit(radix) else {
            if integer.is_some() {
                return Err(LiteralError::new(E::InvalidOctal, 0..raw.len()));
            }
            break;
        };
        if let Some(value) = integer {
            integer = value
                .checked_mul(i64::from(radix))
                .and_then(|v| v.checked_add(i64::from(digit)));
            if integer.is_none() {
                float = value as f64 * f64::from(radix) + f64::from(digit);
            }
        } else {
            float = float * f64::from(radix) + f64::from(digit);
        }
    }
    if let Some(value) = integer {
        Ok(Number::Integer { value, base })
    } else if radix == 10 {
        std::str::from_utf8(&clean)
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .map(Number::Float)
            .ok_or_else(invalid)
    } else {
        Ok(Number::Float(float))
    }
}
/// Value of a simple numeric-looking interpolation offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericOffset {
    /// Canonical decimal spelling within the signed 64-bit range.
    Integer(i64),
    /// Noncanonical or overflowing spelling retained byte for byte.
    String(ByteString),
}
/// Interpret a simple interpolated array offset: only canonical decimal integers
/// within `i64` become integers. Leading zeroes, `-0`, prefixes and overflow remain
/// byte strings. This performs no PHP expression evaluation.
#[must_use]
pub fn decode_numeric_offset(raw: &[u8]) -> NumericOffset {
    if raw == b"0" {
        return NumericOffset::Integer(0);
    }
    let digits = raw.strip_prefix(b"-").unwrap_or(raw);
    if digits.first().is_some_and(|b| matches!(b, b'1'..=b'9'))
        && digits.iter().all(u8::is_ascii_digit)
    {
        if let Some(value) = std::str::from_utf8(raw).ok().and_then(|s| s.parse().ok()) {
            return NumericOffset::Integer(value);
        }
    }
    NumericOffset::String(raw.into())
}
