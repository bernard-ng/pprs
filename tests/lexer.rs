//! Production lexer comparisons and source-preservation checks; no PHP needed.
#[path = "support/lexer.rs"]
mod adapter;
use pprs::{ParseOptions, lex};
use serde_json::{Value, json};

#[test]
fn dedicated_regular_lexer_oracles() {
    let mut failures = Vec::new();
    for data in [
        include_str!("fixtures/lexer/regular.json"),
        include_str!("fixtures/lexer/short-tags.json"),
    ] {
        let document: Value = serde_json::from_str(data).unwrap();
        for case in document["cases"].as_array().unwrap() {
            let input = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
            let target = case["target"].as_str().unwrap();
            let options = ParseOptions {
                version: target.parse().unwrap(),
                short_open_tag: case["short_open_tag"].as_bool().unwrap(),
                ..ParseOptions::default()
            };
            let result = lex(&input, options).unwrap();
            assert!(result.is_complete(), "{}: {:?}", case["id"], result.stop());
            let actual = adapter::observation(&result, target);
            if actual["tokens"] != case["oracle"]["tokens"] {
                let actual = actual["tokens"].as_array().unwrap();
                let expected = case["oracle"]["tokens"].as_array().unwrap();
                let index = actual
                    .iter()
                    .zip(expected)
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                failures.push(format!(
                    "{} token {index}: {:?} vs {:?}",
                    case["id"],
                    actual.get(index),
                    expected.get(index)
                ));
            }
            let expected: Vec<_> = case["oracle"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .map(|diagnostic| json!({"code":diagnostic["code"],"span":diagnostic["span"]}))
                .collect();
            assert_eq!(adapter::diagnostics(&result), expected, "{}", case["id"]);
        }
    }
    assert_eq!(failures.len(), 0, "{}", failures.join("\n"));
}

#[test]
fn complete_regular_corpus_tokens_match_before_coverage_promotion() {
    let mut complete = 0;
    let mut failures = Vec::new();
    for line in include_str!("fixtures/upstream/cases.jsonl").lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        if case["scope"] != "core" {
            continue;
        }
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let target = case["oracle"]["target"].as_str().unwrap();
        let result = lex(
            source,
            ParseOptions {
                version: target.parse().unwrap(),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        if result.is_complete() {
            complete += 1;
            let actual = adapter::observation(&result, target);
            if actual["tokens"] != case["oracle"]["tokens"] {
                let actual = actual["tokens"].as_array().unwrap();
                let expected = case["oracle"]["tokens"].as_array().unwrap();
                let mismatch = actual
                    .iter()
                    .zip(expected)
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                failures.push(format!(
                    "{}: token {mismatch}: actual {:?}, expected {:?}",
                    case["id"],
                    actual.get(mismatch),
                    expected.get(mismatch)
                ));
            } else if std::env::var_os("PPRS_REPORT_TOKENS").is_some() {
                println!("P05_TOKEN_PASS {}", case["id"].as_str().unwrap());
            }
            let expected: Vec<_> = case["oracle"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|diagnostic| diagnostic["category"] == "lexical")
                .map(|diagnostic| json!({"code":diagnostic["code"],"span":diagnostic["span"]}))
                .collect();
            assert_eq!(adapter::diagnostics(&result), expected, "{}", case["id"]);
        }
    }
    assert!(
        complete > 100,
        "must exercise a substantial production corpus"
    );
    assert_eq!(failures, Vec::<String>::new(), "{}", failures.join("\n"));
}

fn assert_lossless(result: &pprs::LexResult, options: ParseOptions) {
    use pprs::token::TokenKind;
    let stream = result.tokens();
    let mut position = 0;
    for (index, token) in stream.tokens().iter().enumerate() {
        assert_eq!(token.span().start(), position);
        position = token.span().end();
        assert!(position <= result.source().len());
        assert_eq!(
            stream.text(index).unwrap(),
            &result.source().as_bytes()[token.span().range()]
        );
        if token.kind() == TokenKind::Eof {
            assert_eq!(index, stream.tokens().len() - 1);
            assert_eq!(token.span(), result.source().eof());
        } else {
            assert!(!token.span().is_empty());
        }
    }
    assert_eq!(position, result.source().len());
    assert_eq!(result.is_complete(), stream.is_complete());
    assert!(stream.tokens().len() <= options.limits.get(pprs::options::Resource::Tokens));
    assert!(result.diagnostics().len() <= options.limits.get(pprs::options::Resource::Diagnostics));
    for diagnostic in result.diagnostics() {
        assert!(result.source().slice(diagnostic.primary).is_some());
    }
}

#[test]
fn every_byte_and_fixture_truncation_preserves_source_and_terminates() {
    for value in 0..=u8::MAX {
        let mut bytes = b"<?php ".to_vec();
        bytes.extend_from_slice(&[value; 16]);
        bytes.extend_from_slice(b"; $after = 1;");
        let options = ParseOptions::default();
        let result = lex(&bytes, options).unwrap();
        assert_lossless(&result, options);
    }
    let inputs: Value = serde_json::from_str(include_str!("../tools/lexer/inputs.json")).unwrap();
    for case in inputs.as_array().unwrap() {
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let options = ParseOptions {
            version: case["target"].as_str().unwrap().parse().unwrap(),
            short_open_tag: case["short_open_tag"].as_bool().unwrap(),
            ..ParseOptions::default()
        };
        for end in 0..=source.len() {
            assert_lossless(&lex(&source[..end], options).unwrap(), options);
        }
    }
}

#[test]
fn resource_limits_and_capability_stops_preserve_unscanned_bytes() {
    use pprs::{
        Limits,
        lexer::{DeferredSyntax, LexStop},
        options::Resource,
        token::TokenKind,
    };
    let options = ParseOptions {
        limits: Limits::new(5, 10, 10, 10, 10).unwrap(),
        ..ParseOptions::default()
    };
    assert_eq!(lex("123456", options).unwrap_err().length, 6);
    assert_eq!(lex("12345", options).unwrap().source().len(), 5);
    for input in ["<?php $a=1;", "<?php \0\x01\x02", "<?php /*"] {
        for tokens in 2..8 {
            for diagnostics in 1..4 {
                let options = ParseOptions {
                    limits: Limits::new(100, 10, tokens, 10, diagnostics).unwrap(),
                    ..ParseOptions::default()
                };
                let result = lex(input, options).unwrap();
                assert_lossless(&result, options);
                if let Some(LexStop::Limit(resource)) = result.stop() {
                    assert!(matches!(resource, Resource::Tokens | Resource::Diagnostics));
                    assert_eq!(
                        result.diagnostics().last().unwrap().code,
                        pprs::diagnostic::DiagnosticCode::ResourceLimit(resource)
                    );
                }
            }
        }
    }
    for (input, expected) in [
        ("<?php 'x'", DeferredSyntax::String),
        ("<?php B\"x\"", DeferredSyntax::String),
        ("<?php `x`", DeferredSyntax::String),
        ("<?php <<<X\nx\nX", DeferredSyntax::Heredoc),
        (
            "<?php __halt_compiler();\0raw",
            DeferredSyntax::HaltCompiler,
        ),
    ] {
        let result = lex(input, ParseOptions::default()).unwrap();
        assert_eq!(result.stop(), Some(LexStop::Deferred(expected)));
        assert_eq!(result.diagnostics().len(), 0);
        assert_eq!(result.tokens().tokens()[1].kind(), TokenKind::Unscanned);
        assert_eq!(result.tokens().text(1), Some(&input.as_bytes()[6..]));
        assert_lossless(&result, ParseOptions::default());
    }
}

#[test]
fn invalid_bytes_recover_and_shared_source_outlives_the_caller() {
    use pprs::token::TokenKind;
    let source = pprs::SourceText::from("<?php \0; $after = 1; /*");
    let pointer = source.as_bytes().as_ptr();
    let result = pprs::lex_source(source, ParseOptions::default()).unwrap();
    assert_eq!(result.source().as_bytes().as_ptr(), pointer);
    assert!(result.is_complete());
    assert_eq!(result.diagnostics().len(), 2);
    assert!(
        result
            .tokens()
            .tokens()
            .iter()
            .any(|token| token.kind() == TokenKind::Variable)
    );
}

#[test]
fn lexical_oracle_provenance_is_pinned_and_reproducible_offline() {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    let inputs: Value = serde_json::from_str(include_str!("../tools/lexer/inputs.json")).unwrap();
    for data in [
        include_str!("fixtures/lexer/regular.json"),
        include_str!("fixtures/lexer/short-tags.json"),
    ] {
        let document: Value = serde_json::from_str(data).unwrap();
        assert_eq!(document["reference_commit"], lock["commit"]);
        assert_eq!(document["php_version"], "8.5.6");
        assert_eq!(document["php_int_size"], 8);
        for (path, digest) in document["sources_sha256"].as_object().unwrap() {
            assert_eq!(
                digest,
                &hex::encode(Sha256::digest(std::fs::read(root.join(path)).unwrap()))
            );
        }
        for case in document["cases"].as_array().unwrap() {
            assert!(ids.insert(case["id"].as_str().unwrap().to_owned()));
            let input = inputs
                .as_array()
                .unwrap()
                .iter()
                .find(|input| input["id"] == case["id"])
                .unwrap();
            for key in ["source", "target", "short_open_tag"] {
                assert_eq!(case[key], input[key]);
            }
        }
    }
    assert_eq!(ids.len(), inputs.as_array().unwrap().len());
}

use proptest::prelude::*;
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn arbitrary_bytes_obey_source_and_budget_invariants(
        bytes in prop::collection::vec(any::<u8>(), 0..4096),
        php in any::<bool>(), short in any::<bool>(), version in 0_usize..10,
        token_limit in 2_usize..1024, diagnostic_limit in 1_usize..64,
    ) {
        let mut source = if php { b"<?php ".to_vec() } else { Vec::new() };
        source.extend_from_slice(&bytes);
        let options = ParseOptions { version:pprs::PhpVersion::ALL[version], short_open_tag:short,
            limits:pprs::Limits::new(8192,128,token_limit,1000,diagnostic_limit).unwrap() };
        assert_lossless(&lex(source,options).unwrap(),options);
    }
}
