//! Callable declarations, closures and shared target-aware type grammar.
use super::{PResult, Parser};
use crate::{
    PhpVersion,
    ast::{
        ArrowFunction, AttributeGroup, Closure, ClosureUse, Expr, ExprKind, Function, Identifier,
        Name, Parameter, Signature, StmtKind, Type,
    },
    diagnostic::{Diagnostic, DiagnosticCode as D},
    token::TokenKind as K,
};
impl Parser {
    pub(super) fn plain_identifier(&mut self) -> PResult<Identifier> {
        let (start, token) = self.peek()?;
        if token.kind() != K::Identifier {
            self.expect(K::Identifier)?;
            unreachable!();
        }
        self.bump()?;
        let bytes = self.bytes(token).to_vec();
        self.identifier(start, &bytes)
    }
    pub(super) fn type_name(&mut self) -> PResult<Name> {
        let (start, token) = self.peek()?;
        if !matches!(
            token.kind(),
            K::Identifier | K::NameQualified | K::NameFullyQualified | K::NameRelative
        ) {
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "expected type name",
            ))?;
            return Err(());
        }
        self.bump()?;
        self.name(start, token)
    }
    pub(super) fn require_simple_variable(&mut self) -> PResult<()> {
        if self.kind()? != K::Variable {
            self.expect(K::Variable)?;
        }
        Ok(())
    }
    pub(super) fn function_statement(
        &mut self,
        attributes: Vec<AttributeGroup>,
    ) -> PResult<StmtKind> {
        self.bump()?;
        let by_ref = self.eat(K::Ampersand)?;
        let (at, token) = self.peek()?;
        if token.kind() == K::Clone {
            self.record(Diagnostic::error(
                D::UnsupportedSyntax,
                token.span(),
                "clone function declarations require PHP 8.5, beyond the PHP 8.4 ceiling",
            ))?;
        }
        let name = if matches!(token.kind(), K::Readonly | K::Exit | K::Clone) {
            self.bump()?;
            let bytes = self.bytes(token).to_vec();
            self.identifier(at, &bytes)?
        } else {
            self.plain_identifier()?
        };
        let params = self.parameters()?;
        let return_type = self.return_type()?;
        let body = self.callable_body()?;
        Ok(StmtKind::Function(Box::new(Function {
            attributes,
            name,
            signature: Signature {
                by_ref,
                params,
                return_type,
            },
            body,
        })))
    }
    pub(super) fn callable_expression(&mut self) -> PResult<Box<Expr>> {
        let start = self.peek()?.0;
        self.callable_expression_with(Vec::new(), start)
    }
    pub(super) fn callable_expression_with(
        &mut self,
        attributes: Vec<AttributeGroup>,
        start: usize,
    ) -> PResult<Box<Expr>> {
        let is_static = self.eat(K::Static)?;
        if !matches!(self.kind()?, K::Function | K::Fn) {
            let (_, token) = self.peek()?;
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "attributes require a declaration or callable",
            ))?;
            return Err(());
        }
        let (_, token) = self.bump()?;
        let arrow = token.kind() == K::Fn;
        let by_ref = self.eat(K::Ampersand)?;
        let params = self.parameters()?;
        let mut uses = Vec::new();
        if !arrow && self.eat(K::Use)? {
            self.expect(K::LeftParen)?;
            loop {
                let at = self.peek()?.0;
                let by_ref = self.eat(K::Ampersand)?;
                self.require_simple_variable()?;
                let variable = self.variable()?;
                self.alloc()?;
                uses.push(ClosureUse {
                    meta: self.meta(at),
                    variable,
                    by_ref,
                });
                if !self.eat(K::Comma)? || self.kind()? == K::RightParen {
                    break;
                }
            }
            self.expect(K::RightParen)?;
        }
        let return_type = self.return_type()?;
        let signature = Signature {
            by_ref,
            params,
            return_type,
        };
        if arrow {
            self.expect(K::DoubleArrow)?;
            self.block_depth += 1;
            let result = self.expression(1);
            self.block_depth -= 1;
            self.make(
                start,
                ExprKind::ArrowFunction(Box::new(ArrowFunction {
                    attributes,
                    is_static,
                    signature,
                    expr: result?,
                })),
            )
        } else {
            let body = self.callable_body()?;
            self.make(
                start,
                ExprKind::Closure(Box::new(Closure {
                    attributes,
                    is_static,
                    signature,
                    uses,
                    body,
                })),
            )
        }
    }
    pub(super) fn callable_body(&mut self) -> PResult<Vec<crate::ast::Stmt>> {
        if self.kind()? == K::LeftBrace {
            self.braced_body()
        } else {
            let (_, token) = self.peek()?;
            self.syntax(token, &[K::LeftBrace], "expected function body")?;
            // The reference retains a declaration with an empty body on this recovery.
            Ok(Vec::new())
        }
    }
    pub(super) fn parameters(&mut self) -> PResult<Vec<Parameter>> {
        self.expect(K::LeftParen)?;
        let mut params = Vec::new();
        while !matches!(self.kind()?, K::RightParen | K::Eof) {
            let before = self.cursor.position();
            match self.parameter() {
                Ok(param) => params.push(param),
                Err(()) if !self.aborted => self.recover_signature(),
                Err(()) => return Err(()),
            }
            if self.kind()? == K::RightParen {
                break;
            }
            if !self.eat(K::Comma)? {
                let (_, token) = self.peek()?;
                self.record(Diagnostic::error(
                    D::UnexpectedToken,
                    token.span(),
                    "expected comma in parameter list",
                ))?;
                if matches!(token.kind(), K::LeftBrace | K::Semicolon | K::CloseTag) {
                    return Err(());
                }
                if self.cursor.position() == before {
                    self.bump()?;
                }
            }
        }
        self.expect(K::RightParen)?;
        Ok(params)
    }
    fn parameter(&mut self) -> PResult<Parameter> {
        let start = self.peek()?.0;
        let attributes = self.attributes()?;
        let flags = self.modifiers(super::declarations::ModifierMode::Parameter)?;
        let ty = if matches!(self.kind()?, K::Ampersand | K::Ellipsis | K::Variable) {
            None
        } else {
            Some(self.parse_type(false)?)
        };
        let by_ref = self.eat(K::Ampersand)?;
        let variadic = self.eat(K::Ellipsis)?;
        let variable = if self.kind()? == K::Variable {
            self.variable()?
        } else {
            let (at, token) = self.peek()?;
            self.syntax(token, &[K::Variable], "expected parameter variable")?;
            self.make(at, ExprKind::Error)?
        };
        let default = if self.eat(K::Equal)? {
            Some(self.expression(0)?)
        } else {
            None
        };
        let hooks = if self.kind()? == K::LeftBrace {
            self.property_hooks()?
        } else {
            Vec::new()
        };
        if variadic {
            if let Some(value) = &default {
                self.error_meta(
                    D::InvalidContext,
                    &value.meta,
                    "Variadic parameter cannot have a default value",
                )?;
            }
        }
        if let Some(Type::Builtin(name)) = &ty {
            if name.bytes().as_bytes() == b"void" {
                self.error_meta(
                    D::InvalidContext,
                    name.meta(),
                    "void cannot be used as a parameter type",
                )?;
            }
        }
        self.alloc()?;
        Ok(Parameter {
            attributes,
            flags,
            hooks,
            meta: self.meta(start),
            variable,
            ty,
            by_ref,
            variadic,
            default,
        })
    }
    pub(super) fn return_type(&mut self) -> PResult<Option<Type>> {
        if self.eat(K::Colon)? {
            Ok(Some(self.parse_type(true)?))
        } else {
            Ok(None)
        }
    }
    pub(super) fn parse_type(&mut self, allow_static: bool) -> PResult<Type> {
        self.enter()?;
        let result = self.type_inner(allow_static);
        self.depth -= 1;
        result
    }
    fn type_inner(&mut self, allow_static: bool) -> PResult<Type> {
        let start = self.peek()?.0;
        if self.eat(K::Question)? {
            let inner = Box::new(self.atomic_type(allow_static)?);
            self.alloc()?;
            return Ok(Type::Nullable {
                meta: self.meta(start),
                inner,
            });
        }
        let (first, parenthesized) = self.union_element(allow_static)?;
        if self.eat(K::Pipe)? {
            if !parenthesized && matches!(first, Type::Intersection { .. }) {
                self.error_meta(
                    D::UnexpectedToken,
                    first.meta(),
                    "intersection in a union requires parentheses",
                )?;
            }
            let mut types = vec![first];
            loop {
                let (ty, parenthesized) = self.union_element(allow_static)?;
                if !parenthesized && matches!(ty, Type::Intersection { .. }) {
                    self.error_meta(
                        D::UnexpectedToken,
                        ty.meta(),
                        "intersection in a union requires parentheses",
                    )?;
                }
                types.push(ty);
                if !self.eat(K::Pipe)? {
                    break;
                }
            }
            self.alloc()?;
            return Ok(Type::Union {
                meta: self.meta(start),
                types,
            });
        }
        if parenthesized {
            self.error_meta(
                D::UnexpectedToken,
                first.meta(),
                "parenthesized intersection requires a union",
            )?;
        }
        Ok(first)
    }
    fn union_element(&mut self, allow_static: bool) -> PResult<(Type, bool)> {
        let start = self.peek()?.0;
        if self.eat(K::LeftParen)? {
            self.enter()?;
            let result = self.union_element(allow_static);
            self.depth -= 1;
            let (ty, nested) = result?;
            if nested {
                self.error_meta(
                    D::UnexpectedToken,
                    ty.meta(),
                    "nested type parentheses are invalid",
                )?;
            }
            self.expect(K::RightParen)?;
            if !matches!(ty, Type::Intersection { .. }) {
                self.error_meta(
                    D::InvalidContext,
                    ty.meta(),
                    "only intersection types may be parenthesized",
                )?;
            }
            return Ok((ty, true));
        }
        let first = self.atomic_type(allow_static)?;
        let mut types = vec![first];
        while self.kind()? == K::Ampersand
            && !matches!(self.ahead(1)?, K::Variable | K::Ellipsis | K::Ampersand)
        {
            self.bump()?;
            types.push(self.atomic_type(allow_static)?);
        }
        if types.len() == 1 {
            return Ok((types.pop().unwrap(), false));
        }
        self.alloc()?;
        Ok((
            Type::Intersection {
                meta: self.meta(start),
                types,
            },
            false,
        ))
    }
    pub(super) fn atomic_type(&mut self, allow_static: bool) -> PResult<Type> {
        let (start, token) = self.peek()?;
        if token.kind() == K::Static && allow_static {
            self.bump()?;
            // Static has a canonical lowercase spelling in type context.
            self.alloc()?;
            return Ok(Type::Name(
                Name::new(
                    vec!["static".into()],
                    crate::ast::Qualification::Normal,
                    self.meta(start),
                )
                .unwrap(),
            ));
        }
        if matches!(token.kind(), K::Array | K::Callable) {
            self.bump()?;
            let bytes = self.bytes(token).to_ascii_lowercase();
            return Ok(Type::Builtin(self.identifier(start, &bytes)?));
        }
        let name = self.type_name()?;
        if name.qualification() == crate::ast::Qualification::Normal && name.parts().len() == 1 {
            let lower = name.parts()[0].as_bytes().to_ascii_lowercase();
            let minimum = match lower.as_slice() {
                b"bool" | b"int" | b"float" | b"string" => Some(PhpVersion::Php70),
                b"iterable" | b"void" => Some(PhpVersion::Php71),
                b"object" => Some(PhpVersion::Php72),
                b"mixed" | b"null" | b"false" => Some(PhpVersion::Php80),
                b"never" => Some(PhpVersion::Php81),
                b"true" => Some(PhpVersion::Php82),
                _ => None,
            };
            if minimum.is_some_and(|v| self.options.version >= v) {
                // Reclassify the existing name node without charging a second AST node.
                return Ok(Type::Builtin(
                    Identifier::new(lower, self.meta(start)).unwrap(),
                ));
            }
        }
        Ok(Type::Name(name))
    }
    fn recover_signature(&mut self) {
        let mut depth = 0;
        while let Ok(kind) = self.kind() {
            if kind == K::Eof
                || depth == 0
                    && matches!(
                        kind,
                        K::Comma
                            | K::RightParen
                            | K::LeftBrace
                            | K::Semicolon
                            | K::CloseTag
                            | K::Eof
                    )
            {
                break;
            }
            match kind {
                K::LeftParen | K::LeftBracket => depth += 1,
                K::RightParen | K::RightBracket if depth > 0 => depth -= 1,
                _ => {}
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
        }
    }
}
