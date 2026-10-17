//! Diagnostic and recovery contract.
#[path = "support/parser.rs"]
mod adapter;
#[path = "support/diagnostics.rs"]
mod diagnostics;
use serde_json::{Value, json};

#[test]
#[ignore = "development audit: set PPRS_RECOVERY_REPORT to an output path"]
fn export_recovery_audit() {
    let mut rows = Vec::new();
    let mut cases: Vec<Value> = include_str!("fixtures/upstream/cases.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let probes: Value =
        serde_json::from_str(include_str!("fixtures/recovery/recovery.json")).unwrap();
    cases.extend(probes["cases"].as_array().unwrap().iter().map(|case| json!({
        "id":format!("P11/{}",case["id"].as_str().unwrap()), "scope":"core", "source":case["source"], "oracle":{"target":case["target"]}
    })));
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
        let mut actual = adapter::full_observation(&parsed, target);
        actual["diagnostics"] = json!(diagnostics::diagnostics(&parsed));
        rows.push(json!({"id":case["id"],"actual":actual,"native":diagnostics::native(&parsed)}));
    }
    std::fs::write(
        std::env::var("PPRS_RECOVERY_REPORT").expect("explicit report destination"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}

#[path = "support/invariants.rs"]
mod checks;
#[allow(dead_code)]
mod support;
use pprs::{
    ParseOptions, ParseStatus, diagnostic::DiagnosticCode as D, options::Resource,
    token::TokenKind as K,
};
use sha2::{Digest, Sha256};

fn parse_case(case: &Value) -> (pprs::ParseResult, ParseOptions) {
    let target = case["target"]
        .as_str()
        .or_else(|| case["oracle"]["target"].as_str())
        .unwrap();
    let options = ParseOptions {
        version: target.parse().unwrap(),
        ..Default::default()
    };
    let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
    (pprs::parse(source, options).unwrap(), options)
}

#[test]
fn every_invalid_core_case_has_a_review_and_deterministic_strict_failure() {
    let reviews: Value =
        serde_json::from_str(include_str!("fixtures/recovery/core-review.json")).unwrap();
    let coverage: Value = serde_json::from_str(include_str!("fixtures/coverage.json")).unwrap();
    let mut used = std::collections::BTreeSet::new();
    for line in include_str!("fixtures/upstream/cases.jsonl").lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        if case["scope"] != "core" {
            continue;
        }
        let id = case["id"].as_str().unwrap();
        let (result, options) = parse_case(&case);
        checks::invariants(&result, options);
        for channel in support::CHANNELS {
            assert_ne!(
                coverage["cases"][id][channel]["state"], "pending",
                "{id}/{channel}"
            );
        }
        if case["oracle"]["status"] == "clean" {
            assert_eq!(result.status(), ParseStatus::Clean, "{id}");
            assert!(result.into_strict().is_ok());
            continue;
        }
        assert_ne!(
            result.status(),
            ParseStatus::Clean,
            "false clean success: {id}"
        );
        let review = reviews
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .expect("invalid fixture review");
        assert!(used.insert(id.to_owned()));
        assert_ne!(review["reason"].as_str().unwrap(), "");
        assert_eq!(
            review["native"],
            diagnostics::native(&result),
            "native diagnostics: {id}"
        );
        assert_eq!(review["reference_status"], case["oracle"]["status"]);
        let target = case["oracle"]["target"].as_str().unwrap();
        let again = pprs::parse(result.source().as_bytes(), options).unwrap();
        assert_eq!(result.diagnostics(), again.diagnostics(), "order: {id}");
        assert_eq!(
            adapter::full_observation(&result, target),
            adapter::full_observation(&again, target),
            "determinism: {id}"
        );
        let pprs::StrictError::Parse(strict) =
            pprs::parse_strict(result.source().as_bytes(), options).unwrap_err()
        else {
            panic!("unexpected preflight failure")
        };
        assert_eq!(
            diagnostics::native(strict.result()),
            diagnostics::native(&result),
            "strict diagnostics: {id}"
        );
        assert_eq!(
            adapter::full_observation(strict.result(), target),
            adapter::full_observation(&result, target),
            "strict AST: {id}"
        );
    }
    assert_eq!(used.len(), 112);
    assert_eq!(reviews.as_array().unwrap().len(), used.len());
}

#[test]
fn focused_diagnostics_match_the_oracle_or_an_individual_review() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/recovery/recovery.json")).unwrap();
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/recovery/inputs.json")).unwrap();
    let reviews: Value =
        serde_json::from_str(include_str!("fixtures/recovery/probe-review.json")).unwrap();
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    assert_eq!(data["reference_commit"], lock["commit"]);
    assert_eq!(data["schema_version"], 1);
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
    assert_eq!(cases.len(), 87);
    assert_eq!(cases.len(), inputs.as_array().unwrap().len());
    assert_eq!(cases.len(), reviews.as_array().unwrap().len());
    let mut failures = Vec::new();
    for ((case, input), review) in cases
        .iter()
        .zip(inputs.as_array().unwrap())
        .zip(reviews.as_array().unwrap())
    {
        for key in ["id", "source", "target", "reason"] {
            assert_eq!(case[key], input[key]);
        }
        assert_eq!(review["id"], case["id"]);
        let (result, options) = parse_case(case);
        checks::invariants(&result, options);
        let target = case["target"].as_str().unwrap();
        let actual = json!({"target":target,"status":adapter::full_observation(&result,target)["status"],"diagnostics":diagnostics::diagnostics(&result)});
        assert_eq!(
            review["native"],
            diagnostics::native(&result),
            "{}",
            case["id"]
        );
        if case.get("oracle_failure").is_some() {
            failures.push(case["id"].as_str().unwrap());
            assert_eq!(case["oracle_failure"]["class"], "AssertionError");
            assert_ne!(result.status(), ParseStatus::Clean);
            assert_eq!(review["state"], "reference-failure");
            support::compare(&review["expected"], &actual, "diagnostics").unwrap();
        } else if review["state"] == "passing" {
            support::compare(&case["expected"], &actual, "diagnostics")
                .unwrap_or_else(|e| panic!("{}: {e}", case["id"]));
        } else {
            assert_eq!(review["state"], "intentional-difference");
            assert_ne!(review["reason"].as_str().unwrap(), "");
            assert!(
                support::compare(&case["expected"], &actual, "diagnostics").is_err(),
                "obsolete difference {}",
                case["id"]
            );
            support::compare(&review["expected"], &actual, "diagnostics")
                .unwrap_or_else(|e| panic!("{}: {e}", case["id"]));
        }
        if case["expected"]["status"] == "clean" {
            assert_eq!(
                result.status(),
                ParseStatus::Clean,
                "control {}",
                case["id"]
            );
        } else {
            assert!(
                pprs::parse_strict(result.source().as_bytes(), options).is_err(),
                "false clean {}",
                case["id"]
            );
        }
    }
    assert_eq!(failures, ["halt-missing-close", "halt-missing-semicolon"]);
}

#[test]
fn malformed_prefixes_and_tiny_budgets_preserve_all_invariants() {
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/recovery/inputs.json")).unwrap();
    for case in inputs.as_array().unwrap() {
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let options = ParseOptions {
            version: case["target"].as_str().unwrap().parse().unwrap(),
            ..Default::default()
        };
        for end in 0..=source.len() {
            checks::invariants(&pprs::parse(&source[..end], options).unwrap(), options);
        }
        for budget in [1, 2, 3, 5, 10] {
            for limits in [
                pprs::Limits::new(10000, 128, 10000, 10000, budget).unwrap(),
                pprs::Limits::new(10000, budget, 10000, 10000, 20).unwrap(),
                pprs::Limits::new(10000, 128, budget.max(2), 10000, 20).unwrap(),
                pprs::Limits::new(10000, 128, 10000, budget, 20).unwrap(),
            ] {
                let options = ParseOptions { limits, ..options };
                checks::invariants(&pprs::parse(&source, options).unwrap(), options);
            }
        }
    }
}

#[test]
fn independent_context_errors_survive_syntax_cascade_suppression() {
    for (source, messages) in [
        (
            "<?php class C { static readonly function __construct() {} }",
            vec![
                "Constructor __construct() cannot be static",
                "Method __construct() cannot be readonly",
            ],
        ),
        (
            "<?php class C { static abstract readonly const X=1; }",
            vec![
                "Cannot use 'static' as constant modifier",
                "Cannot use 'abstract' as constant modifier",
                "Cannot use 'readonly' as constant modifier",
            ],
        ),
    ] {
        let result = pprs::parse(source, ParseOptions::default()).unwrap();
        assert_eq!(
            result
                .diagnostics()
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            messages
        );
        assert!(
            result
                .diagnostics()
                .iter()
                .all(|d| d.code == D::InvalidContext)
        );
    }
    let result = pprs::parse("<?php $", ParseOptions::default()).unwrap();
    assert_eq!(result.diagnostics().len(), 1);
    assert_eq!(result.diagnostics()[0].found, Some(K::Eof));
    let result = pprs::parse("<?php final abstract class C {}", ParseOptions::default()).unwrap();
    let d = &result.diagnostics()[0];
    assert_eq!(result.source().slice(d.primary), Some(&b"abstract"[..]));
    assert_eq!(
        result.source().slice(d.secondary[0].span),
        Some(&b"final"[..])
    );
    let result = pprs::parse("<?php foo(1;", ParseOptions::default()).unwrap();
    let d = &result.diagnostics()[0];
    assert_eq!(d.expected, vec![K::RightParen]);
    assert_eq!(d.found, Some(K::Semicolon));
    assert_eq!(result.source().slice(d.secondary[0].span), Some(&b"("[..]));
    for (source, closing, opening) in [
        ("<?php #[A", K::RightBracket, &b"#["[..]),
        ("<?php \"{$x", K::RightBrace, &b"{"[..]),
    ] {
        let result = pprs::parse(source, ParseOptions::default()).unwrap();
        let diagnostic = result
            .diagnostics()
            .iter()
            .find(|d| d.expected == [closing])
            .unwrap();
        assert_eq!(
            result.source().slice(diagnostic.secondary[0].span),
            Some(opening)
        );
    }
}

#[test]
fn recovery_preserves_following_statements_and_declarations() {
    use pprs::{ast::StmtKind, visit::Node};
    for damaged in [
        "__halt_compiler",
        "__halt_compiler(",
        "__halt_compiler()",
        "$x=[1;",
        "f(1;",
        "$x=+",
    ] {
        let source = format!("<?php {damaged} echo 42;");
        let result = pprs::parse(&source, ParseOptions::default()).unwrap();
        assert_ne!(result.status(), ParseStatus::Clean, "{source}");
        assert!(
            matches!(
                result.program().unwrap().statements.last().unwrap().kind,
                StmtKind::Echo(_)
            ),
            "{source}"
        );
        assert!(
            !result
                .tokens()
                .tokens()
                .iter()
                .any(|t| t.kind() == K::HaltTail)
        );
    }
    for declaration in ["class C {}", "function f() {}", "namespace N { echo 42; }"] {
        let source = format!("<?php $x=+ {declaration}");
        let result = pprs::parse(&source, ParseOptions::default()).unwrap();
        assert!(pprs::visit::walk(result.program().unwrap()).any(|entry|matches!(entry.node,Node::Stmt(s) if matches!(s.kind,StmtKind::Class(_)|StmtKind::Function(_)|StmtKind::Namespace(_)))),"{source}");
    }
    for damaged in ["public ;", "const X=;", "public $x { get => 1;"] {
        let source = format!("<?php class C {{ {damaged} public function after() {{}} }} echo 42;");
        let result = pprs::parse(&source, ParseOptions::default()).unwrap();
        assert!(
            pprs::visit::walk(result.program().unwrap()).any(
                |entry| matches!(entry.node,Node::Stmt(s) if matches!(s.kind,StmtKind::Method(_)))
            ),
            "{source}"
        );
    }
}

#[test]
fn error_storm_stops_at_one_terminal_limit_without_scanning_the_suffix() {
    for limit in [1, 2, 4, 8] {
        let options = ParseOptions {
            limits: pprs::Limits::new(10000, 128, 10000, 10000, limit).unwrap(),
            ..Default::default()
        };
        let source = format!("<?php {} echo 42;", "$a=; ".repeat(100));
        let result = pprs::parse(&source, options).unwrap();
        checks::invariants(&result, options);
        assert_eq!(result.status(), ParseStatus::Aborted);
        assert_eq!(result.diagnostics().len(), limit);
        assert_eq!(
            result.diagnostics().last().unwrap().code,
            D::ResourceLimit(Resource::Diagnostics)
        );
        assert!(
            result
                .tokens()
                .tokens()
                .iter()
                .any(|t| t.kind() == K::Unscanned)
        );
        assert!(result.into_strict().is_err());
    }
    let options = ParseOptions {
        limits: pprs::Limits::new(0, 1, 2, 1, 1).unwrap(),
        ..Default::default()
    };
    assert!(pprs::parse("x", options).is_err());
    assert!(matches!(
        pprs::parse_strict("x", options),
        Err(pprs::StrictError::Input(_))
    ));
}

#[test]
fn diagnostic_comparison_rejects_code_wording_range_order_and_omission() {
    let result = pprs::parse("<?php $a=; $b=;", ParseOptions::default()).unwrap();
    let expected = json!({"target":"8.4","status":"recovered","diagnostics":diagnostics::diagnostics(&result)});
    for (path, value) in [
        ("/diagnostics/0/code", json!("wrong")),
        ("/diagnostics/0/category", json!("contextual")),
        ("/diagnostics/0/message/bytes", json!("")),
        ("/diagnostics/0/span/0", json!(0)),
        ("/diagnostics/0/start/1", json!(0)),
        ("/diagnostics", json!([])),
    ] {
        let mut actual = expected.clone();
        *actual.pointer_mut(path).unwrap() = value;
        assert!(
            support::compare(&expected, &actual, "diagnostics").is_err(),
            "{path}"
        );
    }
    let mut actual = expected.clone();
    actual["diagnostics"].as_array_mut().unwrap().reverse();
    assert!(support::compare(&expected, &actual, "diagnostics").is_err());
    actual.as_object_mut().unwrap().remove("diagnostics");
    assert!(support::compare(&expected, &actual, "diagnostics").is_err());
}
