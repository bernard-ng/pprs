//! Read-only, iterative traversal in source order, including auxiliary AST nodes.
use crate::ast::{
    AnonymousClass, Argument, ArrayItem, Attribute, AttributeGroup, Case, Catch, ClassDeclaration,
    ClosureUse, Constant, DeclareItem, Else, ElseIf, Expr, ExprKind, Finally, HookBody, Identifier,
    MatchArm, Member, Name, Parameter, Program, PropertyHook, PropertyItem, Signature,
    StaticVariable, Stmt, StmtKind, StringPart, Target, TraitAdaptation, Type, UseItem,
};
/// A borrowed AST node; traversal does not mutate metadata or semantic values.
#[derive(Clone, Copy, Debug)]
pub enum Node<'a> {
    /// Anonymous class declaration.
    AnonymousClass(&'a AnonymousClass),
    /// Attribute invocation.
    Attribute(&'a Attribute),
    /// Attribute group.
    AttributeGroup(&'a AttributeGroup),
    /// Constant initializer.
    Constant(&'a Constant),
    /// Import item.
    UseItem(&'a UseItem),
    /// Property item.
    PropertyItem(&'a PropertyItem),
    /// Property hook.
    PropertyHook(&'a PropertyHook),
    /// Trait adaptation.
    TraitAdaptation(&'a TraitAdaptation),
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
    /// Composite type.
    Type(&'a Type),
    /// Parameter.
    Parameter(&'a Parameter),
    /// Captured variable.
    ClosureUse(&'a ClosureUse),
    /// Conditional branch.
    ElseIf(&'a ElseIf),
    /// Fallback branch.
    Else(&'a Else),
    /// Switch case.
    Case(&'a Case),
    /// Catch clause.
    Catch(&'a Catch),
    /// Finally clause.
    Finally(&'a Finally),
    /// Static local declaration.
    StaticVariable(&'a StaticVariable),
    /// Declare directive.
    DeclareItem(&'a DeclareItem),
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
    children: Vec<Node<'a>>,
}
/// Traverse a program in source order.
#[must_use]
pub fn walk(program: &Program) -> Walk<'_> {
    walk_node(Node::Program(program))
}
pub(crate) fn walk_node(node: Node<'_>) -> Walk<'_> {
    Walk {
        pending: vec![Entry { node, depth: 1 }],
        children: Vec::new(),
    }
}
impl<'a> Iterator for Walk<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.pending.pop()?;
        children(entry.node, &mut self.children);
        self.pending
            .extend(self.children.drain(..).rev().map(|node| Entry {
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
fn type_node(ty: &Type) -> Node<'_> {
    match ty {
        Type::Name(n) => Node::Name(n),
        Type::Builtin(n) => Node::Identifier(n),
        _ => Node::Type(ty),
    }
}
fn signature<'a>(value: &'a Signature, out: &mut Vec<Node<'a>>) {
    out.extend(value.params.iter().map(Node::Parameter));
    out.extend(value.return_type.iter().map(type_node));
}
fn attributes<'a>(values: &'a [AttributeGroup], out: &mut Vec<Node<'a>>) {
    out.extend(values.iter().map(Node::AttributeGroup));
}
fn class_header<'a>(value: &'a ClassDeclaration, out: &mut Vec<Node<'a>>) {
    attributes(&value.attributes, out);
    out.extend(value.name.iter().map(Node::Identifier));
}
fn class_body<'a>(value: &'a ClassDeclaration, out: &mut Vec<Node<'a>>) {
    out.extend(value.scalar_type.iter().map(type_node));
    out.extend(value.extends.iter().map(Node::Name));
    out.extend(value.implements.iter().map(Node::Name));
    out.extend(value.members.iter().map(Node::Stmt));
}
#[allow(
    clippy::too_many_lines,
    reason = "exhaustive child ordering for the typed AST"
)]
fn children<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    debug_assert!(out.is_empty(), "walker drains its reusable child buffer");
    match node {
        Node::Program(program) => out.extend(program.statements.iter().map(Node::Stmt)),
        Node::Stmt(stmt) => match &stmt.kind {
            StmtKind::Namespace(ns) => {
                out.extend(ns.name.iter().map(Node::Name));
                if let Some(body) = &ns.body {
                    out.extend(body.iter().map(Node::Stmt));
                }
            }
            StmtKind::Use(u) => {
                out.extend(u.prefix.iter().map(Node::Name));
                out.extend(u.items.iter().map(Node::UseItem));
            }
            StmtKind::Constants(c) | StmtKind::ClassConstants(c) => {
                attributes(&c.attributes, out);
                out.extend(c.ty.iter().map(type_node));
                out.extend(c.constants.iter().map(Node::Constant));
            }
            StmtKind::Class(c) => {
                class_header(c, out);
                class_body(c, out);
            }
            StmtKind::Method(m) => {
                attributes(&m.attributes, out);
                out.push(Node::Identifier(&m.name));
                signature(&m.signature, out);
                if let Some(body) = &m.body {
                    out.extend(body.iter().map(Node::Stmt));
                }
            }
            StmtKind::Property(p) => {
                attributes(&p.attributes, out);
                out.extend(p.ty.iter().map(type_node));
                out.extend(p.items.iter().map(Node::PropertyItem));
                out.extend(p.hooks.iter().map(Node::PropertyHook));
            }
            StmtKind::TraitUse(t) => {
                out.extend(t.traits.iter().map(Node::Name));
                out.extend(t.adaptations.iter().map(Node::TraitAdaptation));
            }
            StmtKind::EnumCase(c) => {
                attributes(&c.attributes, out);
                out.push(Node::Identifier(&c.name));
                out.extend(c.value.iter().map(|e| Node::Expr(e)));
            }

            StmtKind::Expression(expr) => out.push(Node::Expr(expr)),
            StmtKind::Echo(exprs) => out.extend(exprs.iter().map(Node::Expr)),
            StmtKind::Return(expr) => out.extend(expr.iter().map(Node::Expr)),
            StmtKind::Block(stmts) => out.extend(stmts.iter().map(Node::Stmt)),
            StmtKind::Function(f) => {
                attributes(&f.attributes, out);
                out.push(Node::Identifier(&f.name));
                signature(&f.signature, out);
                out.extend(f.body.iter().map(Node::Stmt));
            }
            StmtKind::If(s) => {
                out.push(Node::Expr(&s.condition));
                out.extend(s.body.iter().map(Node::Stmt));
                out.extend(s.elseifs.iter().map(Node::ElseIf));
                out.extend(s.otherwise.iter().map(Node::Else));
            }
            StmtKind::While(s) => {
                out.push(Node::Expr(&s.condition));
                out.extend(s.body.iter().map(Node::Stmt));
            }
            StmtKind::Do(s) => {
                out.extend(s.body.iter().map(Node::Stmt));
                out.push(Node::Expr(&s.condition));
            }
            StmtKind::For(s) => {
                out.extend(s.init.iter().map(Node::Expr));
                out.extend(s.conditions.iter().map(Node::Expr));
                out.extend(s.steps.iter().map(Node::Expr));
                out.extend(s.body.iter().map(Node::Stmt));
            }
            StmtKind::Foreach(s) => {
                out.push(Node::Expr(&s.iterable));
                out.extend(s.key.iter().map(|e| Node::Expr(e)));
                out.push(Node::Expr(&s.value));
                out.extend(s.body.iter().map(Node::Stmt));
            }
            StmtKind::Switch(s) => {
                out.push(Node::Expr(&s.condition));
                out.extend(s.cases.iter().map(Node::Case));
            }
            StmtKind::Try(s) => {
                out.extend(s.body.iter().map(Node::Stmt));
                out.extend(s.catches.iter().map(Node::Catch));
                out.extend(s.finally.iter().map(Node::Finally));
            }
            StmtKind::Declare(s) => {
                out.extend(s.items.iter().map(Node::DeclareItem));
                if let Some(body) = &s.body {
                    out.extend(body.iter().map(Node::Stmt));
                }
            }
            StmtKind::Break(expr) | StmtKind::Continue(expr) => {
                out.extend(expr.iter().map(Node::Expr));
            }
            StmtKind::Global(exprs) | StmtKind::Unset(exprs) => {
                out.extend(exprs.iter().map(Node::Expr));
            }
            StmtKind::Static(vars) => out.extend(vars.iter().map(Node::StaticVariable)),
            StmtKind::Goto(id) | StmtKind::Label(id) => out.push(Node::Identifier(id)),
            StmtKind::Nop | StmtKind::InlineHtml(_) | StmtKind::HaltCompiler(_) => {}
        },
        Node::Expr(expr) => match &expr.kind {
            ExprKind::NewAnonymous(c) => out.push(Node::AnonymousClass(c)),
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
                target(callee, out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::Property { object, name, .. } => {
                out.push(Node::Expr(object));
                member(name, out);
            }
            ExprKind::MethodCall {
                object, name, args, ..
            } => {
                out.push(Node::Expr(object));
                member(name, out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::StaticProperty { class, name } | ExprKind::ClassConst { class, name } => {
                target(class, out);
                member(name, out);
            }
            ExprKind::StaticCall { class, name, args } => {
                target(class, out);
                member(name, out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::New { class, args } => {
                target(class, out);
                out.extend(args.iter().map(Node::Argument));
            }
            ExprKind::Instanceof { expr, class } => {
                out.push(Node::Expr(expr));
                target(class, out);
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
                parts(values, out);
            }
            ExprKind::Closure(c) => {
                attributes(&c.attributes, out);
                out.extend(c.signature.params.iter().map(Node::Parameter));
                out.extend(c.uses.iter().map(Node::ClosureUse));
                out.extend(c.signature.return_type.iter().map(type_node));
                out.extend(c.body.iter().map(Node::Stmt));
            }
            ExprKind::ArrowFunction(f) => {
                attributes(&f.attributes, out);
                signature(&f.signature, out);
                out.push(Node::Expr(&f.expr));
            }
            ExprKind::Integer { .. }
            | ExprKind::Float(_)
            | ExprKind::String { .. }
            | ExprKind::MagicConstant(_)
            | ExprKind::Error => {}
        },
        Node::AnonymousClass(c) => {
            class_header(&c.declaration, out);
            out.extend(c.args.iter().map(Node::Argument));
            class_body(&c.declaration, out);
        }
        Node::AttributeGroup(g) => out.extend(g.attributes.iter().map(Node::Attribute)),
        Node::Attribute(a) => {
            out.push(Node::Name(&a.name));
            out.extend(a.args.iter().map(Node::Argument));
        }
        Node::Constant(c) => {
            out.push(Node::Identifier(&c.name));
            out.push(Node::Expr(&c.value));
        }
        Node::UseItem(u) => {
            out.push(Node::Name(&u.name));
            out.extend(u.alias.iter().map(Node::Identifier));
        }
        Node::PropertyItem(p) => {
            out.push(Node::Identifier(&p.name));
            out.extend(p.default.iter().map(|e| Node::Expr(e)));
        }
        Node::PropertyHook(h) => {
            attributes(&h.attributes, out);
            out.push(Node::Identifier(&h.name));
            out.extend(h.params.iter().map(Node::Parameter));
            match &h.body {
                HookBody::Abstract => {}
                HookBody::Block(stmts) => out.extend(stmts.iter().map(Node::Stmt)),
                HookBody::Expression(e) => out.push(Node::Expr(e)),
            }
        }
        Node::TraitAdaptation(a) => match a {
            TraitAdaptation::Alias {
                trait_name,
                method,
                alias,
                ..
            } => {
                out.extend(trait_name.iter().map(Node::Name));
                out.push(Node::Identifier(method));
                out.extend(alias.iter().map(Node::Identifier));
            }
            TraitAdaptation::Precedence {
                trait_name,
                method,
                instead_of,
                ..
            } => {
                out.push(Node::Name(trait_name));
                out.push(Node::Identifier(method));
                out.extend(instead_of.iter().map(Node::Name));
            }
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
        Node::Type(ty) => match ty {
            Type::Nullable { inner, .. } => out.push(type_node(inner)),
            Type::Union { types, .. } | Type::Intersection { types, .. } => {
                out.extend(types.iter().map(type_node));
            }
            Type::Name(n) => out.push(Node::Name(n)),
            Type::Builtin(n) => out.push(Node::Identifier(n)),
        },
        Node::Parameter(p) => {
            attributes(&p.attributes, out);
            out.extend(p.ty.iter().map(type_node));
            out.push(Node::Expr(&p.variable));
            out.extend(p.default.iter().map(|e| Node::Expr(e)));
            out.extend(p.hooks.iter().map(Node::PropertyHook));
        }
        Node::ClosureUse(u) => out.push(Node::Expr(&u.variable)),
        Node::ElseIf(s) => {
            out.push(Node::Expr(&s.condition));
            out.extend(s.body.iter().map(Node::Stmt));
        }
        Node::Else(s) => out.extend(s.body.iter().map(Node::Stmt)),
        Node::Case(s) => {
            out.extend(s.condition.iter().map(|e| Node::Expr(e)));
            out.extend(s.body.iter().map(Node::Stmt));
        }
        Node::Catch(s) => {
            out.extend(s.types.iter().map(Node::Name));
            out.extend(s.variable.iter().map(|e| Node::Expr(e)));
            out.extend(s.body.iter().map(Node::Stmt));
        }
        Node::Finally(s) => out.extend(s.body.iter().map(Node::Stmt)),
        Node::StaticVariable(s) => {
            out.push(Node::Expr(&s.variable));
            out.extend(s.default.iter().map(|e| Node::Expr(e)));
        }
        Node::DeclareItem(s) => {
            out.push(Node::Identifier(&s.key));
            out.push(Node::Expr(&s.value));
        }
        Node::Name(_) | Node::Identifier(_) | Node::StringPart(_) => {}
    }
}
