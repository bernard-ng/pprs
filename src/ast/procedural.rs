//! Callable signatures and procedural statement payloads.
use super::{Expr, Identifier, Name, NodeMeta, Stmt};

/// A syntactic PHP type, without compiler-level type resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Named class or a spelling not builtin in the selected target.
    Name(Name),
    /// Target-recognized builtin type, canonicalized to lowercase.
    Builtin(Identifier),
    /// Nullable type.
    Nullable {
        /// Source origin.
        meta: NodeMeta,
        /// Inner type.
        inner: Box<Type>,
    },
    /// Union of named types or parenthesized intersections.
    Union {
        /// Source origin.
        meta: NodeMeta,
        /// Members in source order.
        types: Vec<Type>,
    },
    /// Intersection of named types.
    Intersection {
        /// Source origin.
        meta: NodeMeta,
        /// Members in source order.
        types: Vec<Type>,
    },
}
impl Type {
    /// Source origin for this type node.
    #[must_use]
    pub fn meta(&self) -> &NodeMeta {
        match self {
            Self::Name(n) => n.meta(),
            Self::Builtin(n) => n.meta(),
            Self::Nullable { meta, .. }
            | Self::Union { meta, .. }
            | Self::Intersection { meta, .. } => meta,
        }
    }
}
/// Function, closure or arrow-function signature. This is not an additional AST node.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    /// Return by reference.
    pub by_ref: bool,
    /// Ordered parameters.
    pub params: Vec<Parameter>,
    /// Optional declared return type.
    pub return_type: Option<Type>,
}
/// A parameter, including an explicit error expression when its variable is missing.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// Source origin.
    pub meta: NodeMeta,
    /// Simple variable or recovery hole.
    pub variable: Box<Expr>,
    /// Optional declared type.
    pub ty: Option<Type>,
    /// Pass by reference.
    pub by_ref: bool,
    /// Variadic parameter.
    pub variadic: bool,
    /// Optional default value.
    pub default: Option<Box<Expr>>,
}
/// Named function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Declared name.
    pub name: Identifier,
    /// Signature.
    pub signature: Signature,
    /// Body statements.
    pub body: Vec<Stmt>,
}
/// Captured closure variable.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureUse {
    /// Source origin.
    pub meta: NodeMeta,
    /// Captured variable.
    pub variable: Box<Expr>,
    /// Capture by reference.
    pub by_ref: bool,
}
/// Anonymous function expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    /// Static closure.
    pub is_static: bool,
    /// Signature.
    pub signature: Signature,
    /// Explicit captures.
    pub uses: Vec<ClosureUse>,
    /// Body statements.
    pub body: Vec<Stmt>,
}
/// Arrow-function expression with implicit captures.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowFunction {
    /// Static arrow function.
    pub is_static: bool,
    /// Signature.
    pub signature: Signature,
    /// Expression body.
    pub expr: Box<Expr>,
}
/// An if statement and its ordered branches.
#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    /// Initial condition.
    pub condition: Box<Expr>,
    /// Initial body.
    pub body: Vec<Stmt>,
    /// Conditional branches.
    pub elseifs: Vec<ElseIf>,
    /// Optional fallback branch.
    pub otherwise: Option<Else>,
}
/// Else-if branch.
#[derive(Debug, Clone, PartialEq)]
pub struct ElseIf {
    /// Source origin.
    pub meta: NodeMeta,
    /// Condition.
    pub condition: Box<Expr>,
    /// Body.
    pub body: Vec<Stmt>,
}
/// Else body, retaining its own origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Else {
    /// Source origin.
    pub meta: NodeMeta,
    /// Body.
    pub body: Vec<Stmt>,
}
/// While or do-while loop.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalLoop {
    /// Condition.
    pub condition: Box<Expr>,
    /// Body.
    pub body: Vec<Stmt>,
}
/// For loop with independently optional expression lists.
#[derive(Debug, Clone, PartialEq)]
pub struct ForStatement {
    /// Initialization expressions.
    pub init: Vec<Expr>,
    /// Condition expressions.
    pub conditions: Vec<Expr>,
    /// Iteration expressions.
    pub steps: Vec<Expr>,
    /// Body.
    pub body: Vec<Stmt>,
}
/// Foreach loop and its variable/destructuring targets.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeachStatement {
    /// Iterable expression.
    pub iterable: Box<Expr>,
    /// Optional key target.
    pub key: Option<Box<Expr>>,
    /// Value target.
    pub value: Box<Expr>,
    /// Bind the value by reference.
    pub by_ref: bool,
    /// Body.
    pub body: Vec<Stmt>,
}
/// Switch statement.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStatement {
    /// Switched expression.
    pub condition: Box<Expr>,
    /// Ordered cases, including defaults.
    pub cases: Vec<Case>,
}
/// Switch case.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    /// Source origin.
    pub meta: NodeMeta,
    /// Absent for default.
    pub condition: Option<Box<Expr>>,
    /// Body.
    pub body: Vec<Stmt>,
}
/// Exception-handling statement.
#[derive(Debug, Clone, PartialEq)]
pub struct TryStatement {
    /// Protected body.
    pub body: Vec<Stmt>,
    /// Catch clauses.
    pub catches: Vec<Catch>,
    /// Optional cleanup clause.
    pub finally: Option<Finally>,
}
/// Catch clause with a name union and optional variable.
#[derive(Debug, Clone, PartialEq)]
pub struct Catch {
    /// Source origin.
    pub meta: NodeMeta,
    /// Exception types.
    pub types: Vec<Name>,
    /// Optional simple variable.
    pub variable: Option<Box<Expr>>,
    /// Handler body.
    pub body: Vec<Stmt>,
}
/// Finally clause.
#[derive(Debug, Clone, PartialEq)]
pub struct Finally {
    /// Source origin.
    pub meta: NodeMeta,
    /// Cleanup body.
    pub body: Vec<Stmt>,
}
/// Static local variable declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticVariable {
    /// Source origin.
    pub meta: NodeMeta,
    /// Declared variable.
    pub variable: Box<Expr>,
    /// Optional initializer.
    pub default: Option<Box<Expr>>,
}
/// A declare directive.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareItem {
    /// Source origin.
    pub meta: NodeMeta,
    /// Directive name.
    pub key: Identifier,
    /// Directive value.
    pub value: Box<Expr>,
}
/// A declare statement. No body differs from an empty body.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareStatement {
    /// Ordered directives.
    pub items: Vec<DeclareItem>,
    /// Optional scoped body.
    pub body: Option<Vec<Stmt>>,
}
