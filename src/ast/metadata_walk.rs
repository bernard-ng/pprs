//! Private, iterative metadata normalization over all owned syntax.
use crate::ast::{
    AnonymousClass, Argument, ArrayItem, Attribute, AttributeGroup, Case, Catch, ClosureUse,
    Constant, DeclareItem, Else, ElseIf, Expr, ExprKind, Finally, HookBody, Identifier, MatchArm,
    Member, Name, Parameter, Program, PropertyHook, PropertyItem, Signature, StaticVariable, Stmt,
    StmtKind, StringPart, Target, TraitAdaptation, Type, UseItem,
};
enum Node<'a> {
    /// Anonymous class declaration.
    AnonymousClass(&'a mut AnonymousClass),
    /// Attribute invocation.
    Attribute(&'a mut Attribute),
    /// Attribute group.
    AttributeGroup(&'a mut AttributeGroup),
    /// Constant initializer.
    Constant(&'a mut Constant),
    /// Import item.
    UseItem(&'a mut UseItem),
    /// Property item.
    PropertyItem(&'a mut PropertyItem),
    /// Property hook.
    PropertyHook(&'a mut PropertyHook),
    /// Trait adaptation.
    TraitAdaptation(&'a mut TraitAdaptation),
    /// File root.
    Program(&'a mut Program),
    /// Statement.
    Stmt(&'a mut Stmt),
    /// Expression.
    Expr(&'a mut Expr),
    /// Name.
    Name(&'a mut Name),
    /// Identifier.
    Identifier(&'a mut Identifier),
    /// Argument/placeholder.
    Argument(&'a mut Argument),
    /// Array item.
    ArrayItem(&'a mut ArrayItem),
    /// String text segment.
    StringPart(&'a mut StringPart),
    /// Composite type.
    Type(&'a mut Type),
    /// Parameter.
    Parameter(&'a mut Parameter),
    /// Captured variable.
    ClosureUse(&'a mut ClosureUse),
    /// Conditional branch.
    ElseIf(&'a mut ElseIf),
    /// Fallback branch.
    Else(&'a mut Else),
    /// Switch case.
    Case(&'a mut Case),
    /// Catch clause.
    Catch(&'a mut Catch),
    /// Finally clause.
    Finally(&'a mut Finally),
    /// Static local declaration.
    StaticVariable(&'a mut StaticVariable),
    /// Declare directive.
    DeclareItem(&'a mut DeclareItem),
    /// Match arm.
    MatchArm(&'a mut MatchArm),
}
fn target<'a>(value: &'a mut Target, out: &mut Vec<Node<'a>>) {
    out.push(match value {
        Target::Name(name) => Node::Name(name),
        Target::Expr(expr) => Node::Expr(expr),
    });
}
fn member<'a>(value: &'a mut Member, out: &mut Vec<Node<'a>>) {
    out.push(match value {
        Member::Identifier(name) => Node::Identifier(name),
        Member::Expr(expr) => Node::Expr(expr),
    });
}
fn parts<'a>(values: &'a mut [StringPart], out: &mut Vec<Node<'a>>) {
    out.extend(values.iter_mut().map(|part| match part {
        StringPart::Expr(expr) => Node::Expr(expr),
        StringPart::Text { .. } => Node::StringPart(part),
    }));
}
fn type_node(ty: &mut Type) -> Node<'_> {
    match ty {
        Type::Name(n) => Node::Name(n),
        Type::Builtin(n) => Node::Identifier(n),
        _ => Node::Type(ty),
    }
}
fn signature<'a>(value: &'a mut Signature, out: &mut Vec<Node<'a>>) {
    out.extend(value.params.iter_mut().map(Node::Parameter));
    out.extend(value.return_type.iter_mut().map(type_node));
}
fn attributes<'a>(values: &'a mut [AttributeGroup], out: &mut Vec<Node<'a>>) {
    out.extend(values.iter_mut().map(Node::AttributeGroup));
}
#[allow(
    clippy::too_many_lines,
    reason = "exhaustive child ordering for the typed AST"
)]
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    match node {
        Node::Program(program) => out.extend(program.statements.iter_mut().map(Node::Stmt)),
        Node::Stmt(stmt) => match &mut stmt.kind {
            StmtKind::Namespace(ns) => {
                out.extend(ns.name.iter_mut().map(Node::Name));
                if let Some(body) = &mut ns.body {
                    out.extend(body.iter_mut().map(Node::Stmt));
                }
            }
            StmtKind::Use(u) => {
                out.extend(u.prefix.iter_mut().map(Node::Name));
                out.extend(u.items.iter_mut().map(Node::UseItem));
            }
            StmtKind::Constants(c) | StmtKind::ClassConstants(c) => {
                attributes(&mut c.attributes, &mut out);
                out.extend(c.ty.iter_mut().map(type_node));
                out.extend(c.constants.iter_mut().map(Node::Constant));
            }
            StmtKind::Class(c) => {
                attributes(&mut c.attributes, &mut out);
                out.extend(c.name.iter_mut().map(Node::Identifier));
                out.extend(c.scalar_type.iter_mut().map(type_node));
                out.extend(c.extends.iter_mut().map(Node::Name));
                out.extend(c.implements.iter_mut().map(Node::Name));
                out.extend(c.members.iter_mut().map(Node::Stmt));
            }
            StmtKind::Method(m) => {
                attributes(&mut m.attributes, &mut out);
                out.push(Node::Identifier(&mut m.name));
                signature(&mut m.signature, &mut out);
                if let Some(body) = &mut m.body {
                    out.extend(body.iter_mut().map(Node::Stmt));
                }
            }
            StmtKind::Property(p) => {
                attributes(&mut p.attributes, &mut out);
                out.extend(p.ty.iter_mut().map(type_node));
                out.extend(p.items.iter_mut().map(Node::PropertyItem));
                out.extend(p.hooks.iter_mut().map(Node::PropertyHook));
            }
            StmtKind::TraitUse(t) => {
                out.extend(t.traits.iter_mut().map(Node::Name));
                out.extend(t.adaptations.iter_mut().map(Node::TraitAdaptation));
            }
            StmtKind::EnumCase(c) => {
                attributes(&mut c.attributes, &mut out);
                out.push(Node::Identifier(&mut c.name));
                out.extend(c.value.iter_mut().map(|e| Node::Expr(e)));
            }

            StmtKind::Expression(expr) => out.push(Node::Expr(expr)),
            StmtKind::Echo(exprs) => out.extend(exprs.iter_mut().map(Node::Expr)),
            StmtKind::Return(expr) => out.extend(expr.iter_mut().map(Node::Expr)),
            StmtKind::Block(stmts) => out.extend(stmts.iter_mut().map(Node::Stmt)),
            StmtKind::Function(f) => {
                attributes(&mut f.attributes, &mut out);
                out.push(Node::Identifier(&mut f.name));
                signature(&mut f.signature, &mut out);
                out.extend(f.body.iter_mut().map(Node::Stmt));
            }
            StmtKind::If(s) => {
                out.push(Node::Expr(&mut s.condition));
                out.extend(s.body.iter_mut().map(Node::Stmt));
                out.extend(s.elseifs.iter_mut().map(Node::ElseIf));
                out.extend(s.otherwise.iter_mut().map(Node::Else));
            }
            StmtKind::While(s) => {
                out.push(Node::Expr(&mut s.condition));
                out.extend(s.body.iter_mut().map(Node::Stmt));
            }
            StmtKind::Do(s) => {
                out.extend(s.body.iter_mut().map(Node::Stmt));
                out.push(Node::Expr(&mut s.condition));
            }
            StmtKind::For(s) => {
                out.extend(s.init.iter_mut().map(Node::Expr));
                out.extend(s.conditions.iter_mut().map(Node::Expr));
                out.extend(s.steps.iter_mut().map(Node::Expr));
                out.extend(s.body.iter_mut().map(Node::Stmt));
            }
            StmtKind::Foreach(s) => {
                out.push(Node::Expr(&mut s.iterable));
                out.extend(s.key.iter_mut().map(|e| Node::Expr(e)));
                out.push(Node::Expr(&mut s.value));
                out.extend(s.body.iter_mut().map(Node::Stmt));
            }
            StmtKind::Switch(s) => {
                out.push(Node::Expr(&mut s.condition));
                out.extend(s.cases.iter_mut().map(Node::Case));
            }
            StmtKind::Try(s) => {
                out.extend(s.body.iter_mut().map(Node::Stmt));
                out.extend(s.catches.iter_mut().map(Node::Catch));
                out.extend(s.finally.iter_mut().map(Node::Finally));
            }
            StmtKind::Declare(s) => {
                out.extend(s.items.iter_mut().map(Node::DeclareItem));
                if let Some(body) = &mut s.body {
                    out.extend(body.iter_mut().map(Node::Stmt));
                }
            }
            StmtKind::Break(expr) | StmtKind::Continue(expr) => {
                out.extend(expr.iter_mut().map(Node::Expr));
            }
            StmtKind::Global(exprs) | StmtKind::Unset(exprs) => {
                out.extend(exprs.iter_mut().map(Node::Expr));
            }
            StmtKind::Static(vars) => out.extend(vars.iter_mut().map(Node::StaticVariable)),
            StmtKind::Goto(id) | StmtKind::Label(id) => out.push(Node::Identifier(id)),
            StmtKind::Nop | StmtKind::InlineHtml(_) | StmtKind::HaltCompiler(_) => {}
        },
        Node::Expr(expr) => match &mut expr.kind {
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
                out.extend(then.iter_mut().map(|e| Node::Expr(e)));
                out.push(Node::Expr(otherwise));
            }
            ExprKind::Array { items, .. } | ExprKind::List { items, .. } => {
                out.extend(items.iter_mut().flatten().map(Node::ArrayItem));
            }
            ExprKind::ArrayDim { array, dim } => {
                out.push(Node::Expr(array));
                out.extend(dim.iter_mut().map(|e| Node::Expr(e)));
            }
            ExprKind::Call {
                target: callee,
                args,
            } => {
                target(callee, &mut out);
                out.extend(args.iter_mut().map(Node::Argument));
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
                out.extend(args.iter_mut().map(Node::Argument));
            }
            ExprKind::StaticProperty { class, name } | ExprKind::ClassConst { class, name } => {
                target(class, &mut out);
                member(name, &mut out);
            }
            ExprKind::StaticCall { class, name, args } => {
                target(class, &mut out);
                member(name, &mut out);
                out.extend(args.iter_mut().map(Node::Argument));
            }
            ExprKind::New { class, args } => {
                target(class, &mut out);
                out.extend(args.iter_mut().map(Node::Argument));
            }
            ExprKind::Instanceof { expr, class } => {
                out.push(Node::Expr(expr));
                target(class, &mut out);
            }
            ExprKind::Isset(exprs) => out.extend(exprs.iter_mut().map(Node::Expr)),
            ExprKind::Exit(expr) => out.extend(expr.iter_mut().map(|e| Node::Expr(e))),
            ExprKind::Yield { key, value } => {
                out.extend(key.iter_mut().map(|e| Node::Expr(e)));
                out.extend(value.iter_mut().map(|e| Node::Expr(e)));
            }
            ExprKind::Match { condition, arms } => {
                out.push(Node::Expr(condition));
                out.extend(arms.iter_mut().map(Node::MatchArm));
            }
            ExprKind::Interpolated { parts: values, .. } | ExprKind::ShellExec(values) => {
                parts(values, &mut out);
            }
            ExprKind::Closure(c) => {
                attributes(&mut c.attributes, &mut out);
                out.extend(c.signature.params.iter_mut().map(Node::Parameter));
                out.extend(c.uses.iter_mut().map(Node::ClosureUse));
                out.extend(c.signature.return_type.iter_mut().map(type_node));
                out.extend(c.body.iter_mut().map(Node::Stmt));
            }
            ExprKind::ArrowFunction(f) => {
                attributes(&mut f.attributes, &mut out);
                signature(&mut f.signature, &mut out);
                out.push(Node::Expr(&mut f.expr));
            }
            ExprKind::Integer { .. }
            | ExprKind::Float(_)
            | ExprKind::String { .. }
            | ExprKind::MagicConstant(_)
            | ExprKind::Error => {}
        },
        Node::AnonymousClass(c) => {
            attributes(&mut c.declaration.attributes, &mut out);
            out.extend(c.declaration.name.iter_mut().map(Node::Identifier));
            out.extend(c.args.iter_mut().map(Node::Argument));
            out.extend(c.declaration.scalar_type.iter_mut().map(type_node));
            out.extend(c.declaration.extends.iter_mut().map(Node::Name));
            out.extend(c.declaration.implements.iter_mut().map(Node::Name));
            out.extend(c.declaration.members.iter_mut().map(Node::Stmt));
        }
        Node::AttributeGroup(g) => out.extend(g.attributes.iter_mut().map(Node::Attribute)),
        Node::Attribute(a) => {
            out.push(Node::Name(&mut a.name));
            out.extend(a.args.iter_mut().map(Node::Argument));
        }
        Node::Constant(c) => {
            out.push(Node::Identifier(&mut c.name));
            out.push(Node::Expr(&mut c.value));
        }
        Node::UseItem(u) => {
            out.push(Node::Name(&mut u.name));
            out.extend(u.alias.iter_mut().map(Node::Identifier));
        }
        Node::PropertyItem(p) => {
            out.push(Node::Identifier(&mut p.name));
            out.extend(p.default.iter_mut().map(|e| Node::Expr(e)));
        }
        Node::PropertyHook(h) => {
            attributes(&mut h.attributes, &mut out);
            out.push(Node::Identifier(&mut h.name));
            out.extend(h.params.iter_mut().map(Node::Parameter));
            match &mut h.body {
                HookBody::Abstract => {}
                HookBody::Block(stmts) => out.extend(stmts.iter_mut().map(Node::Stmt)),
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
                out.extend(trait_name.iter_mut().map(Node::Name));
                out.push(Node::Identifier(method));
                out.extend(alias.iter_mut().map(Node::Identifier));
            }
            TraitAdaptation::Precedence {
                trait_name,
                method,
                instead_of,
                ..
            } => {
                out.push(Node::Name(trait_name));
                out.push(Node::Identifier(method));
                out.extend(instead_of.iter_mut().map(Node::Name));
            }
        },
        Node::Argument(arg) => {
            out.extend(arg.name.iter_mut().map(Node::Identifier));
            out.extend(arg.value.iter_mut().map(Node::Expr));
        }
        Node::ArrayItem(item) => {
            out.extend(item.key.iter_mut().map(Node::Expr));
            out.push(Node::Expr(&mut item.value));
        }
        Node::MatchArm(arm) => {
            if let Some(conditions) = &mut arm.conditions {
                out.extend(conditions.iter_mut().map(Node::Expr));
            }
            out.push(Node::Expr(&mut arm.body));
        }
        Node::Type(ty) => match ty {
            Type::Nullable { inner, .. } => out.push(type_node(inner)),
            Type::Union { types, .. } | Type::Intersection { types, .. } => {
                out.extend(types.iter_mut().map(type_node));
            }
            Type::Name(n) => out.push(Node::Name(n)),
            Type::Builtin(n) => out.push(Node::Identifier(n)),
        },
        Node::Parameter(p) => {
            attributes(&mut p.attributes, &mut out);
            out.extend(p.ty.iter_mut().map(type_node));
            out.push(Node::Expr(&mut p.variable));
            out.extend(p.default.iter_mut().map(|e| Node::Expr(e)));
            out.extend(p.hooks.iter_mut().map(Node::PropertyHook));
        }
        Node::ClosureUse(u) => out.push(Node::Expr(&mut u.variable)),
        Node::ElseIf(s) => {
            out.push(Node::Expr(&mut s.condition));
            out.extend(s.body.iter_mut().map(Node::Stmt));
        }
        Node::Else(s) => out.extend(s.body.iter_mut().map(Node::Stmt)),
        Node::Case(s) => {
            out.extend(s.condition.iter_mut().map(|e| Node::Expr(e)));
            out.extend(s.body.iter_mut().map(Node::Stmt));
        }
        Node::Catch(s) => {
            out.extend(s.types.iter_mut().map(Node::Name));
            out.extend(s.variable.iter_mut().map(|e| Node::Expr(e)));
            out.extend(s.body.iter_mut().map(Node::Stmt));
        }
        Node::Finally(s) => out.extend(s.body.iter_mut().map(Node::Stmt)),
        Node::StaticVariable(s) => {
            out.push(Node::Expr(&mut s.variable));
            out.extend(s.default.iter_mut().map(|e| Node::Expr(e)));
        }
        Node::DeclareItem(s) => {
            out.push(Node::Identifier(&mut s.key));
            out.push(Node::Expr(&mut s.value));
        }
        Node::Name(_) | Node::Identifier(_) | Node::StringPart(_) => {}
    }
    out
}

pub(super) fn metadata_mut(program: &mut Program, mut visit: impl FnMut(&mut super::NodeMeta)) {
    let mut pending = vec![Node::Program(program)];
    while let Some(mut node) = pending.pop() {
        let meta = match &mut node {
            Node::AnonymousClass(n) => Some(&mut n.meta),
            Node::Attribute(n) => Some(&mut n.meta),
            Node::AttributeGroup(n) => Some(&mut n.meta),
            Node::Constant(n) => Some(&mut n.meta),
            Node::UseItem(n) => Some(&mut n.meta),
            Node::PropertyItem(n) => Some(&mut n.meta),
            Node::PropertyHook(n) => Some(&mut n.meta),
            Node::TraitAdaptation(n) => match n {
                TraitAdaptation::Alias { meta, .. } | TraitAdaptation::Precedence { meta, .. } => {
                    Some(meta)
                }
            },
            Node::Program(_) => None,
            Node::Stmt(n) => Some(&mut n.meta),
            Node::Expr(n) => Some(&mut n.meta),
            Node::Name(n) => Some(&mut n.meta),
            Node::Identifier(n) => Some(&mut n.meta),
            Node::Argument(n) => Some(&mut n.meta),
            Node::ArrayItem(n) => Some(&mut n.meta),
            Node::StringPart(n) => match n {
                StringPart::Text { meta, .. } => Some(meta),
                StringPart::Expr(n) => Some(&mut n.meta),
            },
            Node::Type(n) => match n {
                Type::Nullable { meta, .. }
                | Type::Union { meta, .. }
                | Type::Intersection { meta, .. } => Some(meta),
                Type::Name(n) => Some(&mut n.meta),
                Type::Builtin(n) => Some(&mut n.meta),
            },
            Node::Parameter(n) => Some(&mut n.meta),
            Node::ClosureUse(n) => Some(&mut n.meta),
            Node::ElseIf(n) => Some(&mut n.meta),
            Node::Else(n) => Some(&mut n.meta),
            Node::Case(n) => Some(&mut n.meta),
            Node::Catch(n) => Some(&mut n.meta),
            Node::Finally(n) => Some(&mut n.meta),
            Node::StaticVariable(n) => Some(&mut n.meta),
            Node::DeclareItem(n) => Some(&mut n.meta),
            Node::MatchArm(n) => Some(&mut n.meta),
        };
        if let Some(meta) = meta {
            visit(meta);
        }
        pending.extend(children(node).into_iter().rev());
    }
}
