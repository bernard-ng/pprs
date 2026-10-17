//! Public traversal coverage and iterator contracts.
use pprs::{
    ParseOptions,
    ast::{ExprKind, StmtKind, Type},
    visit::{self, Node},
};
use std::collections::BTreeSet;

fn node_tag(value: Node<'_>) -> &'static str {
    match value {
        Node::AnonymousClass(..) => "AnonymousClass",
        Node::Attribute(..) => "Attribute",
        Node::AttributeGroup(..) => "AttributeGroup",
        Node::Constant(..) => "Constant",
        Node::UseItem(..) => "UseItem",
        Node::PropertyItem(..) => "PropertyItem",
        Node::PropertyHook(..) => "PropertyHook",
        Node::TraitAdaptation(..) => "TraitAdaptation",
        Node::Program(..) => "Program",
        Node::Stmt(..) => "Stmt",
        Node::Expr(..) => "Expr",
        Node::Name(..) => "Name",
        Node::Identifier(..) => "Identifier",
        Node::Argument(..) => "Argument",
        Node::ArrayItem(..) => "ArrayItem",
        Node::StringPart(..) => "StringPart",
        Node::Type(..) => "Type",
        Node::Parameter(..) => "Parameter",
        Node::ClosureUse(..) => "ClosureUse",
        Node::ElseIf(..) => "ElseIf",
        Node::Else(..) => "Else",
        Node::Case(..) => "Case",
        Node::Catch(..) => "Catch",
        Node::Finally(..) => "Finally",
        Node::StaticVariable(..) => "StaticVariable",
        Node::DeclareItem(..) => "DeclareItem",
        Node::MatchArm(..) => "MatchArm",
        _ => panic!("new variant needs traversal coverage"),
    }
}
const NODE_VARIANTS: &[&str] = &[
    "AnonymousClass",
    "Attribute",
    "AttributeGroup",
    "Constant",
    "UseItem",
    "PropertyItem",
    "PropertyHook",
    "TraitAdaptation",
    "Program",
    "Stmt",
    "Expr",
    "Name",
    "Identifier",
    "Argument",
    "ArrayItem",
    "StringPart",
    "Type",
    "Parameter",
    "ClosureUse",
    "ElseIf",
    "Else",
    "Case",
    "Catch",
    "Finally",
    "StaticVariable",
    "DeclareItem",
    "MatchArm",
];
fn expr_tag(value: &ExprKind) -> &'static str {
    match value {
        ExprKind::NewAnonymous(..) => "NewAnonymous",
        ExprKind::Integer { .. } => "Integer",
        ExprKind::Float(..) => "Float",
        ExprKind::String { .. } => "String",
        ExprKind::Variable(..) => "Variable",
        ExprKind::ConstFetch(..) => "ConstFetch",
        ExprKind::Assign { .. } => "Assign",
        ExprKind::Binary { .. } => "Binary",
        ExprKind::DynamicVariable(..) => "DynamicVariable",
        ExprKind::MagicConstant(..) => "MagicConstant",
        ExprKind::Unary { .. } => "Unary",
        ExprKind::AssignOp { .. } => "AssignOp",
        ExprKind::AssignRef { .. } => "AssignRef",
        ExprKind::Ternary { .. } => "Ternary",
        ExprKind::Array { .. } => "Array",
        ExprKind::List { .. } => "List",
        ExprKind::ArrayDim { .. } => "ArrayDim",
        ExprKind::Call { .. } => "Call",
        ExprKind::Property { .. } => "Property",
        ExprKind::MethodCall { .. } => "MethodCall",
        ExprKind::StaticProperty { .. } => "StaticProperty",
        ExprKind::ClassConst { .. } => "ClassConst",
        ExprKind::StaticCall { .. } => "StaticCall",
        ExprKind::New { .. } => "New",
        ExprKind::Instanceof { .. } => "Instanceof",
        ExprKind::Cast { .. } => "Cast",
        ExprKind::Include { .. } => "Include",
        ExprKind::Isset(..) => "Isset",
        ExprKind::Exit(..) => "Exit",
        ExprKind::Yield { .. } => "Yield",
        ExprKind::Match { .. } => "Match",
        ExprKind::Interpolated { .. } => "Interpolated",
        ExprKind::Closure(..) => "Closure",
        ExprKind::ArrowFunction(..) => "ArrowFunction",
        ExprKind::ShellExec(..) => "ShellExec",
        ExprKind::Error => "Error",
        _ => panic!("new variant needs traversal coverage"),
    }
}
const EXPRKIND_VARIANTS: &[&str] = &[
    "NewAnonymous",
    "Integer",
    "Float",
    "String",
    "Variable",
    "ConstFetch",
    "Assign",
    "Binary",
    "DynamicVariable",
    "MagicConstant",
    "Unary",
    "AssignOp",
    "AssignRef",
    "Ternary",
    "Array",
    "List",
    "ArrayDim",
    "Call",
    "Property",
    "MethodCall",
    "StaticProperty",
    "ClassConst",
    "StaticCall",
    "New",
    "Instanceof",
    "Cast",
    "Include",
    "Isset",
    "Exit",
    "Yield",
    "Match",
    "Interpolated",
    "Closure",
    "ArrowFunction",
    "ShellExec",
    "Error",
];
fn stmt_tag(value: &StmtKind) -> &'static str {
    match value {
        StmtKind::Namespace(..) => "Namespace",
        StmtKind::Use(..) => "Use",
        StmtKind::Constants(..) => "Constants",
        StmtKind::Class(..) => "Class",
        StmtKind::ClassConstants(..) => "ClassConstants",
        StmtKind::Method(..) => "Method",
        StmtKind::Property(..) => "Property",
        StmtKind::TraitUse(..) => "TraitUse",
        StmtKind::EnumCase(..) => "EnumCase",
        StmtKind::Expression(..) => "Expression",
        StmtKind::Nop => "Nop",
        StmtKind::InlineHtml(..) => "InlineHtml",
        StmtKind::Echo(..) => "Echo",
        StmtKind::Return(..) => "Return",
        StmtKind::Block(..) => "Block",
        StmtKind::Function(..) => "Function",
        StmtKind::If(..) => "If",
        StmtKind::While(..) => "While",
        StmtKind::Do(..) => "Do",
        StmtKind::For(..) => "For",
        StmtKind::Foreach(..) => "Foreach",
        StmtKind::Switch(..) => "Switch",
        StmtKind::Break(..) => "Break",
        StmtKind::Continue(..) => "Continue",
        StmtKind::Goto(..) => "Goto",
        StmtKind::Label(..) => "Label",
        StmtKind::Global(..) => "Global",
        StmtKind::Static(..) => "Static",
        StmtKind::Unset(..) => "Unset",
        StmtKind::Declare(..) => "Declare",
        StmtKind::Try(..) => "Try",
        StmtKind::HaltCompiler(..) => "HaltCompiler",
        _ => panic!("new variant needs traversal coverage"),
    }
}
const STMTKIND_VARIANTS: &[&str] = &[
    "Namespace",
    "Use",
    "Constants",
    "Class",
    "ClassConstants",
    "Method",
    "Property",
    "TraitUse",
    "EnumCase",
    "Expression",
    "Nop",
    "InlineHtml",
    "Echo",
    "Return",
    "Block",
    "Function",
    "If",
    "While",
    "Do",
    "For",
    "Foreach",
    "Switch",
    "Break",
    "Continue",
    "Goto",
    "Label",
    "Global",
    "Static",
    "Unset",
    "Declare",
    "Try",
    "HaltCompiler",
];

#[test]
fn every_node_expression_statement_and_composite_type_is_walked() {
    let mut nodes = BTreeSet::new();
    let mut expressions = BTreeSet::new();
    let mut statements = BTreeSet::new();
    let mut types = BTreeSet::new();
    for line in include_str!("fixtures/upstream/cases.jsonl").lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        if row["scope"] != "core" {
            continue;
        }
        let bytes = hex::decode(row["source"]["bytes"].as_str().unwrap()).unwrap();
        let options = ParseOptions {
            version: row["oracle"]["target"].as_str().unwrap().parse().unwrap(),
            ..Default::default()
        };
        let result = pprs::parse(bytes, options).unwrap();
        if let Some(program) = result.program() {
            for entry in visit::walk(program) {
                nodes.insert(node_tag(entry.node));
                match entry.node {
                    Node::Expr(e) => {
                        expressions.insert(expr_tag(&e.kind));
                    }
                    Node::Stmt(s) => {
                        statements.insert(stmt_tag(&s.kind));
                    }
                    Node::Type(t) => {
                        types.insert(match t {
                            Type::Nullable { .. } => "Nullable",
                            Type::Union { .. } => "Union",
                            Type::Intersection { .. } => "Intersection",
                            Type::Name(_) | Type::Builtin(_) => {
                                panic!("simple types must not be wrapped")
                            }
                        });
                    }
                    _ => {}
                }
                assert_eq!(
                    entry.node.meta().is_none(),
                    matches!(entry.node, Node::Program(_))
                );
            }
        }
    }
    assert_eq!(nodes, NODE_VARIANTS.iter().copied().collect());
    assert_eq!(expressions, EXPRKIND_VARIANTS.iter().copied().collect());
    assert_eq!(statements, STMTKIND_VARIANTS.iter().copied().collect());
    assert_eq!(
        types,
        ["Intersection", "Nullable", "Union"].into_iter().collect()
    );
}

#[test]
fn early_exit_resume_filtering_and_exhaustion() {
    fn fused(_: &impl std::iter::FusedIterator) {}
    let file = pprs::parse_strict(
        "<?php function f() { echo 1; } echo 2;",
        ParseOptions::default(),
    )
    .unwrap();
    let mut walk = visit::walk(file.program());
    fused(&walk);
    let found = walk
        .find(
            |entry| matches!(entry.node, Node::Stmt(s) if matches!(s.kind, StmtKind::Function(_))),
        )
        .unwrap();
    assert_eq!(found.depth, 2);
    assert!(
        matches!(walk.next().unwrap().node, Node::Identifier(id) if id.bytes().as_bytes() == b"f")
    );
    assert!(walk.any(|entry| matches!(entry.node, Node::Expr(e) if matches!(e.kind, ExprKind::Integer { value: 1, .. }))));
    assert!(
        matches!(walk.next().unwrap().node, Node::Stmt(s) if matches!(s.kind, StmtKind::Echo(_)))
    );
    assert!(walk.all(|entry| entry.depth == 3));
    assert!(walk.next().is_none());
    assert!(walk.next().is_none());
    // Filtering out statements still traverses their expression children.
    let values: Vec<_> = visit::walk(file.program())
        .filter_map(|entry| match entry.node {
            Node::Expr(e) => {
                if let ExprKind::Integer { value, .. } = e.kind {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(values, [1, 2]);
    let mut visited = 0;
    let stopped = visit::walk(file.program()).try_for_each(|entry| {
        visited += 1;
        if entry.depth == 2 {
            std::ops::ControlFlow::Break(entry.depth)
        } else {
            std::ops::ControlFlow::Continue(())
        }
    });
    assert_eq!(visited, 2);
    assert_eq!(stopped, std::ops::ControlFlow::Break(2));
}

#[test]
fn control_flow_order_is_syntactic_not_evaluation_order() {
    let file = pprs::parse_strict(
        "<?php for (init(); condition(); step()) { body(); } do { first(); } while (last());",
        ParseOptions::default(),
    )
    .unwrap();
    let names: Vec<_> = visit::walk(file.program())
        .filter_map(|entry| match entry.node {
            Node::Name(name) => Some(name.parts()[0].as_bytes()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        [
            b"init".as_slice(),
            b"condition",
            b"step",
            b"body",
            b"first",
            b"last"
        ]
    );
}
