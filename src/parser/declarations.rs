//! Declaration grammar shared by top-level statements, members and attributes.
use super::{PResult, Parser};
use crate::{
    ast::{
        Argument, Attribute, AttributeGroup, ClassDeclaration, ClassKind, Constant, Constants,
        EnumCase, HookBody, Identifier, Method, Modifiers, Name, Property, PropertyHook,
        PropertyItem, Qualification, Signature, Stmt, StmtKind, TraitAdaptation, TraitUse,
    },
    diagnostic::{Diagnostic, DiagnosticCode as D},
    token::TokenKind as K,
};

impl Parser {
    pub(super) fn attributes(&mut self) -> PResult<Vec<AttributeGroup>> {
        let mut groups = Vec::new();
        while self.kind()? == K::Attribute {
            let start = self.bump()?.0;
            self.enter()?;
            let result = self.attribute_group(start);
            self.depth -= 1;
            groups.push(result?);
        }
        Ok(groups)
    }
    fn attribute_group(&mut self, start: usize) -> PResult<AttributeGroup> {
        let mut attributes = Vec::new();
        loop {
            let at = self.peek()?.0;
            let name = self.class_name()?;
            let args = if self.kind()? == K::LeftParen {
                self.arguments()?
            } else {
                Vec::new()
            };
            self.alloc()?;
            attributes.push(Attribute {
                meta: self.meta(at),
                name,
                args,
            });
            if !self.eat(K::Comma)? || self.kind()? == K::RightBracket {
                break;
            }
        }
        self.expect(K::RightBracket)?;
        self.alloc()?;
        Ok(AttributeGroup {
            meta: self.meta(start),
            attributes,
        })
    }
    pub(super) fn attributed_statement(&mut self) -> PResult<StmtKind> {
        let start = self.peek()?.0;
        let attributes = self.attributes()?;
        match self.kind()? {
            K::Function
                if self.ahead(1)? != K::LeftParen
                    && !(self.ahead(1)? == K::Ampersand && self.ahead(2)? == K::LeftParen) =>
            {
                self.function_statement(attributes)
            }
            K::Class | K::Interface | K::Trait | K::Enum | K::Abstract | K::Final | K::Readonly => {
                Ok(StmtKind::Class(
                    self.class_declaration(attributes, false)?.0,
                ))
            }
            K::Const => {
                // Global constant attributes were added after the supported ceiling.
                let (_, token) = self.peek()?;
                self.record(Diagnostic::error(
                    D::UnsupportedSyntax,
                    token.span(),
                    "global constant attributes require PHP 8.5",
                ))?;
                self.constants_statement(attributes, Modifiers::default(), false)
            }
            _ => {
                let expr = self.callable_expression_with(attributes, start)?;
                self.terminate()?;
                Ok(StmtKind::Expression(*expr))
            }
        }
    }
    pub(super) fn reserved_identifier(&mut self) -> PResult<Identifier> {
        let (start, token) = self.peek()?;
        let bytes = self.bytes(token).to_vec();
        if token.kind() == K::OpenEchoTag {
            self.record(Diagnostic::error(
                D::InvalidContext,
                token.span(),
                "short echo tag cannot be an identifier",
            ))?;
        } else if !super::expression::is_identifier(&bytes) || token.kind() == K::Variable {
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "expected identifier",
            ))?;
            return Err(());
        }
        self.bump()?;
        self.identifier(start, &bytes)
    }
    pub(super) fn modifier(kind: K) -> Option<Modifiers> {
        Some(match kind {
            K::Public => Modifiers::PUBLIC,
            K::Protected => Modifiers::PROTECTED,
            K::Private => Modifiers::PRIVATE,
            K::Static => Modifiers::STATIC,
            K::Abstract => Modifiers::ABSTRACT,
            K::Final => Modifiers::FINAL,
            K::Readonly => Modifiers::READONLY,
            K::PublicSet => Modifiers::PUBLIC_SET,
            K::ProtectedSet => Modifiers::PROTECTED_SET,
            K::PrivateSet => Modifiers::PRIVATE_SET,
            _ => return None,
        })
    }
    /// Modifier mode: class, parameter, hook, or ordinary member.
    pub(super) fn modifiers(&mut self, mode: ModifierMode) -> PResult<Modifiers> {
        let mut flags = Modifiers::default();
        while let Some(next) = Self::modifier(self.kind()?) {
            if mode == ModifierMode::Class && next.0 & (16 | 32 | 64) == 0 {
                break;
            }
            if mode == ModifierMode::Parameter
                && next.0 & (1 | 2 | 4 | 32 | 64 | 128 | 256 | 512) == 0
            {
                break;
            }
            let (_, token) = self.bump()?;
            if mode == ModifierMode::Parameter && next == Modifiers::FINAL {
                self.record(Diagnostic::error(
                    D::UnsupportedSyntax,
                    token.span(),
                    "final promoted parameters require PHP 8.5",
                ))?;
            }
            let duplicate = flags.0 & next.0 != 0;
            let access = mode != ModifierMode::Class
                && (flags.0 & 7 != 0 && next.0 & 7 != 0
                    || flags.0 & 0x0380 != 0 && next.0 & 0x0380 != 0);
            let conflict = flags.0 & 0x0030 != 0 && next.0 & 0x0030 != 0;
            if duplicate || access || conflict {
                self.record(Diagnostic::error(
                    D::InvalidContext,
                    token.span(),
                    "conflicting or repeated modifiers",
                ))?;
            }
            if mode == ModifierMode::Hook && next != Modifiers::FINAL {
                self.record(Diagnostic::error(
                    D::InvalidContext,
                    token.span(),
                    "only final is allowed on a property hook",
                ))?;
            }
            flags.0 |= next.0;
        }
        Ok(flags)
    }
    pub(super) fn class_declaration(
        &mut self,
        attributes: Vec<AttributeGroup>,
        anonymous: bool,
    ) -> PResult<(Box<ClassDeclaration>, Vec<Argument>)> {
        let flags = self.modifiers(ModifierMode::Class)?;
        let (_, token) = self.bump()?;
        let kind = match token.kind() {
            K::Class => ClassKind::Class,
            K::Interface => ClassKind::Interface,
            K::Trait => ClassKind::Trait,
            K::Enum => ClassKind::Enum,
            _ => {
                self.record(Diagnostic::error(
                    D::UnexpectedToken,
                    token.span(),
                    "expected class declaration",
                ))?;
                return Err(());
            }
        };
        if kind != ClassKind::Class && (anonymous || flags.0 != 0) {
            self.record(Diagnostic::error(
                D::InvalidContext,
                token.span(),
                "modifiers and anonymous form require a class",
            ))?;
        }
        let name = if anonymous {
            None
        } else {
            Some(self.plain_identifier()?)
        };
        let args = if anonymous && self.kind()? == K::LeftParen {
            self.arguments()?
        } else {
            Vec::new()
        };
        let scalar_type = if kind == ClassKind::Enum && self.eat(K::Colon)? {
            Some(self.atomic_type(true)?)
        } else {
            None
        };
        let extends = if self.eat(K::Extends)? {
            if kind == ClassKind::Interface {
                self.class_names()?
            } else {
                vec![self.class_name()?]
            }
        } else {
            Vec::new()
        };
        let implements = if self.eat(K::Implements)? {
            self.class_names()?
        } else {
            Vec::new()
        };
        if kind == ClassKind::Trait && (!extends.is_empty() || !implements.is_empty())
            || kind == ClassKind::Interface && !implements.is_empty()
            || kind == ClassKind::Enum && !extends.is_empty()
        {
            self.record(Diagnostic::error(
                D::InvalidContext,
                token.span(),
                "invalid inheritance clause",
            ))?;
        }
        if kind != ClassKind::Trait {
            if let Some(name) = &name {
                self.check_special_identifier(name)?;
            }
            for n in extends.iter().chain(&implements) {
                self.check_special_name(n)?;
            }
        }
        self.expect(K::LeftBrace)?;
        self.enter()?;
        self.block_depth += 1;
        let result = self.class_members();
        self.block_depth -= 1;
        self.depth -= 1;
        let members = result?;
        self.expect(K::RightBrace)?;
        Ok((
            Box::new(ClassDeclaration {
                kind,
                name,
                flags,
                attributes,
                extends,
                implements,
                scalar_type,
                members,
            }),
            args,
        ))
    }
    fn class_name(&mut self) -> PResult<Name> {
        let (at, token) = self.peek()?;
        if token.kind() == K::Static {
            self.bump()?;
            self.name(at, token)
        } else {
            self.type_name()
        }
    }
    fn class_names(&mut self) -> PResult<Vec<Name>> {
        let mut names = vec![self.class_name()?];
        while self.eat(K::Comma)? {
            if matches!(self.kind()?, K::LeftBrace | K::Semicolon) {
                let (_, token) = self.peek()?;
                self.record(Diagnostic::error(
                    D::UnexpectedToken,
                    token.span(),
                    "trailing comma is not allowed in class names",
                ))?;
                break;
            }
            names.push(self.class_name()?);
        }
        Ok(names)
    }
    pub(super) fn check_special_identifier(&mut self, name: &Identifier) -> PResult<()> {
        if matches!(
            name.bytes().as_bytes().to_ascii_lowercase().as_slice(),
            b"self" | b"parent" | b"static"
        ) {
            self.error_meta(
                D::InvalidContext,
                name.meta(),
                "reserved special class name",
            )?;
        }
        Ok(())
    }
    fn check_special_name(&mut self, name: &Name) -> PResult<()> {
        if name.qualification() == Qualification::Normal
            && name.parts().len() == 1
            && matches!(
                name.parts()[0].as_bytes().to_ascii_lowercase().as_slice(),
                b"self" | b"parent" | b"static"
            )
        {
            self.error_meta(
                D::InvalidContext,
                name.meta(),
                "reserved special class name",
            )?;
        }
        Ok(())
    }
    fn class_members(&mut self) -> PResult<Vec<Stmt>> {
        let mut members = Vec::new();
        while !matches!(self.kind()?, K::RightBrace | K::Eof) {
            let before = self.cursor.position();
            match self.class_member() {
                Ok(member) => members.push(member),
                Err(()) if !self.aborted => self.recover_member(),
                Err(()) => return Err(()),
            }
            if !self.aborted && before == self.cursor.position() {
                self.bump()?;
            }
        }
        if self.kind()? == K::RightBrace {
            let index = self.peek()?.0;
            if let Some(nop) = self.trailing_nop(index)? {
                members.push(nop);
            }
        }
        Ok(members)
    }
    fn class_member(&mut self) -> PResult<Stmt> {
        let start = self.peek()?.0;
        let attributes = self.attributes()?;
        let var = self.eat(K::Var)?;
        let flags = if var {
            Modifiers::default()
        } else {
            self.modifiers(ModifierMode::Member)?
        };
        let kind = match self.kind()? {
            K::Function => self.method(attributes, flags)?,
            K::Const => self.constants_statement(attributes, flags, true)?,
            K::Use if !var && flags.0 == 0 && attributes.is_empty() => self.trait_use()?,
            K::Case if !var && flags.0 == 0 => {
                self.bump()?;
                let name = self.reserved_identifier()?;
                let value = if self.eat(K::Equal)? {
                    Some(self.expression(0)?)
                } else {
                    None
                };
                self.terminate()?;
                StmtKind::EnumCase(Box::new(EnumCase {
                    attributes,
                    name,
                    value,
                }))
            }
            _ if var || flags.0 != 0 => self.property(attributes, flags)?,
            _ => {
                let (_, token) = self.peek()?;
                self.record(Diagnostic::error(
                    D::UnexpectedToken,
                    token.span(),
                    "expected class member",
                ))?;
                return Err(());
            }
        };
        if var && !matches!(kind, StmtKind::Property(_)) {
            let meta = self.meta(start);
            self.error_meta(D::InvalidContext, &meta, "var requires a property")?;
        }
        self.alloc()?;
        Ok(Stmt {
            meta: self.meta(start),
            kind,
        })
    }
    fn method(&mut self, attributes: Vec<AttributeGroup>, flags: Modifiers) -> PResult<StmtKind> {
        self.bump()?;
        let by_ref = self.eat(K::Ampersand)?;
        let name = self.reserved_identifier()?;
        let params = self.parameters()?;
        let return_type = self.return_type()?;
        let body = if self.eat(K::Semicolon)? {
            None
        } else {
            Some(self.callable_body()?)
        };
        if flags.contains(Modifiers::READONLY)
            || flags.contains(Modifiers::STATIC)
                && matches!(
                    name.bytes().as_bytes().to_ascii_lowercase().as_slice(),
                    b"__construct" | b"__destruct" | b"__clone"
                )
        {
            self.error_meta(D::InvalidContext, name.meta(), "invalid method modifier")?;
        }
        Ok(StmtKind::Method(Box::new(Method {
            attributes,
            flags,
            name,
            signature: Signature {
                by_ref,
                params,
                return_type,
            },
            body,
        })))
    }
    pub(super) fn constants_statement(
        &mut self,
        attributes: Vec<AttributeGroup>,
        flags: Modifiers,
        class: bool,
    ) -> PResult<StmtKind> {
        let start = self.bump()?.0;
        let ty = if class && self.ahead(1)? != K::Equal {
            Some(self.parse_type(true)?)
        } else {
            None
        };
        let mut constants = Vec::new();
        loop {
            let at = self.peek()?.0;
            let name = if class {
                self.reserved_identifier()?
            } else {
                self.plain_identifier()?
            };
            self.expect(K::Equal)?;
            let value = self.expression(0)?;
            self.alloc()?;
            constants.push(Constant {
                meta: self.meta(at),
                name,
                value,
            });
            if !self.eat(K::Comma)? {
                break;
            }
            if matches!(self.kind()?, K::Semicolon | K::CloseTag) {
                let meta = self.meta(at);
                self.error_meta(
                    D::UnexpectedToken,
                    &meta,
                    "trailing comma is not allowed in constants",
                )?;
                break;
            }
        }
        self.terminate()?;
        if class && flags.0 & (8 | 16 | 64) != 0 {
            let meta = self.meta(start);
            self.error_meta(D::InvalidContext, &meta, "invalid class constant modifier")?;
        }
        let value = Box::new(Constants {
            attributes,
            flags,
            ty,
            constants,
        });
        Ok(if class {
            StmtKind::ClassConstants(value)
        } else {
            StmtKind::Constants(value)
        })
    }
    fn property(&mut self, attributes: Vec<AttributeGroup>, flags: Modifiers) -> PResult<StmtKind> {
        let ty = if self.kind()? == K::Variable {
            None
        } else {
            Some(self.parse_type(false)?)
        };
        let mut items = Vec::new();
        loop {
            let (start, token) = self.expect(K::Variable)?;
            let bytes = self.bytes(token)[1..].to_vec();
            let name = self.identifier(start, &bytes)?;
            let default = if self.eat(K::Equal)? {
                Some(self.expression(0)?)
            } else {
                None
            };
            self.alloc()?;
            items.push(PropertyItem {
                meta: self.meta(start),
                name,
                default,
            });
            if !self.eat(K::Comma)? {
                break;
            }
            if matches!(self.kind()?, K::Semicolon | K::LeftBrace | K::CloseTag) {
                let meta = self.meta(start);
                self.error_meta(
                    D::UnexpectedToken,
                    &meta,
                    "trailing comma is not allowed in properties",
                )?;
                break;
            }
        }
        let hooks = if self.kind()? == K::LeftBrace {
            if items.len() > 1 {
                let (_, token) = self.peek()?;
                self.record(Diagnostic::error(
                    D::InvalidContext,
                    token.span(),
                    "hooks require a single property",
                ))?;
            }
            self.property_hooks()?
        } else {
            self.terminate()?;
            Vec::new()
        };
        Ok(StmtKind::Property(Box::new(Property {
            attributes,
            flags,
            ty,
            items,
            hooks,
        })))
    }
    pub(super) fn property_hooks(&mut self) -> PResult<Vec<PropertyHook>> {
        let (start, token) = self.expect(K::LeftBrace)?;
        if self.options.version < crate::PhpVersion::Php80 {
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "property hooks require the PHP 8 grammar",
            ))?;
        }
        self.enter()?;
        self.block_depth += 1;
        let result = self.hook_list();
        self.block_depth -= 1;
        self.depth -= 1;
        let hooks = result?;
        if self.starts_member_in_hook()? {
            let (_, token) = self.peek()?;
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "expected closing brace before next member",
            ))?;
        } else {
            self.expect(K::RightBrace)?;
        }
        if hooks.is_empty() {
            let meta = self.meta(start);
            self.error_meta(
                D::InvalidContext,
                &meta,
                "property hook list cannot be empty",
            )?;
        }
        Ok(hooks)
    }
    fn hook_list(&mut self) -> PResult<Vec<PropertyHook>> {
        let mut hooks = Vec::new();
        while !matches!(self.kind()?, K::RightBrace | K::Eof) {
            if self.starts_member_in_hook()? {
                break;
            }
            let before = self.cursor.position();
            match self.property_hook() {
                Ok(hook) => hooks.push(hook),
                Err(()) if !self.aborted => self.recover_member(),
                Err(()) => return Err(()),
            }
            if !self.aborted && before == self.cursor.position() {
                self.bump()?;
            }
        }
        Ok(hooks)
    }
    fn starts_member_in_hook(&mut self) -> PResult<bool> {
        let mut offset = 0;
        // Bound lookahead independently of malformed modifier lists.
        while offset < 2 && Self::modifier(self.ahead(offset)?).is_some() {
            offset += 1;
        }
        let kind = self.ahead(offset)?;
        if matches!(kind, K::Function | K::Const | K::Use | K::Var) {
            return Ok(true);
        }
        if offset == 0 {
            return Ok(false);
        }
        if kind == K::Variable {
            return Ok(true);
        }
        Ok(offset < 2
            && matches!(
                kind,
                K::Identifier
                    | K::NameQualified
                    | K::NameFullyQualified
                    | K::Array
                    | K::Callable
                    | K::Question
            )
            && self.ahead(offset + 1)? == K::Variable)
    }
    fn property_hook(&mut self) -> PResult<PropertyHook> {
        let start = self.peek()?.0;
        let attributes = self.attributes()?;
        let flags = self.modifiers(ModifierMode::Hook)?;
        let by_ref = self.eat(K::Ampersand)?;
        let name = self.plain_identifier()?;
        let has_parameter_list = self.kind()? == K::LeftParen;
        let params = if has_parameter_list {
            self.parameters()?
        } else {
            Vec::new()
        };
        let body = if self.eat(K::Semicolon)? {
            HookBody::Abstract
        } else if self.eat(K::DoubleArrow)? {
            let expr = self.expression(0)?;
            self.terminate()?;
            HookBody::Expression(expr)
        } else {
            HookBody::Block(self.braced_body()?)
        };
        let lower = name.bytes().as_bytes().to_ascii_lowercase();
        if !matches!(lower.as_slice(), b"get" | b"set") || lower == b"get" && has_parameter_list {
            self.error_meta(
                D::InvalidContext,
                name.meta(),
                "invalid property hook name or parameter list",
            )?;
        }
        self.alloc()?;
        Ok(PropertyHook {
            meta: self.meta(start),
            attributes,
            flags,
            by_ref,
            name,
            params,
            has_parameter_list,
            body,
        })
    }
    fn trait_use(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        let traits = self.class_names()?;
        let mut adaptations = Vec::new();
        if !self.eat(K::Semicolon)? {
            self.expect(K::LeftBrace)?;
            while !matches!(self.kind()?, K::RightBrace | K::Eof) {
                let before = self.cursor.position();
                match self.trait_adaptation() {
                    Ok(adaptation) => adaptations.push(adaptation),
                    Err(()) if !self.aborted => self.recover_statement(),
                    Err(()) => return Err(()),
                }
                if !self.aborted && self.cursor.position() == before {
                    self.bump()?;
                }
            }
            self.expect(K::RightBrace)?;
        }
        Ok(StmtKind::TraitUse(Box::new(TraitUse {
            traits,
            adaptations,
        })))
    }
    fn trait_adaptation(&mut self) -> PResult<TraitAdaptation> {
        let start = self.peek()?.0;
        let (trait_name, method) = if matches!(
            self.kind()?,
            K::NameQualified | K::NameFullyQualified | K::NameRelative
        ) || self.ahead(1)? == K::DoubleColon
        {
            let n = self.type_name()?;
            self.expect(K::DoubleColon)?;
            (Some(n), self.reserved_identifier()?)
        } else {
            (None, self.reserved_identifier()?)
        };
        let adaptation = if self.eat(K::InsteadOf)? {
            let Some(trait_name) = trait_name else {
                let meta = self.meta(start);
                self.error_meta(
                    D::UnexpectedToken,
                    &meta,
                    "insteadof requires a trait qualifier",
                )?;
                return Err(());
            };
            let instead_of = self.class_names()?;
            self.terminate()?;
            TraitAdaptation::Precedence {
                meta: self.meta(start),
                trait_name,
                method,
                instead_of,
            }
        } else {
            self.expect(K::As)?;
            let modifier = Self::modifier(self.kind()?);
            if modifier.is_some() {
                self.bump()?;
            }
            let alias = if self.kind()? == K::Semicolon {
                None
            } else {
                Some(self.reserved_identifier()?)
            };
            if modifier.is_none() && alias.is_none() {
                let meta = self.meta(start);
                self.error_meta(D::UnexpectedToken, &meta, "expected alias or modifier")?;
            }
            self.terminate()?;
            TraitAdaptation::Alias {
                meta: self.meta(start),
                trait_name,
                method,
                modifier,
                alias,
            }
        };
        self.alloc()?;
        Ok(adaptation)
    }
    /// Synchronize one damaged member without consuming the next member's anchor.
    fn recover_member(&mut self) {
        let mut depth = 0usize;
        while let Ok(kind) = self.kind() {
            if kind == K::Eof || depth == 0 && kind == K::RightBrace {
                break;
            }
            if depth == 0
                && (Self::modifier(kind).is_some()
                    || matches!(
                        kind,
                        K::Function | K::Const | K::Use | K::Case | K::Var | K::Attribute
                    ))
            {
                break;
            }
            if kind == K::LeftBrace {
                depth += 1;
            }
            if kind == K::RightBrace {
                depth = depth.saturating_sub(1);
            }
            if depth
                > self
                    .options
                    .limits
                    .get(crate::options::Resource::Depth)
                    .min(128)
            {
                let _ = self.limit(crate::options::Resource::Depth);
                break;
            }
            if self.bump().is_err() {
                break;
            }
            if depth == 0 && matches!(kind, K::Semicolon | K::RightBrace) {
                break;
            }
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ModifierMode {
    Class,
    Member,
    Parameter,
    Hook,
}
