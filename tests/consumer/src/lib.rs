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
