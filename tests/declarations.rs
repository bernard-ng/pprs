//! P09 declaration grammar and public AST contract.
#[path = "support/parser.rs"]
mod adapter;
use pprs::{ParseOptions, ParseStatus};
use serde_json::Value;
use sha2::{Digest, Sha256};
#[test]
fn declaration_oracle_is_reproducible_and_matches_every_probe() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/declarations/declarations.json")).unwrap();
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
        serde_json::from_str(include_str!("../tools/declarations/inputs.json")).unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), inputs.as_array().unwrap().len());
    assert_eq!(cases.len(), 60);
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
fn declaration_truncations_and_small_budgets_preserve_invariants() {
    use pprs::Limits;
    let inputs: Value =
        serde_json::from_str(include_str!("../tools/declarations/inputs.json")).unwrap();
    for input in inputs.as_array().unwrap() {
        let source = hex::decode(input["source"]["bytes"].as_str().unwrap()).unwrap();
        let options = ParseOptions {
            version: input["target"].as_str().unwrap().parse().unwrap(),
            ..Default::default()
        };
        for end in 0..=source.len() {
            invariants(&pprs::parse(&source[..end], options).unwrap(), options);
        }
        for budget in 1..=24 {
            for limits in [
                Limits::new(10000, budget, 10000, 10000, 20).unwrap(),
                Limits::new(10000, 128, 10000, budget, 20).unwrap(),
                Limits::new(10000, 128, budget.max(2), 10000, 20).unwrap(),
            ] {
                let options = ParseOptions { limits, ..options };
                invariants(&pprs::parse(&source, options).unwrap(), options);
            }
        }
    }
}

#[test]
fn deep_declarations_attributes_hooks_and_namespaces_stop_at_the_budget() {
    use pprs::{diagnostic::DiagnosticCode, options::Resource};
    for (prefix, suffix, middle) in [
        ("class C {function f(){", "}}", "echo 1;"),
        ("namespace A {", "}", "echo 1;"),
        ("$x=new class {public $x=", ";};", "1"),
        ("#[A(new class {public $x=", ";})] class X {}", "1"),
        ("class C {public $x {get {", "}}}", "echo 1;"),
    ] {
        let source = format!(
            "<?php {}{middle}{}",
            prefix.repeat(1000),
            suffix.repeat(1000)
        );
        let result = pprs::parse(source, ParseOptions::default()).unwrap();
        assert_eq!(result.status(), ParseStatus::Aborted, "{prefix}");
        assert_eq!(
            result.diagnostics().last().unwrap().code,
            DiagnosticCode::ResourceLimit(Resource::Depth),
            "{prefix}"
        );
        invariants(&result, ParseOptions::default());
    }
    let source = format!(
        "<?php class C {{function f({}$x{}) {{}} }}",
        "public $x {set(".repeat(1000),
        ") {}}".repeat(1000)
    );
    let result = pprs::parse(source, ParseOptions::default()).unwrap();
    assert_eq!(result.status(), ParseStatus::Aborted);
    invariants(&result, ParseOptions::default());
}

#[test]
fn malformed_declarations_and_hooks_retain_later_members_and_statements() {
    use pprs::ast::StmtKind;
    for source in [
        "<?php class C { public $bad = ; public $good; function ok() {} } echo 42;",
        "<?php class C { public $bad { get => ; set($v) {} } public $good; function ok() {} } echo 42;",
        "<?php class C { public $bad { get ???; set => 1; } public $good; function ok() {} } echo 42;",
        "<?php class C { function bad($x, @, $y) {} public $good; function ok() {} } echo 42;",
        "<?php class C { nonsense; public $good; function ok() {} } echo 42;",
        "<?php class C { const X; public $good; function ok() {} } echo 42;",
        "<?php class C { use T { T::f insteadof ; } public $good; function ok() {} } echo 42;",
        "<?php class C { public $bad { get => 1; public $good; function ok() {} } echo 42;",
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
        let StmtKind::Class(c) = &program.statements[0].kind else {
            panic!("lost class {source}")
        };
        assert!(c.members.iter().any(|s|matches!(&s.kind,StmtKind::Property(p) if p.items.iter().any(|i|i.name.bytes().as_bytes()==b"good"))),"{source}");
        assert!(
            c.members
                .iter()
                .any(|s| matches!(&s.kind,StmtKind::Method(m) if m.name.bytes().as_bytes()==b"ok")),
            "{source}"
        );
        invariants(&result, ParseOptions::default());
    }
}

#[test]
fn invalid_declaration_productions_are_never_clean() {
    for source in [
        "namespace;",
        "namespace \\A;",
        "use Foo as static;",
        "use Foo\\{};",
        "use function Foo\\{const A};",
        "class C extends A,B {}",
        "trait T implements I {}",
        "interface I implements J {}",
        "enum E extends C {}",
        "public class C {}",
        "final interface I {}",
        "new interface I {};",
        "class static {}",
        "class C {public static static $x;}",
        "class C {public(set) private(set) $x;}",
        "class C {var function f(){}}",
        "class C {public static $x {get;} readonly function f(){}}",
        "class C {const X;}",
        "class C {public static $x {private get;}}",
        "class C {public $x {get();}}",
        "class C {public $x {unknown;}}",
        "class C {public $x {}}",
        "class C {public $x,$y {get;}}",
        "class C {use T {f insteadof T;}}",
        "class C {use T {f as;}}",
        "class C {use T {f as public private x;}}",
        "#[A] echo 1;",
        "#[A] const X=1;",
        "class C {function __construct(final public $x){}}",
        "class C {#[A] $x;}",
        "class C {public static $x; function f(public static $x){}}",
        "class C {public static $x; public static function __clone(){}}",
        "class C {public static $x; readonly const X=1;}",
        "if($x) class C {}",
        "while($x) #[A] class C {}",
        "if($x) #[A] function f() {}",
        "function f(){namespace A;}",
        "function f(){use A;}",
        "function f(){const X=1;}",
    ] {
        let result = pprs::parse(format!("<?php {source}"), ParseOptions::default()).unwrap();
        assert_ne!(result.status(), ParseStatus::Clean, "{source}");
        assert!(result.into_strict().is_err());
    }
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(256))]
    #[test]
    fn arbitrary_declaration_tokens_make_progress(indices in proptest::collection::vec(0usize..30,0..96), depth in 1usize..40) {
        let words=["class", "interface", "trait", "enum", "namespace", "use", "as", "insteadof", "public", "private(set)", "function", "const", "static", "readonly", "final", "#[", "$x", "A", "get", "set", "(", ")", "{", "}", "[", "]", "&", ",", ";", "=>"];
        let body=indices.iter().map(|i|words[*i]).collect::<Vec<_>>().join(" ");
        let options=ParseOptions{limits:pprs::Limits::new(10000,depth,512,512,16).unwrap(),..Default::default()};
        for prefix in ["<?php ","<?php class C {", "<?php class C {public $x {", "<?php use Foo\\{", "<?php class C {function f("] {
            let result=pprs::parse(format!("{prefix}{body}"),options).unwrap();
            invariants(&result,options);
        }
    }
}

#[test]
fn hook_recovery_obeys_the_three_token_cursor_window() {
    let options = ParseOptions {
        limits: pprs::Limits::new(10000, 4, 512, 512, 16).unwrap(),
        ..Default::default()
    };
    let result = pprs::parse("<?php class C {public $x {public public public", options).unwrap();
    assert_ne!(result.status(), ParseStatus::Clean);
    invariants(&result, options);
}

#[test]
fn namespace_grouping_and_declaration_traversal_preserve_source_order() {
    use pprs::{
        ast::{NamespaceStyle, Origin, StmtKind},
        visit::Node,
    };
    let parsed=pprs::parse_strict("<?php namespace A; #[Before] class C { #[Prop] public X $x { #[Hook] get => 1; } function f(#[Param] public Y $y): Z {} } namespace B; use Foo as Bar;",ParseOptions::default()).unwrap();
    let result = parsed.result();
    let program = result.program().unwrap();
    assert_eq!(program.statements.len(), 2);
    let StmtKind::Namespace(ns) = &program.statements[0].kind else {
        panic!()
    };
    assert_eq!(ns.style, NamespaceStyle::Semicolon);
    assert_eq!(ns.body.as_ref().unwrap().len(), 1);
    let Origin::Parsed { span: parent, .. } = program.statements[0].meta.origin() else {
        panic!()
    };
    let Origin::Parsed { span: child, .. } = ns.body.as_ref().unwrap()[0].meta.origin() else {
        panic!()
    };
    assert!(parent.start() <= child.start() && parent.end() >= child.end());
    let names: Vec<_> = pprs::visit::walk(program)
        .filter_map(|entry| match entry.node {
            Node::Name(n) => Some(n.parts()[0].as_bytes()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        [
            b"A".as_slice(),
            b"Before",
            b"Prop",
            b"X",
            b"Hook",
            b"Param",
            b"Y",
            b"Z",
            b"B",
            b"Foo"
        ]
    );
    invariants(result, ParseOptions::default());

    let parsed = pprs::parse_strict(
        "<?php $x=new #[Attr] class(arg()) extends Base {function f(){body();}};",
        ParseOptions::default(),
    )
    .unwrap();
    let names: Vec<_> = pprs::visit::walk(parsed.result().program().unwrap())
        .filter_map(|entry| match entry.node {
            Node::Name(n) => Some(n.parts()[0].as_bytes()),
            _ => None,
        })
        .collect();
    assert_eq!(names, [b"Attr".as_slice(), b"arg", b"Base", b"body"]);
}

#[test]
fn damaged_top_level_declarations_keep_following_valid_statements() {
    for damaged in [
        "class static {}",
        "interface static {}",
        "class ReadOnly {}",
        "use Foo {Bar,Baz};",
        "use Foo\\{\\Bar};",
    ] {
        let result =
            pprs::parse(format!("<?php {damaged} echo 42;"), ParseOptions::default()).unwrap();
        assert_eq!(result.status(), ParseStatus::Recovered);
        assert!(matches!(
            result.program().unwrap().statements.last().unwrap().kind,
            pprs::ast::StmtKind::Echo(_)
        ));
    }
}
