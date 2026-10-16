//! P03 experiments are test-only and do not certify parser compatibility.
mod strategy_support;
use serde_json::Value;
use strategy_support::{
    parser::{Parser, Tree},
    scanner::Kind,
};

fn cases() -> Value {
    serde_json::from_str(include_str!("fixtures/strategy/cases.json")).unwrap()
}
fn shape(tree: &Tree) -> String {
    if tree.children.is_empty() {
        tree.tag.into()
    } else {
        format!(
            "{}({})",
            tree.tag,
            tree.children
                .iter()
                .map(shape)
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
fn oracle_shape(node: &Value) -> String {
    let fields = &node["fields"];
    let unary = |tag: &str, nodes: Vec<String>| format!("{tag}({})", nodes.join(","));
    let fold_types = |tag: &str| {
        fields["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(oracle_shape)
            .reduce(|a, b| unary(tag, vec![a, b]))
            .unwrap()
    };
    match node["kind"].as_str().unwrap() {
        "Stmt_Expression" => oracle_shape(&fields["expr"]),
        "Expr_Assign" => unary(
            "assign",
            vec![oracle_shape(&fields["var"]), oracle_shape(&fields["expr"])],
        ),
        "Expr_BinaryOp_Concat" | "Expr_BinaryOp_Plus" | "Expr_BinaryOp_Mul" => {
            let tag = match node["kind"].as_str().unwrap() {
                "Expr_BinaryOp_Concat" => "concat",
                "Expr_BinaryOp_Plus" => "add",
                _ => "mul",
            };
            unary(
                tag,
                vec![
                    oracle_shape(&fields["left"]),
                    oracle_shape(&fields["right"]),
                ],
            )
        }
        "Expr_FuncCall" => {
            let mut args = vec![oracle_shape(&fields["name"])];
            args.extend(
                fields["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| oracle_shape(&a["fields"]["value"])),
            );
            unary("call", args)
        }
        "Expr_MethodCall" | "Expr_NullsafeMethodCall" => {
            let tag = if node["kind"] == "Expr_MethodCall" {
                "member"
            } else {
                "null-member"
            };
            let mut args = vec![unary(
                tag,
                vec![oracle_shape(&fields["var"]), oracle_shape(&fields["name"])],
            )];
            args.extend(
                fields["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| oracle_shape(&a["fields"]["value"])),
            );
            unary("call", args)
        }
        "Expr_ArrayDimFetch" => unary(
            "index",
            vec![oracle_shape(&fields["var"]), oracle_shape(&fields["dim"])],
        ),
        "Stmt_Function" => unary(
            "function",
            vec![oracle_shape(&fields["params"][0]["fields"]["type"])],
        ),
        "UnionType" => fold_types("union"),
        "IntersectionType" => fold_types("intersection"),
        "Stmt_HaltCompiler" => "halt".into(),
        "Expr_Variable"
        | "Scalar_Int"
        | "Scalar_String"
        | "Scalar_InterpolatedString"
        | "Identifier"
        | "Name" => "atom".into(),
        other => panic!("unmapped prototype oracle node {other}"),
    }
}
fn check_spans(parser: &Parser<'_>, source: &[u8]) {
    let mut end = 0;
    for token in &parser.tokens {
        assert_eq!(token.span.start, end);
        assert!(token.span.end <= source.len());
        end = token.span.end;
    }
    assert_eq!(end, source.len());
    assert_eq!(parser.tokens.last().unwrap().kind, Kind::Eof);
    for event in &parser.scanner.events {
        assert!(event.span.start <= event.span.end && event.span.end <= source.len());
    }
}
#[test]
fn hard_cases_match_reference_shape_or_explicit_recovery_contract() {
    let data = cases();
    for case in data["cases"].as_array().unwrap() {
        let source = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        let mut parser = Parser::new(&source, case["target"] != "7.4");
        let trees = parser.program(0);
        check_spans(&parser, &source);
        let errors = parser.errors.len() + parser.scanner.errors.len();
        assert_eq!(
            errors > 0,
            case["errors"].as_bool().unwrap(),
            "{} {:?}",
            case["id"],
            parser.errors
        );
        if errors == 0 {
            let expected: Vec<_> = case["oracle"]["ast"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|n| n["kind"] != "Stmt_InlineHTML")
                .map(oracle_shape)
                .collect();
            assert_eq!(
                trees.iter().map(shape).collect::<Vec<_>>(),
                expected,
                "{}",
                case["id"]
            );
        }
        match case["id"].as_str().unwrap() {
            "nested-interpolation" => assert_eq!(
                parser
                    .scanner
                    .events
                    .iter()
                    .filter(|e| e.kind == "interpolation")
                    .count(),
                2
            ),
            "heredoc" | "nowdoc" => assert!(
                parser
                    .scanner
                    .events
                    .iter()
                    .any(|e| e.kind == "heredoc-label" && &source[e.span.clone()] == b"END")
            ),
            "missing-operand" | "missing-call-arg" | "halt-invalid" => {
                assert_eq!(&source[trees.last().unwrap().span.clone()], b"7");
            }
            "halt-nested" => assert_eq!(
                &source[trees[0].children.last().unwrap().span.clone()],
                b"7"
            ),
            "halt-tail" => {
                let tail = parser.tokens.iter().find(|t| t.kind == Kind::Tail).unwrap();
                assert_eq!(&source[tail.span.clone()], b"\xff\x00\"unterminated");
            }
            _ => {}
        }
    }
}
#[test]
fn every_truncation_finishes_and_preserves_bytes() {
    for case in cases()["cases"].as_array().unwrap() {
        let bytes = hex::decode(case["source"]["bytes"].as_str().unwrap()).unwrap();
        for end in 0..=bytes.len() {
            let source = &bytes[..end];
            let mut parser = Parser::new(source, true);
            parser.program(0);
            check_spans(&parser, source);
        }
    }
}

#[test]
fn trivia_between_reference_marker_and_variable_does_not_change_type() {
    let source = b"<?php function f(A&B & /* reference */ $x) {}";
    let mut parser = Parser::new(source, true);
    let trees = parser.program(0);
    assert_eq!(parser.errors.len(), 0);
    assert_eq!(shape(&trees[0]), "function(intersection(atom,atom))");
    check_spans(&parser, source);
}

#[test]
fn arbitrary_bytes_and_depth_limit_make_progress() {
    for byte in 0..=u8::MAX {
        let mut source = b"<?php ".to_vec();
        source.extend([byte, byte, b';', b'7', b';']);
        let mut parser = Parser::new(&source, true);
        parser.program(0);
        check_spans(&parser, &source);
    }
    for source in [
        format!("<?php {}1{}; 7;", "(".repeat(128), ")".repeat(128)),
        format!(
            "<?php {}7;{}",
            "function f(A &$x) {".repeat(128),
            "}".repeat(128)
        ),
    ] {
        let mut parser = Parser::new(source.as_bytes(), true);
        parser.program(0);
        assert_ne!(parser.errors.len(), 0);
        check_spans(&parser, source.as_bytes());
    }
}

#[test]
fn hard_case_provenance_is_pinned() {
    use sha2::{Digest, Sha256};
    let data = cases();
    let lock: Value = serde_json::from_str(include_str!("../reference/lock.json")).unwrap();
    assert_eq!(data["reference_commit"], lock["commit"]);
    let definitions = include_bytes!("../tools/strategy/inputs.json");
    assert_eq!(
        data["inputs_sha256"],
        hex::encode(Sha256::digest(definitions))
    );
    let definitions: Value = serde_json::from_slice(definitions).unwrap();
    assert_eq!(
        data["cases"].as_array().unwrap().len(),
        definitions.as_array().unwrap().len()
    );
    let grammar: Vec<Value> = include_str!("../docs/compatibility/grammar.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for (case, definition) in data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(definitions.as_array().unwrap())
    {
        for (key, value) in definition.as_object().unwrap() {
            assert_eq!(&case[key], value);
        }
        for production in case["productions"].as_array().unwrap() {
            assert!(grammar.iter().any(|entry| entry["name"] == *production));
        }
        if case.get("oracle_failure").is_some() {
            assert_eq!(case["id"], "halt-invalid");
            assert_eq!(
                case["oracle_failure"],
                "assert($prevToken->id == \\T_CLOSE_TAG)"
            );
        }
    }
}
