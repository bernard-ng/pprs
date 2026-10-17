//! Offline P07 expression grammar oracle and public parser contract.
#[path = "support/parser.rs"]
mod adapter;
use pprs::{ParseOptions, ParseStatus, PhpVersion};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn expression_oracle_is_reproducible_and_matches_every_probe() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/parser/expressions.json")).unwrap();
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    assert_eq!(data["reference_commit"], lock["commit"]);
    assert_eq!(data["php_version"], "8.5.6");
    assert_eq!(data["php_int_size"], 8);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, hash) in data["sources_sha256"].as_object().unwrap() {
        assert_eq!(
            hash,
            &hex::encode(Sha256::digest(std::fs::read(root.join(path)).unwrap()))
        );
    }
    let inputs: Value = serde_json::from_str(include_str!("../tools/parser/inputs.json")).unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), inputs.as_array().unwrap().len());
    assert_eq!(cases.len(), 133);
    for (case, input) in cases.iter().zip(inputs.as_array().unwrap()) {
        for field in ["id", "source", "target", "reason"] {
            assert_eq!(case[field], input[field]);
        }
        let bytes = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let target = case["target"].as_str().unwrap();
        let result = pprs::parse(
            bytes,
            ParseOptions {
                version: target.parse().unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            adapter::observation(&result, target),
            case["expected"],
            "{}: {:?}",
            case["id"],
            result.diagnostics()
        );
    }
}

#[path = "support/invariants.rs"]
mod checks;
use checks::invariants;

#[test]
fn public_api_retains_source_and_strict_failures() {
    let source = pprs::SourceText::from("<?php echo $x + 1; return 2;");
    let pointer = source.as_bytes().as_ptr();
    let parsed = pprs::parse_source(source, ParseOptions::default()).unwrap();
    assert_eq!(pointer, parsed.source().as_bytes().as_ptr());
    assert_eq!(parsed.status(), ParseStatus::Clean);
    assert_eq!(parsed.program().unwrap().statements.len(), 2);
    invariants(&parsed, ParseOptions::default());
    parsed.into_strict().unwrap();
    assert!(pprs::parse_strict("", ParseOptions::default()).is_ok());
    let pprs::StrictError::Parse(error) =
        pprs::parse_strict("<?php echo ; echo 2;", ParseOptions::default()).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(error.result().status(), ParseStatus::Recovered);
    assert_eq!(error.result().program().unwrap().statements.len(), 2);
    assert_eq!(
        error.into_result().source().as_bytes(),
        b"<?php echo ; echo 2;"
    );
    let options = ParseOptions {
        limits: pprs::Limits::new(0, 8, 10, 10, 2).unwrap(),
        ..Default::default()
    };
    assert!(matches!(
        pprs::parse_strict("x", options),
        Err(pprs::StrictError::Input(_))
    ));
}

#[test]
fn truncations_and_every_core_fixture_preserve_invariants() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/parser/expressions.json")).unwrap();
    for case in document["cases"].as_array().unwrap().iter().skip(100) {
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        for end in 0..=source.len() {
            let options = ParseOptions::default();
            invariants(&pprs::parse(&source[..end], options).unwrap(), options);
        }
    }
    for line in include_str!("fixtures/upstream/cases.jsonl").lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        if case["scope"] != "core" {
            continue;
        }
        let options = ParseOptions {
            version: case["oracle"]["target"].as_str().unwrap().parse().unwrap(),
            ..Default::default()
        };
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        invariants(&pprs::parse(source, options).unwrap(), options);
    }
}

#[test]
fn budgets_bound_recursive_and_flat_input() {
    use pprs::{Limits, diagnostic::DiagnosticCode as D, options::Resource};
    for (source, resource, limits) in [
        (
            format!("<?php {}1{};", "f(".repeat(1000), ")".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}1{};", "[".repeat(1000), "]".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!(
                "<?php {}1{};",
                "match(1){1=>".repeat(1000),
                "}".repeat(1000)
            ),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}1{};", "\"{$a[".repeat(1000), "]}\"".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}1{};", "(".repeat(1000), ")".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}$x;", "!".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}$x;", "$x+".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}{}", "{".repeat(1000), "}".repeat(1000)),
            Resource::Depth,
            Limits::default(),
        ),
        (
            format!("<?php {}", "1;".repeat(1000)),
            Resource::Nodes,
            Limits::new(10000, 128, 10000, 12, 10).unwrap(),
        ),
        (
            "<?php echo 1,2,3;".into(),
            Resource::Tokens,
            Limits::new(10000, 128, 6, 100, 10).unwrap(),
        ),
        (
            "<?php echo ; echo ;".into(),
            Resource::Diagnostics,
            Limits::new(10000, 128, 100, 100, 2).unwrap(),
        ),
    ] {
        let options = ParseOptions {
            limits,
            ..Default::default()
        };
        let result = pprs::parse(source, options).unwrap();
        invariants(&result, options);
        assert_eq!(result.status(), ParseStatus::Aborted);
        assert_eq!(
            result.diagnostics().last().unwrap().code,
            D::ResourceLimit(resource)
        );
        assert!(result.into_strict().is_err());
    }
}

#[test]
fn recovery_preserves_following_statements_and_halt_requires_valid_outermost_syntax() {
    use pprs::{ast::StmtKind, token::TokenKind};
    for input in [
        "<?php echo ; echo 42;",
        "<?php f(,); echo 42;",
        "<?php \"\\u{110000}\"; echo 42;",
        "<?php $x = ; echo 42;",
    ] {
        let result = pprs::parse(input, ParseOptions::default()).unwrap();
        assert_eq!(result.status(), ParseStatus::Recovered, "{input}");
        assert!(
            matches!(
                result.program().unwrap().statements.last().unwrap().kind,
                StmtKind::Echo(_)
            ),
            "{input}"
        );
        assert!(result.into_strict().is_err());
    }
    for input in [
        "<?php { __halt_compiler(); echo 42; }",
        "<?php __halt_compiler(1); echo 42;",
        "<?php if ($x) { __halt_compiler(); } echo 42;",
    ] {
        let result = pprs::parse(input, ParseOptions::default()).unwrap();
        assert_ne!(result.status(), ParseStatus::Clean);
        assert!(
            !result
                .tokens()
                .tokens()
                .iter()
                .any(|t| t.kind() == TokenKind::HaltTail)
        );
        assert!(result.into_strict().is_err());
    }
    let input = b"<?php __halt_compiler();\xff\0 <?php \"unterminated";
    let result = pprs::parse(input, ParseOptions::default()).unwrap();
    assert_eq!(result.status(), ParseStatus::Clean);
    assert!(
        result
            .tokens()
            .tokens()
            .iter()
            .any(|t| t.kind() == TokenKind::HaltTail)
    );
    invariants(&result, ParseOptions::default());
}

#[test]
fn traversal_is_preorder_and_visits_auxiliary_nodes() {
    use pprs::{
        ast::{ExprKind, StmtKind},
        visit::Node,
    };
    let parsed = pprs::parse_strict("<?php f(x: [2 => $v]);", ParseOptions::default()).unwrap();
    let order: Vec<_> = pprs::visit::walk(parsed.result().program().unwrap())
        .map(|entry| {
            (
                match entry.node {
                    Node::Program(_) => "program",
                    Node::Stmt(s) if matches!(s.kind, StmtKind::Expression(_)) => "stmt",
                    Node::Expr(e) => match e.kind {
                        ExprKind::Call { .. } => "call",
                        ExprKind::Array { .. } => "array",
                        ExprKind::Integer { .. } => "integer",
                        ExprKind::Variable(_) => "variable",
                        _ => panic!(),
                    },
                    Node::Name(_) => "name",
                    Node::Argument(_) => "arg",
                    Node::ArrayItem(_) => "item",
                    Node::Identifier(_) => "identifier",
                    _ => panic!(),
                },
                entry.depth,
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            ("program", 1),
            ("stmt", 2),
            ("call", 3),
            ("name", 4),
            ("arg", 4),
            ("identifier", 5),
            ("array", 5),
            ("item", 6),
            ("integer", 7),
            ("variable", 7),
            ("identifier", 8)
        ]
    );
}

use proptest::prelude::*;
proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    fn arbitrary_bytes_and_small_budgets_never_panic(
        bytes in prop::collection::vec(any::<u8>(),0..1024),
        version in 0_usize..10, depth in 1_usize..32, nodes in 1_usize..256,
        tokens in 2_usize..256, diagnostics in 1_usize..16,
    ) {
        let input=[b"<?php ".as_slice(),bytes.as_slice()].concat();
        let options=ParseOptions{version:PhpVersion::ALL[version],limits:pprs::Limits::new(2048,depth,tokens,nodes,diagnostics).unwrap(),..Default::default()};
        invariants(&pprs::parse(input,options).unwrap(),options);
    }
}

#[test]
fn invalid_grammar_contexts_cannot_be_clean() {
    for source in [
        "($a) = 1;",
        "($a)++;",
        "++($a);",
        "new Foo[0];",
        "new Foo->x;",
        "new Foo::BAR;",
        "new (Foo)[0]();",
        "__FILE__();",
        "__FILE__::X;",
        "$ foo;",
        "$1;",
        "\"{$a+1}\";",
        "$a->b\"text\";",
        "f(x: &$a);",
        "f(x: ...$a);",
        "f(&...$a);",
        "[&$a=>1];",
        "[...$a=>1];",
        "1();",
        "1->x;",
        "1[0];",
        "static;",
        "readonly;",
        "list($x);",
        "f(list($x));",
        "$a =& 1;",
        "[&1];",
        "f(&1);",
        "new 1;",
        "$a instanceof 1;",
        "f(A\\B: 1);",
        "f(1: 1);",
        "1 = 2;",
        "++1;",
    ] {
        let input = format!("<?php {source} echo 42;");
        let result = pprs::parse(&input, ParseOptions::default()).unwrap();
        assert_ne!(result.status(), ParseStatus::Clean, "{source}");
        assert!(result.into_strict().is_err(), "{source}");
    }
}

#[test]
fn abort_during_block_recovery_never_requests_another_token() {
    let options = ParseOptions {
        version: PhpVersion::Php70,
        limits: pprs::Limits::new(100, 4, 6, 2, 5).unwrap(),
        ..Default::default()
    };
    let result = pprs::parse(b"<?php {\0\0\0\0\0", options).unwrap();
    assert_eq!(result.status(), ParseStatus::Aborted);
    invariants(&result, options);
}

#[test]
fn trailing_comment_noop_obeys_the_depth_budget() {
    let options = ParseOptions {
        limits: pprs::Limits::new(100, 1, 4, 2, 1).unwrap(),
        ..Default::default()
    };
    let result = pprs::parse(b"<?php #\0", options).unwrap();
    assert_eq!(result.status(), ParseStatus::Aborted);
    invariants(&result, options);
}

#[test]
fn small_budgets_cover_auxiliary_nodes_and_recovery_paths() {
    for source in [
        "<?php // eof",
        "<?php { /*x*/ }",
        "<?php { echo ; $a = ; } echo 1;",
        "<?php f(x: [1 => $a,]);",
        "<?php match($a) {1,2 => \"x $b\", default => 3};",
        "<?php __halt_compiler();raw",
        "<?php new C()->f(...);",
    ] {
        for depth in [1, 2, 4, 8, 128] {
            for tokens in [2, 3, 6, 12, 64] {
                for nodes in [1, 2, 4, 16, 64] {
                    for diagnostics in [1, 2, 8] {
                        let options = ParseOptions {
                            limits: pprs::Limits::new(1024, depth, tokens, nodes, diagnostics)
                                .unwrap(),
                            ..Default::default()
                        };
                        invariants(&pprs::parse(source, options).unwrap(), options);
                    }
                }
            }
        }
    }
}
