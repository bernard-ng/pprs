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
    if let Some(program) = result.program() {
        assert!(pprs::visit::walk(program).count() <= options.limits.get(Resource::Nodes));
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
                Node::Parameter(n) => &n.meta,
                Node::ClosureUse(n) => &n.meta,
                Node::ElseIf(n) => &n.meta,
                Node::Else(n) => &n.meta,
                Node::Case(n) => &n.meta,
                Node::Catch(n) => &n.meta,
                Node::Finally(n) => &n.meta,
                Node::StaticVariable(n) => &n.meta,
                Node::DeclareItem(n) => &n.meta,
            };
            let Origin::Parsed { span, tokens } = meta.origin() else {
                panic!("synthetic parser node")
            };
            assert!(span.start() <= span.end() && span.end() <= result.source().len());
            assert!(
                tokens.start() <= tokens.end() && tokens.end() <= result.tokens().tokens().len()
            );
            assert!(result.source().slice(span).is_some());
        }
    }
    if result.status() == ParseStatus::Clean {
        assert_eq!(result.diagnostics(), []);
    }
}
