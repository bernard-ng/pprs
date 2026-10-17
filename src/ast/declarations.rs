//! Namespace, class-like and member syntax.
use super::{Argument, Expr, Identifier, Name, NodeMeta, Signature, Stmt, Type};

/// Syntactic modifier bits, including combinations retained during recovery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers(pub u16);
impl Modifiers {
    /// Public read visibility.
    pub const PUBLIC: Self = Self(1);
    /// Protected read visibility.
    pub const PROTECTED: Self = Self(2);
    /// Private read visibility.
    pub const PRIVATE: Self = Self(4);
    /// Static member.
    pub const STATIC: Self = Self(8);
    /// Abstract declaration.
    pub const ABSTRACT: Self = Self(16);
    /// Final declaration.
    pub const FINAL: Self = Self(32);
    /// Readonly declaration.
    pub const READONLY: Self = Self(64);
    /// Public write visibility.
    pub const PUBLIC_SET: Self = Self(128);
    /// Protected write visibility.
    pub const PROTECTED_SET: Self = Self(256);
    /// Private write visibility.
    pub const PRIVATE_SET: Self = Self(512);
    /// Whether all specified modifier bits are present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
/// One attribute invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// Source origin.
    pub meta: NodeMeta,
    /// Attribute class.
    pub name: Name,
    /// Unevaluated arguments.
    pub args: Vec<Argument>,
}
/// A bracketed group of attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeGroup {
    /// Source origin.
    pub meta: NodeMeta,
    /// Ordered attributes.
    pub attributes: Vec<Attribute>,
}
/// Namespace declaration style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceStyle {
    /// Braced body.
    Braced,
    /// Semicolon followed by implicitly grouped statements.
    Semicolon,
}
/// A namespace, preserving its syntax style.
#[derive(Debug, Clone, PartialEq)]
pub struct Namespace {
    /// Absent for an explicitly global namespace.
    pub name: Option<Name>,
    /// Syntax form.
    pub style: NamespaceStyle,
    /// Statements grouped into this namespace; absent only for an unnormalized
    /// semicolon namespace retained in malformed nested syntax.
    pub body: Option<Vec<Stmt>>,
}
/// Import category. Inherited is used for entries in a homogeneous import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    /// Inherit from the parent import.
    Inherited,
    /// Class or namespace.
    Normal,
    /// Function.
    Function,
    /// Constant.
    Constant,
}
/// One imported name.
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// Source origin.
    pub meta: NodeMeta,
    /// Name without a leading separator.
    pub name: Name,
    /// Optional alias.
    pub alias: Option<Identifier>,
    /// Import category.
    pub kind: UseKind,
}
/// Simple or grouped imports.
#[derive(Debug, Clone, PartialEq)]
pub struct UseStatement {
    /// Shared category; inherited for mixed groups.
    pub kind: UseKind,
    /// Present for grouped imports.
    pub prefix: Option<Name>,
    /// Ordered entries.
    pub items: Vec<UseItem>,
}
/// A constant name and initializer.
#[derive(Debug, Clone, PartialEq)]
pub struct Constant {
    /// Source origin.
    pub meta: NodeMeta,
    /// Declared identifier.
    pub name: Identifier,
    /// Unevaluated value.
    pub value: Box<Expr>,
}
/// Global constants, or class constants with optional modifiers and type.
#[derive(Debug, Clone, PartialEq)]
pub struct Constants {
    /// Attributes (global constant attributes remain outside the PHP 8.4 ceiling).
    pub attributes: Vec<AttributeGroup>,
    /// Explicit modifiers; implicit public remains zero.
    pub flags: Modifiers,
    /// Optional class constant type.
    pub ty: Option<Type>,
    /// Ordered declarations.
    pub constants: Vec<Constant>,
}
/// Class-like declaration category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    /// Class (named or anonymous).
    Class,
    /// Interface.
    Interface,
    /// Trait.
    Trait,
    /// Enum.
    Enum,
}
/// A class, interface, trait or enum body.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDeclaration {
    /// Declaration category.
    pub kind: ClassKind,
    /// Absent for anonymous classes.
    pub name: Option<Identifier>,
    /// Explicit modifiers.
    pub flags: Modifiers,
    /// Attribute groups.
    pub attributes: Vec<AttributeGroup>,
    /// Parent class or extended interfaces.
    pub extends: Vec<Name>,
    /// Implemented interfaces.
    pub implements: Vec<Name>,
    /// Optional enum backing type.
    pub scalar_type: Option<Type>,
    /// Ordered members, including comment no-ops.
    pub members: Vec<Stmt>,
}
/// Anonymous class construction, with arguments between the class header and body.
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousClass {
    /// Class origin (the enclosing expression includes `new`).
    pub meta: NodeMeta,
    /// Class declaration.
    pub declaration: Box<ClassDeclaration>,
    /// Constructor arguments.
    pub args: Vec<Argument>,
}
/// Class method, distinguishing abstract signatures from empty bodies.
#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    /// Attribute groups.
    pub attributes: Vec<AttributeGroup>,
    /// Explicit modifiers.
    pub flags: Modifiers,
    /// Method name, including semi-reserved spellings.
    pub name: Identifier,
    /// Shared callable signature.
    pub signature: Signature,
    /// Absent for a semicolon body.
    pub body: Option<Vec<Stmt>>,
}
/// One declared property.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyItem {
    /// Source origin.
    pub meta: NodeMeta,
    /// Name without the dollar sign.
    pub name: Identifier,
    /// Optional initializer.
    pub default: Option<Box<Expr>>,
}
/// A property declaration with optional PHP 8.4 hooks.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// Attribute groups.
    pub attributes: Vec<AttributeGroup>,
    /// Explicit modifiers; `var` preserves zero.
    pub flags: Modifiers,
    /// Optional declared type.
    pub ty: Option<Type>,
    /// Ordered property items.
    pub items: Vec<PropertyItem>,
    /// Optional hooks.
    pub hooks: Vec<PropertyHook>,
}
/// Property hook body syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum HookBody {
    /// Abstract hook with a semicolon.
    Abstract,
    /// Braced statement body.
    Block(Vec<Stmt>),
    /// Arrow expression body.
    Expression(Box<Expr>),
}
/// A property hook, also usable on promoted parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyHook {
    /// Source origin.
    pub meta: NodeMeta,
    /// Attribute groups.
    pub attributes: Vec<AttributeGroup>,
    /// Explicit modifiers.
    pub flags: Modifiers,
    /// Return by reference.
    pub by_ref: bool,
    /// Original hook name (unknown names retained on recovery).
    pub name: Identifier,
    /// Explicit parameters, if present.
    pub params: Vec<super::Parameter>,
    /// Whether parentheses were present, including an empty list.
    pub has_parameter_list: bool,
    /// Hook body.
    pub body: HookBody,
}
/// Enum case declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumCase {
    /// Attribute groups.
    pub attributes: Vec<AttributeGroup>,
    /// Case name.
    pub name: Identifier,
    /// Optional backing value.
    pub value: Option<Box<Expr>>,
}
/// Trait import and adaptations.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitUse {
    /// Imported traits.
    pub traits: Vec<Name>,
    /// Ordered method adaptations.
    pub adaptations: Vec<TraitAdaptation>,
}
/// Method alias or precedence selection.
#[derive(Debug, Clone, PartialEq)]
pub enum TraitAdaptation {
    /// Rename a method or change a modifier.
    Alias {
        /// Source origin.
        meta: NodeMeta,
        /// Optional trait qualifier.
        trait_name: Option<Name>,
        /// Original method.
        method: Identifier,
        /// Optional modifier.
        modifier: Option<Modifiers>,
        /// Optional new name.
        alias: Option<Identifier>,
    },
    /// Prefer a qualified method over those of other traits.
    Precedence {
        /// Source origin.
        meta: NodeMeta,
        /// Selected trait.
        trait_name: Name,
        /// Selected method.
        method: Identifier,
        /// Excluded traits.
        instead_of: Vec<Name>,
    },
}
impl TraitAdaptation {
    /// Source origin.
    #[must_use]
    pub const fn meta(&self) -> &NodeMeta {
        match self {
            Self::Alias { meta, .. } | Self::Precedence { meta, .. } => meta,
        }
    }
}
