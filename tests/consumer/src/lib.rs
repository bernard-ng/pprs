//! Compile-check the public library dependency from an independent Cargo package.

pub use pprs;

use pprs::{
    ParseOptions, SourceText,
    ast::{Identifier, NodeMeta},
    source::RangeError,
};

/// Copy caller bytes into a source that outlives the input, then inspect EOF.
///
/// # Errors
/// Propagates a range-construction error.
pub fn retain_source(input: &[u8]) -> Result<SourceText, RangeError> {
    let source = SourceText::from(input);
    let eof = source.span(input.len()..input.len())?;
    assert_eq!(source.slice(eof), Some(&b""[..]));
    Ok(source)
}

/// Build options and a synthetic identifier through public types only.
///
/// # Panics
/// Panics only if the fixed, valid identifier is rejected by the library.
#[must_use]
pub fn configure() -> (ParseOptions, Identifier) {
    let identifier = Identifier::new("consumer", NodeMeta::synthetic())
        .expect("nonempty identifier without namespace separators");
    (ParseOptions::default(), identifier)
}

/// Tokenize a file using the production lexer, retaining owned output.
///
/// # Errors
/// Returns an input-size error before allocation for oversized input.
pub fn tokenize(input: &[u8]) -> Result<pprs::LexResult, pprs::lexer::InputTooLarge> {
    pprs::lex(input, ParseOptions::default())
}

/// Decode a constant quoted token without exposing a dependency's byte type.
///
/// # Errors
/// Returns an input-relative literal error for malformed spelling or escapes.
pub fn decode_string(raw: &[u8]) -> Result<pprs::ByteString, pprs::literal::LiteralError> {
    pprs::literal::decode_quoted(raw)
}

/// Parse PHP and inspect a clean, owned syntax tree through public APIs.
///
/// # Errors
/// Returns preflight, syntax or resource failures with their available context.
pub fn parse_and_count(input: &[u8]) -> Result<usize, pprs::StrictError> {
    let parsed = pprs::parse_strict(input, ParseOptions::default())?;
    Ok(parsed
        .result()
        .program()
        .map_or(0, |program| pprs::visit::walk(program).count()))
}

/// Inspect declared property hook names through the downstream public API.
///
/// # Errors
/// Returns preflight, syntax or resource failures.
pub fn property_hook_names(input: &[u8]) -> Result<Vec<pprs::ByteString>, pprs::StrictError> {
    let parsed = pprs::parse_strict(input, ParseOptions::default())?;
    Ok(parsed.result().program().map_or_else(Vec::new, |program| {
        pprs::visit::walk(program)
            .filter_map(|entry| match entry.node {
                pprs::visit::Node::PropertyHook(hook) => Some(hook.name.bytes().clone()),
                _ => None,
            })
            .collect()
    }))
}
