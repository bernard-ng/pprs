//! Literal semantics compared to frozen reference utility/parser observations.
use pprs::{
    ByteString,
    literal::{self, EscapeMode, LiteralError, LiteralErrorKind as E, Number, NumericOffset},
};
use serde_json::{Value, json};
fn bytes(value: &Value) -> Vec<u8> {
    hex::decode(value["bytes"].as_str().unwrap()).unwrap()
}
fn encoded(result: Result<ByteString, LiteralError>) -> Value {
    match result {
        Ok(value) => json!({"bytes":hex::encode(value.as_bytes())}),
        Err(error) => error_value(&error),
    }
}
fn error_value(error: &LiteralError) -> Value {
    json!({"error":match error.kind {E::UnicodeEscape=>"unicode",E::MixedIndentation=>"mixed-indent",E::InsufficientIndentation=>"short-indent",E::InvalidOctal=>"octal",other=>panic!("unexpected {other:?}")}})
}
#[test]
fn frozen_literal_values_and_errors() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/literal/values.json")).unwrap();
    for case in document["cases"].as_array().unwrap() {
        let raw = bytes(&case["raw"]);
        let actual = match case["kind"].as_str().unwrap() {
            "quoted" => encoded(literal::decode_quoted(&raw)),
            "escapes" => encoded(literal::decode_escapes(
                &raw,
                match case["mode"].as_str().unwrap() {
                    "single" => EscapeMode::SingleQuoted,
                    "double" => EscapeMode::DoubleQuoted,
                    "backtick" => EscapeMode::Backtick,
                    "heredoc" => EscapeMode::Heredoc,
                    "nowdoc" => EscapeMode::Nowdoc,
                    _ => unreachable!(),
                },
            )),
            "doc" => encoded(literal::decode_doc_segment(
                &raw,
                &bytes(&case["indent"]),
                true,
                true,
                case["nowdoc"].as_bool().unwrap(),
            )),
            "number" => match literal::decode_number(&raw) {
                Ok(Number::Integer { value, .. }) => json!({"integer":value.to_string()}),
                Ok(Number::Float(value)) => json!({"float64":format!("{:016x}",value.to_bits())}),
                Err(error) => error_value(&error),
            },
            "offset" => match literal::decode_numeric_offset(&raw) {
                NumericOffset::Integer(value) => json!({"integer":value.to_string()}),
                NumericOffset::String(value) => json!({"bytes":hex::encode(value.as_bytes())}),
            },
            _ => unreachable!(),
        };
        assert_eq!(actual, case["expected"], "{}: {:?}", case["id"], raw);
    }
}
#[test]
fn literal_oracle_provenance() {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc: Value = serde_json::from_str(include_str!("fixtures/literal/values.json")).unwrap();
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    assert_eq!(doc["reference_commit"], lock["commit"]);
    assert_eq!(doc["php_version"], "8.5.6");
    assert_eq!(doc["php_int_size"], 8);
    for (path, digest) in doc["sources_sha256"].as_object().unwrap() {
        assert_eq!(
            digest,
            &hex::encode(Sha256::digest(std::fs::read(root.join(path)).unwrap()))
        );
    }
    let inputs: Value = serde_json::from_str(include_str!("../tools/literal/inputs.json")).unwrap();
    assert_eq!(
        doc["cases"].as_array().unwrap().len(),
        inputs.as_array().unwrap().len()
    );
    for (actual, input) in doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(inputs.as_array().unwrap())
    {
        let mut actual = actual.clone();
        actual.as_object_mut().unwrap().remove("expected");
        assert_eq!(actual, *input);
    }
}
#[test]
fn invalid_spellings_and_segment_relative_errors() {
    for raw in ["", "'", "b'", "'x", "'a'b'", "'abc\\'", "x'abc'"] {
        assert_eq!(
            literal::decode_quoted(raw.as_bytes()).unwrap_err().kind,
            E::InvalidSpelling
        );
    }
    for raw in [
        "", "0x", "1e+", "1__2", "+1", "-1", "1 ", "1;", "NaN", "INF",
    ] {
        assert_eq!(
            literal::decode_number(raw.as_bytes()).unwrap_err().kind,
            E::InvalidSpelling
        );
    }
    let error = literal::decode_quoted(br#"b"xx\u{110000}""#).unwrap_err();
    assert_eq!(error.kind, E::UnicodeEscape);
    assert_eq!(error.range, 4..14);
    let error =
        literal::decode_doc_segment(b"  a\n  \\u{}\n", b"  ", true, true, false).unwrap_err();
    assert_eq!(error.kind, E::UnicodeEscape);
    assert_eq!(error.range, 6..10);
    // The parser passes empty leading segments to reject interpolation before indentation.
    assert_eq!(
        literal::decode_doc_segment(b"", b" ", true, false, false)
            .unwrap_err()
            .kind,
        E::InsufficientIndentation
    );
    assert_eq!(
        literal::decode_doc_segment(b"  a", b"  ", true, false, false)
            .unwrap()
            .as_bytes(),
        b"a"
    );
    assert_eq!(
        literal::decode_doc_segment(b" b\n  c\n", b"  ", false, true, false)
            .unwrap()
            .as_bytes(),
        b" b\nc"
    );
}
use proptest::prelude::*;
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn arbitrary_literal_bytes_are_bounded(bytes in prop::collection::vec(any::<u8>(),0..4096)) {
        for mode in [EscapeMode::SingleQuoted,EscapeMode::DoubleQuoted,EscapeMode::Backtick,EscapeMode::Heredoc,EscapeMode::Nowdoc] {
            match literal::decode_escapes(&bytes,mode) {
                Ok(value)=>prop_assert!(value.as_bytes().len()<=bytes.len()),
                Err(error)=>prop_assert!(error.range.start<=error.range.end && error.range.end<=bytes.len()),
            }
        }
        for result in [literal::decode_quoted(&bytes),literal::decode_doc_segment(&bytes,b"  ",true,true,false)] {
            match result {Ok(value)=>prop_assert!(value.as_bytes().len()<=bytes.len()),Err(error)=>prop_assert!(error.range.start<=error.range.end && error.range.end<=bytes.len())}
        }
        if let Err(error)=literal::decode_number(&bytes) {prop_assert!(error.range.end<=bytes.len());}
    }
}
