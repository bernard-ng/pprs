//! P08 procedural grammar and callable reference probes.
#[path = "support/parser.rs"]
mod adapter;
use pprs::{ParseOptions, ParseStatus};
use serde_json::Value;
use sha2::{Digest, Sha256};
#[test]
fn statement_oracle_is_reproducible_and_matches_every_probe() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/statements/statements.json")).unwrap();
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
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/statements/inputs.json")).unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), inputs.as_array().unwrap().len());
    assert_eq!(cases.len(), 49);
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
fn every_statement_probe_truncation_and_small_budget_is_bounded() {
    use pprs::options::Limits;
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/statements/inputs.json")).unwrap();
    for input in inputs.as_array().unwrap() {
        let bytes = hex::decode(input["source"]["bytes"].as_str().unwrap()).unwrap();
        let options = ParseOptions {
            version: input["target"].as_str().unwrap().parse().unwrap(),
            ..Default::default()
        };
        for end in 0..=bytes.len() {
            let result = pprs::parse(&bytes[..end], options).unwrap();
            invariants(&result, options);
        }
        for budget in 1..=24 {
            for limits in [
                Limits::new(10000, budget, 10000, 10000, 20).unwrap(),
                Limits::new(10000, 128, 10000, budget, 20).unwrap(),
                Limits::new(10000, 128, budget.max(2), 10000, 20).unwrap(),
            ] {
                let options = ParseOptions { limits, ..options };
                invariants(&pprs::parse(&bytes, options).unwrap(), options);
            }
        }
    }
}

#[test]
fn damaged_statements_signatures_and_catches_keep_following_code() {
    use pprs::{ast::StmtKind, visit::Node};
    for source in [
        "<?php function f($a, @, $b) { echo 42; } echo 42;",
        "<?php function f() echo 42;",
        "<?php if ($a) { echo ; echo 42; } echo 42;",
        "<?php while($a): echo ; echo 42; endwhile; echo 42;",
        "<?php try {} catch (A|) {echo 1;} catch(B $e) {echo 42;} finally {echo 42;} echo 42;",
        "<?php global $$foo->bar; echo 42;",
        "<?php switch ($x) {case 1: echo ; echo 42; default: echo 42;} echo 42;",
    ] {
        let result = pprs::parse(source, ParseOptions::default()).unwrap();
        assert_eq!(
            result.status(),
            ParseStatus::Recovered,
            "{source}: {:?}",
            result.diagnostics()
        );
        let program = result.program().unwrap();
        assert!(
            matches!(program.statements.last().unwrap().kind, StmtKind::Echo(_)),
            "{source}"
        );
        if source.contains("catch (A|)") {
            let StmtKind::Try(t) = &program.statements[0].kind else {
                panic!("lost try")
            };
            assert_eq!(t.catches.len(), 1);
            assert!(t.finally.is_some());
        }
        if source.contains("$a, @, $b") {
            let StmtKind::Function(f) = &program.statements[0].kind else {
                panic!("lost function")
            };
            assert_eq!(f.signature.params.len(), 2);
            assert!(
                pprs::visit::walk(program)
                    .any(|e| matches!(e.node,Node::Stmt(s) if matches!(s.kind,StmtKind::Echo(_))))
            );
        }
        invariants(&result, ParseOptions::default());
    }
}

#[test]
fn invalid_procedural_and_type_grammar_is_never_clean() {
    for source in [
        "function f(A&B|C $x) {}",
        "function f(A|B&C $x) {}",
        "function f((A&B) $x) {}",
        "function f((A|B)&C $x) {}",
        "function f(((A&B))|C $x) {}",
        "function f(?A|B $x) {}",
        "function f(static $x) {}",
        "function f(void $x) {}",
        "function f(...$x=1) {}",
        "$f=function() use () {};",
        "$f=function() use ($x->y) {};",
        "foreach($xs as [$a] => $b) {}",
        "foreach($xs as &[$a]) {}",
        "foreach($xs as &list($a)) {}",
        "foreach($xs as array($a)) {}",
        "try {}",
        "if($x): echo 1;",
        "declare(ticks=1,) {}",
        "unset();",
        "function f($a,, $b) {}",
        "catch(A $e) {}",
        "if($x) function f() {}",
        "while($x) function f() {}",
        "else echo 1;",
    ] {
        let result = pprs::parse(format!("<?php {source}"), ParseOptions::default()).unwrap();
        assert_ne!(result.status(), ParseStatus::Clean, "{source}");
        assert!(result.into_strict().is_err());
    }
}

#[test]
fn procedural_nesting_stops_before_exhausting_the_native_stack() {
    use pprs::{diagnostic::DiagnosticCode, options::Resource};
    for (prefix, suffix) in [
        ("if($x) ", ""),
        ("while($x){", "}"),
        ("for(;;){", "}"),
        ("foreach($xs as $x){", "}"),
        ("declare(ticks=1){", "}"),
        ("do {", "} while($x);"),
        ("function f(){", "}"),
        ("$f=function(){", "};"),
        ("fn()=>", ""),
        ("try {", "} finally {}"),
        ("switch($x){default:", "}"),
        ("if($x):", "endif;"),
    ] {
        let source = format!("<?php {}1;{}", prefix.repeat(1000), suffix.repeat(1000));
        let result = pprs::parse(source, ParseOptions::default()).unwrap();
        assert_eq!(result.status(), ParseStatus::Aborted, "{prefix}");
        assert_eq!(
            result.diagnostics().last().unwrap().code,
            DiagnosticCode::ResourceLimit(Resource::Depth)
        );
        invariants(&result, ParseOptions::default());
    }
    let source = format!(
        "<?php function f({}A&B{}|C $x) {{}}",
        "(".repeat(1000),
        ")".repeat(1000)
    );
    let result = pprs::parse(source, ParseOptions::default()).unwrap();
    assert_eq!(result.status(), ParseStatus::Aborted);
}

#[test]
fn traversal_visits_signature_captures_and_return_type_in_source_order() {
    use pprs::visit::Node;
    let parsed = pprs::parse_strict(
        "<?php $f=function(A $x) use (&$y): B { return $x; };",
        ParseOptions::default(),
    )
    .unwrap();
    let nodes: Vec<_> = pprs::visit::walk(parsed.result().program().unwrap())
        .filter_map(|entry| match entry.node {
            Node::Parameter(_) => Some("param"),
            Node::ClosureUse(_) => Some("capture"),
            Node::Name(n) if n.parts()[0].as_bytes() == b"B" => Some("return type"),
            _ => None,
        })
        .collect();
    assert_eq!(nodes, ["param", "capture", "return type"]);
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(256))]
    #[test]
    fn arbitrary_procedural_token_sequences_make_progress(indices in proptest::collection::vec(0usize..24,0..96), depth in 1usize..40) {
        let words=["if", "else", "elseif", "while", "for", "foreach", "as", "try", "catch", "finally", "function", "fn", "use", "$x", "A", "(", ")", "{", "}", "[", "]", "&", ",", ";"];
        let body=indices.iter().map(|i|words[*i]).collect::<Vec<_>>().join(" ");
        let options=ParseOptions{limits:pprs::Limits::new(10000,depth,512,512,16).unwrap(),..Default::default()};
        for prefix in ["<?php ","<?php function f(","<?php try {} catch (", "<?php if ($x):"] {
            let result=pprs::parse(format!("{prefix}{body}"),options).unwrap();
            invariants(&result,options);
        }
    }
}
