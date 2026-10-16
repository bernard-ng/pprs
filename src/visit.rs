//! Read-only, iterative traversal in source order, including auxiliary AST nodes.
use crate::ast::{
    Argument, ArrayItem, Expr, ExprKind, Identifier, MatchArm, Member, Name, Program, Stmt,
    StmtKind, StringPart, Target,
};
/// A borrowed AST node; traversal does not mutate metadata or semantic values.
#[derive(Clone, Copy, Debug)]
pub enum Node<'a> {
    /// File root.
    Program(&'a Program),
    /// Statement.
    Stmt(&'a Stmt),
    /// Expression.
    Expr(&'a Expr),
    /// Name.
    Name(&'a Name),
    /// Identifier.
    Identifier(&'a Identifier),
    /// Argument/placeholder.
    Argument(&'a Argument),
    /// Array item.
    ArrayItem(&'a ArrayItem),
    /// String text segment.
    StringPart(&'a StringPart),
    /// Match arm.
    MatchArm(&'a MatchArm),
}
/// A node and its depth, where the starting node has depth one.
#[derive(Clone, Copy, Debug)]
pub struct Entry<'a> {
    /// Borrowed node.
    pub node: Node<'a>,
    /// One-based depth.
    pub depth: usize,
}
/// Explicit-stack preorder iterator. No recursive visitor calls are used.
pub struct Walk<'a> {
    pending: Vec<Entry<'a>>,
}
/// Traverse a program in source order.
#[must_use]
pub fn walk(program: &Program) -> Walk<'_> {
    walk_node(Node::Program(program))
}
pub(crate) fn walk_node(node: Node<'_>) -> Walk<'_> {
    Walk {
        pending: vec![Entry { node, depth: 1 }],
    }
}
impl<'a> Iterator for Walk<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.pending.pop()?;
        self.pending
            .extend(children(entry.node).into_iter().rev().map(|node| Entry {
                node,
                depth: entry.depth + 1,
            }));
        Some(entry)
    }
}
fn target<'a>(value: &'a Target, out: &mut Vec<Node<'a>>) {
    out.push(match value {
        Target::Name(name) => Node::Name(name),
        Target::Expr(expr) => Node::Expr(expr),
    });
}
fn member<'a>(value: &'a Member, out: &mut Vec<Node<'a>>) {
    out.push(match value {
        Member::Identifier(name) => Node::Identifier(name),
        Member::Expr(expr) => Node::Expr(expr),
    });
}
fn parts<'a>(values: &'a [StringPart], out: &mut Vec<Node<'a>>) {
    out.extend(values.iter().map(|part| match part {
        StringPart::Expr(expr) => Node::Expr(expr),
        StringPart::Text { .. } => Node::StringPart(part),
    }));
}
#[allow(
    clippy::too_many_lines,
    reason = "exhaustive child ordering for the typed AST"
)]
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    match node {
        Node::Program(program) => out.extend(program.statements.iter().map(Node::Stmt)),
        Node::Stmt(stmt) => match &stmt.kind {
            StmtKind::Expression(expr) => out.push(Node::Expr(expr)),
            StmtKind::Echo(exprs) => out.extend(exprs.iter().map(Node::Expr)),
            StmtKind::Return(expr) => out.extend(expr.iter().map(Node::Expr)),
            StmtKind::Block(stmts) => out.extend(stmts.iter().map(Node::Stmt)),
            StmtKind::Nop | StmtKind::InlineHtml(_) | StmtKind::HaltCompiler(_) => {}
        },
        Node::Expr(expr) => match &expr.kind {
            ExprKind::Variable(name) => out.push(Node::Identifier(name)),
            ExprKind::ConstFetch(name) => out.push(Node::Name(name)),
            ExprKind::DynamicVariable(expr)
            | ExprKind::Unary { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Include { expr, .. } => out.push(Node::Expr(expr)),
            ExprKind::Assign { target, value }
            | ExprKind::AssignRef { target, value }
            | ExprKind::AssignOp { target, value, .. } => {
                out.push(Node::Expr(target));
                out.push(Node::Expr(value));
            }
            ExprKind::Binary { left, right, .. } => {
                out.push(Node::Expr(left));
                out.push(Node::Expr(right));
            }
            ExprKind::Ternary {
                condition,
                then,
                otherwise,
            } => {
                out.push(Node::Expr(condition));
                out.extend(then.iter().map(|e| Node::Expr(e)));
                out.push(Node::Expr(otherwise));
            }
            ExprKind::Array { items, .. } | ExprKind::List { items, .. } => {
                out.extend(items.iter().flatten().map(Node::ArrayItem));
            }
            ExprKind::ArrayDim { array, dim } => {
                out.push(Node::Expr(array));
                out.extend(dim.iter().map(|e| Node::Expr(e)));
            }
            ExprKind::Call {
                target: callee,
                args,
            } => {
                target(callee, &mut out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::Property { object, name, .. } => {
                out.push(Node::Expr(object));
                member(name, &mut out);
            }
            ExprKind::MethodCall {
                object, name, args, ..
            } => {
                out.push(Node::Expr(object));
                member(name, &mut out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::StaticProperty { class, name } | ExprKind::ClassConst { class, name } => {
                target(class, &mut out);
                member(name, &mut out);
            }
            ExprKind::StaticCall { class, name, args } => {
                target(class, &mut out);
                member(name, &mut out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::New { class, args } => {
                target(class, &mut out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::Instanceof { expr, class } => {
                out.push(Node::Expr(expr));
                target(class, &mut out);
            }
            ExprKind::Isset(exprs) => out.extend(exprs.iter().map(Node::Expr)),
            ExprKind::Exit(expr) => out.extend(expr.iter().map(|e| Node::Expr(e))),
            ExprKind::Yield { key, value } => {
                out.extend(key.iter().map(|e| Node::Expr(e)));
                out.extend(value.iter().map(|e| Node::Expr(e)));
            }
            ExprKind::Match { condition, arms } => {
                out.push(Node::Expr(condition));
                out.extend(arms.iter().map(Node::MatchArm));
            }
            ExprKind::Interpolated { parts: values, .. } | ExprKind::ShellExec(values) => {
                parts(values, &mut out);
            }
            ExprKind::Integer { .. }
            | ExprKind::Float(_)
            | ExprKind::String { .. }
            | ExprKind::MagicConstant(_)
            | ExprKind::Error => {}
        },
        Node::Argument(arg) => {
            out.extend(arg.name.iter().map(Node::Identifier));
            out.extend(arg.value.iter().map(Node::Expr));
        }
        Node::ArrayItem(item) => {
            out.extend(item.key.iter().map(Node::Expr));
            out.push(Node::Expr(&item.value));
        }
        Node::MatchArm(arm) => {
            if let Some(conditions) = &arm.conditions {
                out.extend(conditions.iter().map(Node::Expr));
            }
            out.push(Node::Expr(&arm.body));
        }
        Node::Name(_) | Node::Identifier(_) | Node::StringPart(_) => {}
    }
    out
}
