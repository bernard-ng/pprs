//! Offline reference corpus integrity and comparison adapter tests.
#[path = "support/diagnostics.rs"]
#[allow(dead_code)]
mod diagnostic_adapter;
#[path = "support/lexer.rs"]
mod lexer_adapter;
#[path = "support/parser.rs"]
mod parser_adapter;
mod support;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
use support::{CHANNELS, bytes, compare, gate, validate_observation};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn json_file(path: &str) -> Value {
    serde_json::from_slice(&fs::read(fixture_root().join(path)).unwrap()).unwrap()
}

fn records() -> Vec<Value> {
    fs::read_to_string(fixture_root().join("upstream/cases.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn corpus_is_complete_pinned_and_byte_preserving() {
    let root = fixture_root().join("upstream");
    let manifest = json_file("upstream/manifest.json");
    assert_eq!(manifest["schema_version"], 1);
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    assert_eq!(manifest["reference_commit"], lock["commit"]);
    for (path, expected) in manifest["files"].as_object().unwrap() {
        let digest = hex::encode(Sha256::digest(fs::read(root.join(path)).unwrap()));
        assert_eq!(expected, &digest, "file hash: {path}");
    }
    let cases = records();
    assert_eq!(cases.len(), 313);
    let inventory: Vec<Value> = include_str!("../docs/compatibility/fixtures.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let expected: BTreeSet<_> = inventory
        .iter()
        .filter_map(|entry| {
            let id = entry["id"].as_str().unwrap();
            id.starts_with("parser/").then_some(id)
        })
        .collect();
    let originals: BTreeSet<_> = cases
        .iter()
        .filter(|case| !case["origin"].is_null())
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(originals, expected);
    let ids: BTreeSet<_> = cases
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), cases.len());
    for case in &cases {
        let source = bytes(&case["source"]);
        assert_eq!(
            case["source_sha256"],
            hex::encode(Sha256::digest(&source)),
            "{}",
            case["id"]
        );
        assert_eq!(case["profile_id"], manifest["profile_id"]);
        validate_observation(&case["oracle"], &source);
    }
}

#[test]
fn recorded_coverage_requires_a_real_implementation_for_promoted_cases() {
    let states = json_file("coverage.json");
    let cases = records();
    let reviews = json_file("fidelity/recovery-review.json");
    let mut reviewed = BTreeSet::new();
    assert_eq!(states["cases"].as_object().unwrap().len(), cases.len());
    for case in &cases {
        let id = case["id"].as_str().unwrap();
        let channels = states["cases"].get(id).expect("explicit case state");
        assert_eq!(channels.as_object().unwrap().len(), CHANNELS.len());
        let source = bytes(&case["source"]);
        let target = case["oracle"]["target"].as_str().unwrap();
        let actual = if case["scope"] == "core" {
            let lexical = pprs::lex(
                &source,
                pprs::ParseOptions {
                    version: target.parse().unwrap(),
                    ..pprs::ParseOptions::default()
                },
            )
            .unwrap();
            lexical
                .is_complete()
                .then(|| lexer_adapter::observation(&lexical, target))
        } else {
            // Deferred originals may target PHP 8.5/8.6; do not silently coerce
            // them into this library's PHP 7.0–8.4 compatibility range.
            None
        };
        let parsed = (case["scope"] == "core").then(|| {
            let result = pprs::parse(
                &source,
                pprs::ParseOptions {
                    version: target.parse().unwrap(),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut observation = parser_adapter::full_observation(&result, target);
            observation["diagnostics"] = json!(diagnostic_adapter::diagnostics(&result));
            observation
        });
        for channel in CHANNELS {
            let state = channels.get(channel).expect("explicit channel state");
            if case["scope"] == "core" {
                assert_ne!(state["state"], "deferred");
            } else {
                assert_eq!(state["state"], "deferred");
            }
            if case["scope"] == "core"
                && matches!(channel, "structure" | "positions" | "diagnostics")
                && case["oracle"]["status"] == "clean"
            {
                assert_eq!(
                    state["state"], "passing",
                    "valid fixture must have exact fidelity: {id}/{channel}"
                );
            }
            if channel == "positions" && state["state"] == "intentional-difference" {
                let review = reviews
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|review| review["id"] == id)
                    .expect("individual source recovery review");
                assert!(reviewed.insert(id));
                assert_eq!(review["reason"], state["reason"]);
                assert_eq!(review["expectation"], state["expectation"]);
            }
            let difference = state.get("expectation").map(|path| {
                let path = path.as_str().unwrap();
                assert!(path.starts_with("differences/") && !path.contains(".."));
                json_file(path)
            });
            gate(
                &case["oracle"],
                if channel == "tokens" {
                    actual.as_ref()
                } else if matches!(channel, "structure" | "positions" | "diagnostics") {
                    parsed.as_ref()
                } else {
                    None
                },
                channel,
                state,
                difference.as_ref(),
            )
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        }
    }
    assert_eq!(reviewed.len(), reviews.as_array().unwrap().len());
}

#[test]
fn comparison_rejects_independent_mutations_and_missing_fields() {
    let cases = records();
    let sample = &cases
        .iter()
        .find(|case| case["id"] == "project/adapter/numbers")
        .unwrap()["oracle"];
    for channel in CHANNELS {
        compare(sample, sample, channel).unwrap();
    }
    for (pointer, replacement, channel) in [
        ("/target", json!("7.0"), "tokens"),
        ("/ast/0/kind", json!("WrongNode"), "structure"),
        (
            "/ast/0/fields/expr/fields/value/integer",
            json!("9007199254740992"),
            "structure",
        ),
        ("/tokens/0/span/1", json!(0), "tokens"),
        ("/tokens/0/text/bytes", json!(""), "tokens"),
        ("/metadata/0/origin/span/1", json!(0), "positions"),
        ("/metadata/0/comments", json!([{"kind":"doc"}]), "positions"),
        ("/status", json!("recovered"), "diagnostics"),
    ] {
        let mut actual = sample.clone();
        *actual
            .pointer_mut(pointer)
            .expect("mutation must hit an existing field") = replacement;
        assert!(
            compare(sample, &actual, channel).is_err(),
            "missed {pointer}"
        );
    }
    let mut actual = sample.clone();
    actual.as_object_mut().unwrap().remove("ast");
    assert!(compare(sample, &actual, "structure").is_err());
    let error = &cases
        .iter()
        .find(|case| case["id"] == "project/adapter/eof-error")
        .unwrap()["oracle"];
    let mut actual = error.clone();
    actual["diagnostics"] = json!([]);
    assert!(compare(error, &actual, "diagnostics").is_err());
}

#[test]
fn passing_and_intentional_difference_states_cannot_hide_regressions() {
    let expected = json!({"target":"8.4", "status":"clean", "ast":[]});
    let different = json!({"target":"8.4", "status":"recovered", "ast":null});
    let passing = json!({"state":"passing"});
    assert!(gate(&expected, None, "structure", &passing, None).is_err());
    assert!(gate(&expected, Some(&different), "structure", &passing, None).is_err());
    gate(&expected, Some(&expected), "structure", &passing, None).unwrap();
    let intentional =
        json!({"state":"intentional-difference", "reason":"Explicit test recovery deviation"});
    let wrong_target = json!({"target":"7.4", "status":"recovered", "ast":null});
    assert!(
        gate(
            &expected,
            Some(&wrong_target),
            "structure",
            &intentional,
            Some(&wrong_target)
        )
        .is_err()
    );
    assert!(gate(&expected, Some(&different), "structure", &intentional, None).is_err());
    assert!(
        gate(
            &expected,
            Some(&different),
            "structure",
            &intentional,
            Some(&expected)
        )
        .is_err()
    );
    gate(
        &expected,
        Some(&different),
        "structure",
        &intentional,
        Some(&different),
    )
    .unwrap();
    assert!(
        gate(
            &expected,
            Some(&expected),
            "structure",
            &intentional,
            Some(&different)
        )
        .is_err()
    );
}

#[test]
fn independent_adapter_edge_expectations() {
    let cases = records();
    let find = |id: &str| &cases.iter().find(|case| case["id"] == id).unwrap()["oracle"];
    let numbers = find("project/adapter/numbers");
    assert_eq!(
        numbers
            .pointer("/ast/0/fields/expr/fields/value/integer")
            .unwrap(),
        "9223372036854775807"
    );
    assert_eq!(
        numbers
            .pointer("/ast/1/fields/expr/fields/value/float64")
            .unwrap(),
        "7ff0000000000000"
    );
    let comment = find("project/adapter/unterminated-comment");
    assert_eq!(
        comment["diagnostics"][0]["code"],
        "lex.unterminated-comment"
    );
    assert_eq!(comment["diagnostics"][0]["span"], json!([6, 13]));
    let bad = find("project/adapter/bad-bytes");
    assert_eq!(bad["diagnostics"][0]["span"], json!([6, 7]));
    assert_eq!(bad["diagnostics"][1]["span"], json!([7, 8]));
    let eof = find("project/adapter/eof-error");
    assert_eq!(eof["diagnostics"][0]["span"], json!([17, 17]));
    let empty = find("project/adapter/empty");
    assert_eq!(empty["ast"], json!([]));
    assert_eq!(empty["tokens"][0]["span"], json!([0, 0]));
    assert_eq!(support::coordinate(b"a\r\nb\rc\nd", 2), [1, 3]);
    assert_eq!(support::coordinate(b"a\r\nb\rc\nd", 3), [2, 1]);
    assert_eq!(support::coordinate(b"a\r\nb\rc\nd", 5), [3, 1]);
    assert_eq!(support::coordinate(b"a\r\nb\rc\nd", 7), [4, 1]);
    assert_ne!(
        find("project/version/7.4")["ast"],
        find("project/version/8.0")["ast"]
    );
}
