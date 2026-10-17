//! Shared parser ownership, metadata and resource invariants.
use pprs::{ParseOptions, ParseStatus};
pub fn invariants(result: &pprs::ParseResult, options: ParseOptions) {
    use pprs::{ast::Origin, options::Resource, visit::Node};
    let mut offset = 0;
    for (index, token) in result.tokens().tokens().iter().enumerate() {
        assert_eq!(token.span().start(), offset);
        offset = token.span().end();
        assert_eq!(
            result.tokens().text(index).unwrap(),
            &result.source().as_bytes()[token.span().range()]
        );
    }
    assert_eq!(offset, result.source().len());
    assert!(result.tokens().tokens().len() <= options.limits.get(Resource::Tokens));
    assert!(result.diagnostics().len() <= options.limits.get(Resource::Diagnostics));
    diagnostic_invariants(result);
    if let Some(program) = result.program() {
        assert!(pprs::visit::walk(program).count() <= options.limits.get(Resource::Nodes));
        let mut comments = std::collections::HashSet::new();
        for entry in pprs::visit::walk(program) {
            assert!(entry.depth <= options.limits.get(Resource::Depth));
            let meta = match entry.node {
                Node::Program(_) => continue,
                Node::Stmt(n) => &n.meta,
                Node::Expr(n) => &n.meta,
                Node::Name(n) => n.meta(),
                Node::Identifier(n) => n.meta(),
                Node::Argument(n) => &n.meta,
                Node::ArrayItem(n) => &n.meta,
                Node::StringPart(pprs::ast::StringPart::Text { meta, .. }) => meta,
                Node::StringPart(pprs::ast::StringPart::Expr(e)) => &e.meta,
                Node::MatchArm(n) => &n.meta,
                Node::Type(n) => n.meta(),
                Node::TraitAdaptation(n) => n.meta(),
                Node::AnonymousClass(n) => &n.meta,
                Node::Attribute(n) => &n.meta,
                Node::AttributeGroup(n) => &n.meta,
                Node::Constant(n) => &n.meta,
                Node::UseItem(n) => &n.meta,
                Node::PropertyItem(n) => &n.meta,
                Node::PropertyHook(n) => &n.meta,
                Node::Parameter(n) => &n.meta,
                Node::ClosureUse(n) => &n.meta,
                Node::ElseIf(n) => &n.meta,
                Node::Else(n) => &n.meta,
                Node::Case(n) => &n.meta,
                Node::Catch(n) => &n.meta,
                Node::Finally(n) => &n.meta,
                Node::StaticVariable(n) => &n.meta,
                Node::DeclareItem(n) => &n.meta,
                _ => panic!("new visitor node needs an independent invariant check"),
            };
            assert_eq!(entry.node.meta(), Some(meta));
            let Origin::Parsed { span, tokens } = meta.origin() else {
                panic!("synthetic parser node")
            };
            assert!(span.start() <= span.end() && span.end() <= result.source().len());
            assert!(
                tokens.start() <= tokens.end() && tokens.end() <= result.tokens().tokens().len()
            );
            assert!(result.source().slice(span).is_some());
            assert_eq!(result.tokens().span(tokens).unwrap(), span);
            let Origin::Parsed {
                span: outer,
                tokens: outer_tokens,
            } = meta.enclosing_origin()
            else {
                panic!("synthetic enclosing origin")
            };
            assert!(outer.start() <= span.start() && outer.end() >= span.end());
            assert_eq!(result.tokens().span(outer_tokens).unwrap(), outer);
            for &index in meta.comments() {
                assert!(comments.insert(index), "comment has more than one owner");
                assert!(result.tokens().tokens()[index].kind().is_comment());
                assert!(index < tokens.start());
            }
        }
    }
    if result.status() == ParseStatus::Clean {
        assert_eq!(result.diagnostics(), []);
    }
}

fn diagnostic_invariants(result: &pprs::ParseResult) {
    let mut syntax_locations = std::collections::HashSet::new();
    for (index, diagnostic) in result.diagnostics().iter().enumerate() {
        assert!(diagnostic.primary.end() <= result.source().len());
        assert!(result.source().slice(diagnostic.primary).is_some());
        for label in &diagnostic.secondary {
            assert!(result.source().slice(label.span).is_some());
        }
        for (position, kind) in diagnostic.expected.iter().enumerate() {
            assert!(!diagnostic.expected[..position].contains(kind));
        }
        if diagnostic.code == pprs::diagnostic::DiagnosticCode::UnexpectedToken {
            assert!(
                syntax_locations.insert(diagnostic.primary),
                "duplicate syntax cascade"
            );
        }
        if let Some(found) = diagnostic.found {
            assert!(
                result
                    .tokens()
                    .tokens()
                    .iter()
                    .any(|token| token.kind() == found && token.span() == diagnostic.primary)
            );
        }
        if matches!(
            diagnostic.code,
            pprs::diagnostic::DiagnosticCode::ResourceLimit(_)
        ) {
            assert_eq!(
                index + 1,
                result.diagnostics().len(),
                "resource failure must be terminal"
            );
            assert_eq!(result.status(), ParseStatus::Aborted);
        }
    }
}
