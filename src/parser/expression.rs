use super::{PResult, Parser, operators};
use crate::{
    PhpVersion,
    ast::{
        Argument, ArrayItem, ArrayKind, Expr, ExprKind, IncludeKind, IntegerBase, MagicConstant,
        MatchArm, Member, Origin, StringKind, Target, UnaryOp,
    },
    diagnostic::{Diagnostic, DiagnosticCode as D},
    literal::{self, Number},
    token::{Token, TokenKind as K},
};
impl Parser {
    pub(super) fn expression(&mut self, min: u8) -> PResult<Box<Expr>> {
        self.enter()?;
        let result = self.expression_inner(min).and_then(|expr| {
            if matches!(expr.kind, ExprKind::List { .. }) {
                self.error_meta(
                    D::InvalidContext,
                    &expr.meta,
                    "list requires an assignment context",
                )?;
            }
            Ok(expr)
        });
        self.depth -= 1;
        result
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Pratt operator dispatch keeps binding powers visible"
    )]
    fn expression_inner(&mut self, min: u8) -> PResult<Box<Expr>> {
        let start = self.peek()?.0;
        let mut left = self.prefix()?;
        let mut comparison = None;
        loop {
            let kind = self.kind()?;
            if min <= 80
                && matches!(
                    kind,
                    K::LeftParen
                        | K::LeftBracket
                        | K::ObjectArrow
                        | K::NullsafeArrow
                        | K::DoubleColon
                        | K::Increment
                        | K::Decrement
                )
            {
                left = self.postfix(start, left, true)?;
                continue;
            }
            if min <= 80 && kind == K::LeftBrace && self.options.version < PhpVersion::Php80 {
                left = self.dimension(start, left, K::RightBrace)?;
                continue;
            }
            if min <= 24 && (kind == K::Equal || operators::assignment(kind).is_some()) {
                self.bump()?;
                let by_ref = kind == K::Equal && self.eat(K::Ampersand)?;
                if kind == K::Equal {
                    Self::pattern(&mut left);
                }
                if !self.writable(&left, kind == K::Equal) {
                    self.error_meta(D::InvalidContext, &left.meta, "invalid assignment target")?;
                }
                let right = self.expression(if by_ref { 80 } else { 24 })?;
                if by_ref
                    && !self.writable(&right, false)
                    && !matches!(right.kind, ExprKind::New { .. } | ExprKind::NewAnonymous(_))
                {
                    self.error_meta(
                        D::InvalidContext,
                        &right.meta,
                        "reference requires a variable",
                    )?;
                }
                if by_ref && matches!(right.kind, ExprKind::New { .. } | ExprKind::NewAnonymous(_))
                {
                    let meta = self.meta(start);
                    self.error_meta(D::InvalidContext, &meta, "Cannot assign new by reference")?;
                }
                let node = if by_ref {
                    ExprKind::AssignRef {
                        target: left,
                        value: right,
                    }
                } else if let Some(op) = operators::assignment(kind) {
                    ExprKind::AssignOp {
                        op,
                        target: left,
                        value: right,
                    }
                } else {
                    ExprKind::Assign {
                        target: left,
                        value: right,
                    }
                };
                left = self.make(start, node)?;
                continue;
            }
            if kind == K::Question && min <= 26 {
                self.bump()?;
                let then = if self.eat(K::Colon)? {
                    None
                } else {
                    let value = self.expression(0)?;
                    self.expect(K::Colon)?;
                    Some(value)
                };
                let otherwise = self.expression(27)?;
                left = self.make(
                    start,
                    ExprKind::Ternary {
                        condition: left,
                        then,
                        otherwise,
                    },
                )?;
                continue;
            }
            if kind == K::Instanceof && min <= 54 {
                self.bump()?;
                let class = self.class_target()?;
                left = self.make(start, ExprKind::Instanceof { expr: left, class })?;
                continue;
            }
            let Some((op, bp, right_assoc)) = operators::binary(kind, self.options.version) else {
                break;
            };
            if bp < min {
                break;
            }
            if matches!(bp, 40 | 42) && comparison == Some(bp) {
                let (_, token) = self.peek()?;
                self.record(Diagnostic::error(
                    D::UnexpectedToken,
                    token.span(),
                    "comparison operators are non-associative",
                ))?;
            }
            comparison = Some(bp);
            self.bump()?;
            let right = self.expression(bp + u8::from(!right_assoc))?;
            left = self.make(start, ExprKind::Binary { op, left, right })?;
        }
        Ok(left)
    }
    #[allow(clippy::too_many_lines, reason = "exhaustive prefix grammar dispatch")]
    fn prefix(&mut self) -> PResult<Box<Expr>> {
        let (start, token) = self.peek()?;
        let kind = token.kind();
        if kind == K::Clone && self.ahead(1)? == K::LeftParen {
            return self.clone_expression(start, token);
        }
        if let Some((op, bp)) = operators::unary(kind) {
            self.bump()?;
            let expr = self.expression(bp)?;
            if matches!(op, UnaryOp::PreIncrement | UnaryOp::PreDecrement)
                && !self.writable(&expr, false)
            {
                self.error_meta(D::InvalidContext, &expr.meta, "invalid increment target")?;
            }
            return self.make(start, ExprKind::Unary { op, expr });
        }
        if let Some(kind) = operators::cast(kind) {
            self.bump()?;
            let expr = self.expression(56)?;
            return self.make(start, ExprKind::Cast { kind, expr });
        }
        match kind {
            K::Integer | K::Float => self.number_literal(start, token),
            K::QuotedString => self.quoted_literal(start, token),
            K::DoubleQuote | K::Backtick | K::StartHeredoc => self.string_expression(),
            K::Variable | K::Dollar => self.variable(),
            K::Attribute => {
                let start = self.peek()?.0;
                let attributes = self.attributes()?;
                self.callable_expression_with(attributes, start)
            }
            K::Function
                if self.ahead(1)? == K::Identifier
                    || self.ahead(1)? == K::Ampersand && self.ahead(2)? == K::Identifier =>
            {
                self.syntax(
                    token,
                    &[],
                    "expected expression before function declaration",
                )?;
                Err(())
            }
            K::Function | K::Fn => self.callable_expression(),
            K::Static if matches!(self.ahead(1)?, K::Function | K::Fn) => {
                self.callable_expression()
            }
            K::Identifier
            | K::NameQualified
            | K::NameFullyQualified
            | K::NameRelative
            | K::Static
            | K::Readonly => self.constant_name(start, token),
            K::LineConstant
            | K::FileConstant
            | K::DirConstant
            | K::ClassConstant
            | K::TraitConstant
            | K::MethodConstant
            | K::FunctionConstant
            | K::NamespaceConstant
            | K::PropertyConstant => self.magic_constant(start, kind),
            K::LeftParen => self.grouped(start),
            K::LeftBracket | K::Array | K::List => self.array(),
            K::New => self.new_expression(start),
            K::Include | K::IncludeOnce | K::Require | K::RequireOnce => {
                self.include_expression(start, kind)
            }
            K::Eval | K::Empty => self.parenthesized_construct(start, kind),
            K::Isset => self.isset_expression(start),
            K::Exit => self.exit_expression(start, token),
            K::Yield => self.yield_expression(start),
            K::Match => self.match_expression(),
            _ => {
                self.syntax(token, &[], format!("expected expression, found {kind:?}"))?;
                if Self::statement_anchor(kind) {
                    return Err(());
                }
                if operators::boundary(kind) {
                    self.alloc()?;
                    Ok(Box::new(Expr {
                        meta: self.point(start),
                        kind: ExprKind::Error,
                    }))
                } else {
                    self.bump()?;
                    self.make(start, ExprKind::Error)
                }
            }
        }
    }
    fn number_literal(&mut self, start: usize, token: Token) -> PResult<Box<Expr>> {
        self.bump()?;
        let value = match literal::decode_number(self.bytes(token)) {
            Ok(Number::Integer { value, base }) => ExprKind::Integer { value, base },
            Ok(Number::Float(value)) => ExprKind::Float(value),
            Err(error) => {
                self.record(Diagnostic::error(
                    D::InvalidLiteral,
                    token.span(),
                    error.to_string(),
                ))?;
                ExprKind::Integer {
                    value: 0,
                    base: IntegerBase::Octal,
                }
            }
        };
        self.make(start, value)
    }

    fn quoted_literal(&mut self, start: usize, token: Token) -> PResult<Box<Expr>> {
        self.bump()?;
        let Some(value) = self.decode_literal(token)? else {
            return self.make(start, ExprKind::Error);
        };
        let raw = self.bytes(token);
        let single = raw[0] == b'\'' || raw.get(1) == Some(&b'\'');
        self.make(
            start,
            ExprKind::String {
                value,
                kind: if single {
                    StringKind::SingleQuoted
                } else {
                    StringKind::DoubleQuoted
                },
            },
        )
    }

    fn constant_name(&mut self, start: usize, token: Token) -> PResult<Box<Expr>> {
        self.bump()?;
        if (token.kind() == K::Static && self.kind()? != K::DoubleColon)
            || (token.kind() == K::Readonly && self.kind()? != K::LeftParen)
        {
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "keyword is not a constant name in this context",
            ))?;
        }
        let name = self.name(start, token)?;
        self.make(start, ExprKind::ConstFetch(name))
    }

    fn magic_constant(&mut self, start: usize, kind: K) -> PResult<Box<Expr>> {
        self.bump()?;
        let magic = match kind {
            K::LineConstant => MagicConstant::Line,
            K::FileConstant => MagicConstant::File,
            K::DirConstant => MagicConstant::Directory,
            K::ClassConstant => MagicConstant::Class,
            K::TraitConstant => MagicConstant::Trait,
            K::MethodConstant => MagicConstant::Method,
            K::FunctionConstant => MagicConstant::Function,
            K::NamespaceConstant => MagicConstant::Namespace,
            _ => MagicConstant::Property,
        };
        self.make(start, ExprKind::MagicConstant(magic))
    }

    fn grouped(&mut self, start: usize) -> PResult<Box<Expr>> {
        self.bump()?;
        let mut expr = self.expression(0)?;
        self.expect(K::RightParen)?;
        expr.meta.parenthesize(&self.meta(start));
        Ok(expr)
    }

    fn new_expression(&mut self, start: usize) -> PResult<Box<Expr>> {
        self.bump()?;
        if matches!(
            self.kind()?,
            K::Class | K::Attribute | K::Readonly | K::Abstract | K::Final
        ) {
            let at = self.peek()?.0;
            let attributes = self.attributes()?;
            let (declaration, args) = self.class_declaration(attributes, true)?;
            self.alloc()?;
            return self.make(
                start,
                ExprKind::NewAnonymous(Box::new(crate::ast::AnonymousClass {
                    meta: self.meta(at),
                    declaration,
                    args,
                })),
            );
        }
        let class = self.class_target()?;
        let args = if self.kind()? == K::LeftParen {
            self.arguments()?
        } else {
            Vec::new()
        };
        self.make(start, ExprKind::New { class, args })
    }

    fn clone_expression(&mut self, start: usize, token: Token) -> PResult<Box<Expr>> {
        self.bump()?;
        let mut args = self.arguments()?;
        let trailing_comma = self.cursor.retained()[..self.last_end.saturating_sub(1)]
            .iter()
            .rev()
            .find(|t| !t.kind().is_trivia())
            .is_some_and(|t| t.kind() == K::Comma);
        if args.len() == 1
            && !trailing_comma
            && args[0].name.is_none()
            && !args[0].by_ref
            && !args[0].unpack
            && args[0].value.is_some()
        {
            let mut expr = Box::new(args.pop().unwrap().value.unwrap());
            // The ordinary `(expr)` form is a unary operand, so following
            // dereferences bind inside clone. Other argument lists are calls.
            let open = (start + 1..self.last_end)
                .find(|&i| self.cursor.retained()[i].kind() == K::LeftParen)
                .unwrap();
            expr.meta.parenthesize(&self.meta(open));
            while matches!(
                self.kind()?,
                K::LeftParen
                    | K::LeftBracket
                    | K::ObjectArrow
                    | K::NullsafeArrow
                    | K::DoubleColon
                    | K::Increment
                    | K::Decrement
            ) {
                expr = self.postfix(open, expr, true)?;
            }
            self.make(
                start,
                ExprKind::Unary {
                    op: UnaryOp::Clone,
                    expr,
                },
            )
        } else {
            self.record(Diagnostic::error(
                D::UnsupportedSyntax,
                token.span(),
                "expanded clone calls require PHP 8.5, beyond the PHP 8.4 ceiling",
            ))?;
            let name = self.name(start, token)?;
            self.make(
                start,
                ExprKind::Call {
                    target: Target::Name(name),
                    args,
                },
            )
        }
    }
    fn include_expression(&mut self, start: usize, kind: K) -> PResult<Box<Expr>> {
        self.bump()?;
        let expr = self.expression(2)?;
        let kind = match kind {
            K::Include => IncludeKind::Include,
            K::IncludeOnce => IncludeKind::IncludeOnce,
            K::Require => IncludeKind::Require,
            _ => IncludeKind::RequireOnce,
        };
        self.make(start, ExprKind::Include { kind, expr })
    }

    fn parenthesized_construct(&mut self, start: usize, kind: K) -> PResult<Box<Expr>> {
        self.bump()?;
        self.expect(K::LeftParen)?;
        let expr = self.expression(0)?;
        self.expect(K::RightParen)?;
        self.make(
            start,
            ExprKind::Unary {
                op: if kind == K::Eval {
                    UnaryOp::Eval
                } else {
                    UnaryOp::Empty
                },
                expr,
            },
        )
    }

    fn isset_expression(&mut self, start: usize) -> PResult<Box<Expr>> {
        self.bump()?;
        self.expect(K::LeftParen)?;
        let mut values = vec![*self.expression(0)?];
        while self.eat(K::Comma)? {
            if self.kind()? == K::RightParen {
                break;
            }
            values.push(*self.expression(0)?);
        }
        self.expect(K::RightParen)?;
        self.make(start, ExprKind::Isset(values))
    }

    fn exit_expression(&mut self, start: usize, token: Token) -> PResult<Box<Expr>> {
        self.bump()?;
        let mut args = if self.kind()? == K::LeftParen {
            self.arguments()?
        } else {
            Vec::new()
        };
        let simple = args.len() <= 1
            && args
                .iter()
                .all(|a| a.value.is_some() && a.name.is_none() && !a.by_ref && !a.unpack);
        if simple {
            let value = args.pop().and_then(|arg| arg.value).map(Box::new);
            self.make(start, ExprKind::Exit(value))
        } else {
            let name = self.name(start, token)?;
            self.make(
                start,
                ExprKind::Call {
                    target: Target::Name(name),
                    args,
                },
            )
        }
    }

    fn yield_expression(&mut self, start: usize) -> PResult<Box<Expr>> {
        self.bump()?;
        let kind = self.kind()?;
        let no_operand = matches!(kind, K::Question | K::Instanceof)
            || operators::binary(kind, self.options.version).is_some()
                && !matches!(kind, K::Plus | K::Minus);
        let first = if operators::boundary(kind) || no_operand {
            None
        } else {
            Some(self.expression(20)?)
        };
        let (key, value) = if self.eat(K::DoubleArrow)? {
            (first, Some(self.expression(18)?))
        } else {
            (None, first)
        };
        self.make(start, ExprKind::Yield { key, value })
    }
    pub(super) fn variable(&mut self) -> PResult<Box<Expr>> {
        self.enter()?;
        let result = self.variable_inner();
        self.depth -= 1;
        result
    }
    fn variable_inner(&mut self) -> PResult<Box<Expr>> {
        let (start, token) = self.bump()?;
        if token.kind() == K::Variable {
            let bytes = self.bytes(token)[1..].to_vec();
            let name = self.identifier(start, &bytes)?;
            return self.make(start, ExprKind::Variable(name));
        }
        if token.kind() != K::Dollar {
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "expected variable",
            ))?;
            return self.make(start, ExprKind::Error);
        }
        let name = if self.eat(K::LeftBrace)? {
            let expr = self.expression(0)?;
            self.expect(K::RightBrace)?;
            expr
        } else if matches!(self.kind()?, K::Dollar | K::Variable) {
            self.variable()?
        } else {
            let (_, found) = self.peek()?;
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                found.span(),
                "expected variable after dollar sign",
            ))?;
            self.expression(81)?
        };
        self.make(start, ExprKind::DynamicVariable(name))
    }
    fn class_target(&mut self) -> PResult<Target> {
        self.enter()?;
        let result = self.class_target_inner();
        self.depth -= 1;
        result
    }
    fn class_target_inner(&mut self) -> PResult<Target> {
        let (start, token) = self.peek()?;
        let mut expr = match token.kind() {
            K::Identifier
            | K::NameQualified
            | K::NameFullyQualified
            | K::NameRelative
            | K::Static => {
                self.bump()?;
                let name = self.name(start, token)?;
                self.make(start, ExprKind::ConstFetch(name))?
            }
            K::Variable | K::Dollar => self.variable()?,
            K::LeftParen => self.grouped(start)?,
            _ => {
                self.record(Diagnostic::error(
                    if matches!(token.kind(), K::Class | K::Attribute) {
                        D::UnsupportedSyntax
                    } else {
                        D::UnexpectedToken
                    },
                    token.span(),
                    "expected class name or dynamic class expression",
                ))?;
                return Err(());
            }
        };
        if self.grouped_expr(&expr) {
            return Ok(self.target(expr));
        }
        loop {
            let next = self.kind()?;
            if next == K::DoubleColon && matches!(self.ahead(1)?, K::Variable | K::Dollar) {
                expr = self.postfix(start, expr, false)?;
            } else if matches!(expr.kind, ExprKind::ConstFetch(_)) {
                break;
            } else if next == K::LeftBrace && self.options.version < PhpVersion::Php80 {
                expr = self.dimension(start, expr, K::RightBrace)?;
            } else if matches!(next, K::LeftBracket | K::ObjectArrow | K::NullsafeArrow) {
                expr = self.postfix(start, expr, false)?;
            } else {
                break;
            }
        }
        Ok(self.target(expr))
    }
    fn grouped_expr(&self, expr: &Expr) -> bool {
        let Origin::Parsed { tokens, .. } = expr.meta.enclosing_origin() else {
            return false;
        };
        let range = &self.cursor.retained()[tokens.range()];
        if range.first().is_none_or(|t| t.kind() != K::LeftParen) {
            return false;
        }
        let mut nesting = 0;
        for (index, token) in range.iter().enumerate() {
            match token.kind() {
                K::LeftParen => nesting += 1,
                K::RightParen => {
                    nesting -= 1;
                    if nesting == 0 {
                        return index + 1 == range.len();
                    }
                }
                _ => {}
            }
        }
        false
    }
    fn dereferenceable(&self, expr: &Expr, next: K) -> bool {
        if self.grouped_expr(expr) {
            return true;
        }
        if self.writable(expr, false) || matches!(expr.kind, ExprKind::ConstFetch(_)) {
            return true;
        }
        match &expr.kind {
            ExprKind::MagicConstant(_) => matches!(
                next,
                K::LeftBracket | K::LeftBrace | K::ObjectArrow | K::NullsafeArrow
            ),
            ExprKind::String { kind, .. } | ExprKind::Interpolated { kind, .. } => {
                matches!(kind, StringKind::SingleQuoted | StringKind::DoubleQuoted)
            }
            ExprKind::Array { .. } | ExprKind::NewAnonymous(_) => true,
            ExprKind::ClassConst { .. } => next != K::LeftParen,
            ExprKind::New { class, .. } => {
                let class_meta = match class {
                    Target::Name(name) => name.meta(),
                    Target::Expr(expr) => &expr.meta,
                };
                match (expr.meta.origin(), class_meta.enclosing_origin()) {
                    (
                        Origin::Parsed { tokens: whole, .. },
                        Origin::Parsed { tokens: class, .. },
                    ) => whole.end() > class.end(),
                    _ => false,
                }
            }
            _ => false,
        }
    }
    fn target(&self, expr: Box<Expr>) -> Target {
        let grouped = self.grouped_expr(&expr);
        if !grouped {
            if let ExprKind::ConstFetch(name) = expr.kind {
                return Target::Name(name);
            }
        }
        Target::Expr(expr)
    }
    fn member(&mut self) -> PResult<Member> {
        let (start, token) = self.peek()?;
        if matches!(token.kind(), K::Variable | K::Dollar) {
            return Ok(Member::Expr(self.variable()?));
        }
        if self.eat(K::LeftBrace)? {
            let expr = self.expression(0)?;
            self.expect(K::RightBrace)?;
            return Ok(Member::Expr(expr));
        }
        let bytes = self.bytes(token).to_vec();
        if is_identifier(&bytes) {
            self.bump()?;
            return Ok(Member::Identifier(self.identifier(start, &bytes)?));
        }
        self.record(Diagnostic::error(
            D::UnexpectedToken,
            token.span(),
            "expected member name",
        ))?;
        self.alloc()?;
        Ok(Member::Expr(Box::new(Expr {
            meta: self.point(start),
            kind: ExprKind::Error,
        })))
    }
    fn postfix(&mut self, start: usize, left: Box<Expr>, calls: bool) -> PResult<Box<Expr>> {
        let kind = self.kind()?;
        if !self.dereferenceable(&left, kind) {
            self.error_meta(
                D::InvalidContext,
                &left.meta,
                "expression cannot be dereferenced without parentheses",
            )?;
        }
        match kind {
            K::LeftBracket => self.dimension(start, left, K::RightBracket),
            K::LeftParen => {
                let args = self.arguments()?;
                let target = self.target(left);
                self.make(start, ExprKind::Call { target, args })
            }
            K::Increment | K::Decrement => {
                self.bump()?;
                if !self.writable(&left, false) {
                    self.error_meta(D::InvalidContext, &left.meta, "invalid increment target")?;
                }
                self.make(
                    start,
                    ExprKind::Unary {
                        op: if kind == K::Increment {
                            UnaryOp::PostIncrement
                        } else {
                            UnaryOp::PostDecrement
                        },
                        expr: left,
                    },
                )
            }
            K::ObjectArrow | K::NullsafeArrow => {
                self.bump()?;
                let name = self.member()?;
                let nullsafe = kind == K::NullsafeArrow;
                if calls && self.kind()? == K::LeftParen {
                    let args = self.arguments()?;
                    self.make(
                        start,
                        ExprKind::MethodCall {
                            object: left,
                            name,
                            args,
                            nullsafe,
                        },
                    )
                } else {
                    self.make(
                        start,
                        ExprKind::Property {
                            object: left,
                            name,
                            nullsafe,
                        },
                    )
                }
            }
            K::DoubleColon => {
                self.bump()?;
                let variable = matches!(self.kind()?, K::Variable | K::Dollar);
                let mut name = self.member()?;
                let class = self.target(left);
                if calls && self.kind()? == K::LeftParen {
                    let args = self.arguments()?;
                    return self.make(start, ExprKind::StaticCall { class, name, args });
                }
                if variable {
                    if let Member::Expr(expr) = name {
                        name = match *expr {
                            Expr {
                                kind: ExprKind::Variable(id),
                                ..
                            } => Member::Identifier(id),
                            Expr {
                                kind: ExprKind::DynamicVariable(inner),
                                ..
                            } => Member::Expr(inner),
                            other => Member::Expr(Box::new(other)),
                        };
                    }
                    self.make(start, ExprKind::StaticProperty { class, name })
                } else {
                    self.make(start, ExprKind::ClassConst { class, name })
                }
            }
            _ => unreachable!(),
        }
    }
    fn dimension(&mut self, start: usize, left: Box<Expr>, close: K) -> PResult<Box<Expr>> {
        if !self.dereferenceable(&left, K::LeftBracket) {
            self.error_meta(
                D::InvalidContext,
                &left.meta,
                "expression cannot be indexed without parentheses",
            )?;
        }
        self.bump()?;
        let dim = if self.kind()? == close {
            if close == K::RightBrace {
                let meta = self.meta(start);
                self.error_meta(
                    D::InvalidContext,
                    &meta,
                    "curly offset requires an expression",
                )?;
            }
            None
        } else {
            Some(self.expression(0)?)
        };
        self.expect(close)?;
        self.make(start, ExprKind::ArrayDim { array: left, dim })
    }
    pub(super) fn arguments(&mut self) -> PResult<Vec<Argument>> {
        self.expect(K::LeftParen)?;
        let mut args = Vec::new();
        while self.kind()? != K::RightParen && self.kind()? != K::Eof {
            let (start, first_token) = self.peek()?;
            let name = if self.ahead(1)? == K::Colon && is_identifier(self.bytes(first_token)) {
                let (_, token) = self.bump()?;
                let bytes = self.bytes(token).to_vec();
                let name = self.identifier(start, &bytes)?;
                self.bump()?;
                Some(name)
            } else {
                None
            };
            let by_ref = self.eat(K::Ampersand)?;
            let unpack = self.eat(K::Ellipsis)?;
            if (name.is_some() && (by_ref || unpack)) || (by_ref && unpack) {
                let meta = self.meta(start);
                self.error_meta(
                    D::InvalidContext,
                    &meta,
                    "named, reference and unpack argument forms cannot be combined",
                )?;
            }
            let value = if unpack
                && matches!(self.kind()?, K::RightParen | K::Comma)
                && name.is_none()
                && !by_ref
            {
                None
            } else {
                Some(*self.expression(0)?)
            };
            if by_ref
                && value
                    .as_ref()
                    .is_some_and(|expr| !self.writable(expr, false))
            {
                self.error_meta(
                    D::InvalidContext,
                    &value.as_ref().unwrap().meta,
                    "reference argument requires a variable",
                )?;
            }
            self.alloc()?;
            args.push(Argument {
                meta: self.meta(start),
                name,
                by_ref,
                unpack: unpack && value.is_some(),
                value,
            });
            if !self.eat(K::Comma)? {
                break;
            }
        }
        self.expect(K::RightParen)?;
        Ok(args)
    }
    pub(super) fn array(&mut self) -> PResult<Box<Expr>> {
        let (start, token) = self.bump()?;
        let kind = match token.kind() {
            K::LeftBracket => ArrayKind::Short,
            K::List => ArrayKind::List,
            _ => ArrayKind::Long,
        };
        let close = if kind == ArrayKind::Short {
            K::RightBracket
        } else {
            self.expect(K::LeftParen)?;
            K::RightParen
        };
        let mut items = Vec::new();
        while self.kind()? != close && self.kind()? != K::Eof {
            let at = self.peek()?.0;
            if self.kind()? == K::Comma {
                self.bump()?;
                self.alloc()?;
                let value = Expr {
                    meta: self.meta(at),
                    kind: ExprKind::Error,
                };
                self.alloc()?;
                let item = ArrayItem {
                    meta: self.meta(at),
                    key: None,
                    value,
                    by_ref: false,
                    unpack: false,
                };
                items.push(Some(item));
                continue;
            }
            let unpack = self.eat(K::Ellipsis)?;
            let mut by_ref = self.eat(K::Ampersand)?;
            let first = if self.kind()? == K::List {
                self.array()?
            } else {
                self.expression(0)?
            };
            let (key, value) = if self.eat(K::DoubleArrow)? {
                if by_ref || unpack {
                    self.error_meta(
                        D::InvalidContext,
                        &first.meta,
                        "array keys cannot be references or unpacked expressions",
                    )?;
                }
                by_ref = self.eat(K::Ampersand)?;
                (
                    Some(*first),
                    if self.kind()? == K::List {
                        self.array()?
                    } else {
                        self.expression(0)?
                    },
                )
            } else {
                (None, first)
            };
            if by_ref && !self.writable(&value, false) {
                self.error_meta(
                    D::InvalidContext,
                    &value.meta,
                    "reference array item requires a variable",
                )?;
            }
            self.alloc()?;
            items.push(Some(ArrayItem {
                meta: self.meta(at),
                key,
                value: *value,
                by_ref,
                unpack,
            }));
            if !self.eat(K::Comma)? {
                break;
            }
            // list() preserves a trailing omitted slot; short/long arrays drop it.
            if kind == ArrayKind::List && self.kind()? == close {
                items.push(None);
            }
        }
        self.expect(close)?;
        let mut expr = self.make(
            start,
            if kind == ArrayKind::List {
                ExprKind::List { items, kind }
            } else {
                ExprKind::Array { items, kind }
            },
        )?;
        if kind == ArrayKind::List {
            Self::pattern(&mut expr);
        }
        Ok(expr)
    }
    pub(super) fn pattern(expr: &mut Expr) {
        match &mut expr.kind {
            ExprKind::Array {
                items,
                kind: ArrayKind::Short,
            }
            | ExprKind::List { items, .. } => {
                for item in items.iter_mut() {
                    if item
                        .as_ref()
                        .is_some_and(|item| matches!(item.value.kind, ExprKind::Error))
                    {
                        *item = None;
                    } else if let Some(item) = item {
                        Self::pattern(&mut item.value);
                    }
                }
                if let ExprKind::Array { items, kind } = &mut expr.kind {
                    expr.kind = ExprKind::List {
                        items: std::mem::take(items),
                        kind: *kind,
                    };
                }
            }
            _ => {}
        }
    }
    fn match_expression(&mut self) -> PResult<Box<Expr>> {
        let start = self.bump()?.0;
        self.expect(K::LeftParen)?;
        let condition = self.expression(0)?;
        self.expect(K::RightParen)?;
        self.expect(K::LeftBrace)?;
        let mut arms = Vec::new();
        while !matches!(self.kind()?, K::RightBrace | K::Eof) {
            let at = self.peek()?.0;
            let conditions = if self.eat(K::Default)? {
                self.eat(K::Comma)?;
                None
            } else {
                let mut values = vec![*self.expression(0)?];
                while self.eat(K::Comma)? {
                    if self.kind()? == K::DoubleArrow {
                        break;
                    }
                    values.push(*self.expression(0)?);
                }
                Some(values)
            };
            self.expect(K::DoubleArrow)?;
            let body = self.expression(0)?;
            self.alloc()?;
            arms.push(MatchArm {
                meta: self.meta(at),
                conditions,
                body: *body,
            });
            if !self.eat(K::Comma)? {
                break;
            }
        }
        self.expect(K::RightBrace)?;
        self.make(start, ExprKind::Match { condition, arms })
    }
    pub(super) fn writable(&self, expr: &Expr, patterns: bool) -> bool {
        if self.grouped_expr(expr) {
            return false;
        }
        match &expr.kind {
            ExprKind::Variable(_)
            | ExprKind::DynamicVariable(_)
            | ExprKind::StaticProperty { .. }
            | ExprKind::ArrayDim { .. }
            | ExprKind::Property { .. }
            | ExprKind::Call { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::StaticCall { .. } => true,
            ExprKind::List { .. } => patterns,
            _ => false,
        }
    }
}
pub(super) fn is_identifier(bytes: &[u8]) -> bool {
    bytes
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_' || *b >= 128)
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b >= 128)
}
