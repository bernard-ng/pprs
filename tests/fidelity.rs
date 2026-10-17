//! Source fidelity and version boundary probes.
#[path = "support/parser.rs"]
mod adapter;
use serde_json::Value;
#[test]
#[ignore = "development audit: set PPRS_FIDELITY_REPORT to an output path"]
fn export_fidelity_audit() {
    let mut rows = Vec::new();
    let mut cases = include_str!("fixtures/upstream/cases.jsonl")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let dedicated: Value =
        serde_json::from_str(include_str!("fixtures/fidelity/fidelity.json")).unwrap();
    cases.extend(dedicated["cases"].as_array().unwrap().iter().map(|c|serde_json::json!({"id":format!("P10/{}",c["id"].as_str().unwrap()),"source":c["source"],"oracle":c["expected"],"scope":"core"})));
    for case in cases {
        if case["scope"] != "core" {
            continue;
        }
        let target = case["oracle"]["target"].as_str().unwrap();
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let parsed = pprs::parse(
            source,
            pprs::ParseOptions {
                version: target.parse().unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        rows.push(
            serde_json::json!({"id":case["id"],"actual":adapter::full_observation(&parsed,target)}),
        );
    }
    std::fs::write(
        std::env::var("PPRS_FIDELITY_REPORT").expect("explicit report destination"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}

#[path = "support/invariants.rs"]
mod checks;
#[allow(dead_code)]
mod support;
use sha2::{Digest, Sha256};

#[test]
fn version_and_source_oracle_matches_reviewed_policy() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/fidelity/fidelity.json")).unwrap();
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/fidelity/inputs.json")).unwrap();
    let differences: Value =
        serde_json::from_str(include_str!("fixtures/fidelity/differences.json")).unwrap();
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    assert_eq!(data["schema_version"], 1);
    assert_eq!(data["reference_commit"], lock["commit"]);
    assert_eq!(data["php_version"], "8.5.6");
    assert_eq!(data["php_int_size"], 8);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, hash) in data["sources_sha256"].as_object().unwrap() {
        assert_eq!(
            hash,
            &hex::encode(Sha256::digest(std::fs::read(root.join(path)).unwrap())),
            "{path}"
        );
    }
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 222);
    assert_eq!(cases.len(), inputs.as_array().unwrap().len());
    let mut used = std::collections::BTreeSet::new();
    for (case, input) in cases.iter().zip(inputs.as_array().unwrap()) {
        for field in ["id", "source", "target", "reason", "policy"] {
            assert_eq!(case[field], input[field]);
        }
        let target = case["target"].as_str().unwrap();
        let options = pprs::ParseOptions {
            version: target.parse().unwrap(),
            ..Default::default()
        };
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let result = pprs::parse(&source, options).unwrap();
        checks::invariants(&result, options);
        if case["policy"] == "ceiling" {
            assert_ne!(result.status(), pprs::ParseStatus::Clean, "{}", case["id"]);
            assert!(pprs::parse_strict(&source, options).is_err());
        }
        let actual = adapter::full_observation(&result, target);
        let expected = if let Some(d) = differences
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == case["id"])
        {
            assert!(
                case["policy"] == "ceiling" || case["expected"]["status"] != "clean",
                "valid in-scope probe must match exactly"
            );
            assert_ne!(d["reason"].as_str().unwrap(), "");
            assert!(used.insert(d["id"].as_str().unwrap()));
            assert!(
                ["structure", "positions"]
                    .iter()
                    .any(
                        |channel| support::compare(&case["expected"], &d["expected"], channel)
                            .is_err()
                    ),
                "obsolete difference {}",
                case["id"]
            );
            &d["expected"]
        } else {
            &case["expected"]
        };
        for channel in ["structure", "positions"] {
            support::compare(expected, &actual, channel)
                .unwrap_or_else(|e| panic!("{}: {e}", case["id"]));
        }
    }
    assert_eq!(used.len(), differences.as_array().unwrap().len());
}

#[test]
fn source_metadata_remains_checked_on_truncations_and_resource_aborts() {
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/fidelity/inputs.json")).unwrap();
    for case in inputs.as_array().unwrap() {
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let options = pprs::ParseOptions {
            version: case["target"].as_str().unwrap().parse().unwrap(),
            ..Default::default()
        };
        for end in 0..=source.len() {
            checks::invariants(&pprs::parse(&source[..end], options).unwrap(), options);
        }
        for budget in [1, 2, 4, 8, 16] {
            for limits in [
                pprs::Limits::new(10000, budget, 10000, 10000, 20).unwrap(),
                pprs::Limits::new(10000, 128, 10000, budget, 20).unwrap(),
                pprs::Limits::new(10000, 128, budget.max(2), 10000, 20).unwrap(),
            ] {
                let options = pprs::ParseOptions { limits, ..options };
                checks::invariants(&pprs::parse(&source, options).unwrap(), options);
            }
        }
    }
}

#[test]
fn grouping_comments_and_raw_spelling_are_publicly_recoverable() {
    use pprs::ast::{Origin, StmtKind};
    let result = pprs::parse(
        "<?php /** statement */ ((0XfF)); /* tail */ \r\n",
        pprs::ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(result.status(), pprs::ParseStatus::Clean);
    let stmts = &result.program().unwrap().statements;
    assert_eq!(stmts.len(), 2);
    let StmtKind::Expression(expr) = &stmts[0].kind else {
        panic!()
    };
    let Origin::Parsed { span, .. } = expr.meta.origin() else {
        panic!()
    };
    assert_eq!(result.source().slice(span), Some(&b"0XfF"[..]));
    let Origin::Parsed { span, .. } = expr.meta.enclosing_origin() else {
        panic!()
    };
    assert_eq!(result.source().slice(span), Some(&b"((0XfF))"[..]));
    assert_eq!(stmts[0].meta.comments().len(), 1);
    assert_eq!(expr.meta.comments().len(), 0);
    let Origin::Parsed { span, tokens } = stmts[1].meta.origin() else {
        panic!()
    };
    assert!(span.is_empty() && tokens.is_empty());
    let comment = stmts[1].meta.comments()[0];
    assert_eq!(span.start(), result.tokens().tokens()[comment].span().end());
    assert_eq!(tokens.start(), comment + 1);
    checks::invariants(&result, pprs::ParseOptions::default());
}

#[test]
fn metadata_comparison_rejects_spelling_ranges_ownership_and_missing_nodes() {
    let result = pprs::parse("<?php /** doc */ 0xff;", pprs::ParseOptions::default()).unwrap();
    let expected = adapter::full_observation(&result, "8.4");
    for (path, replacement) in [
        (
            "/metadata/1/spelling/rawValue/bytes",
            serde_json::json!("30586666"),
        ),
        ("/metadata/1/spelling/kind/integer", serde_json::json!("10")),
        ("/metadata/0/comments", serde_json::json!([])),
        ("/metadata/1/origin/token_span/1", serde_json::json!(0)),
        ("/metadata/1/origin/start/1", serde_json::json!(0)),
        ("/metadata/1/path", serde_json::json!("/wrong")),
        ("/metadata", serde_json::json!([])),
    ] {
        let mut actual = expected.clone();
        *actual.pointer_mut(path).unwrap() = replacement;
        assert!(
            support::compare(&expected, &actual, "positions").is_err(),
            "missed {path}"
        );
    }
}
