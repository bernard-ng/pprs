//! Procedural grammar and delimiter-aware statement bodies.
use super::{PResult, Parser};
use crate::{
    ast::{
        Case, Catch, ConditionalLoop, DeclareItem, DeclareStatement, Else, ElseIf, Expr, Finally,
        ForStatement, ForeachStatement, IfStatement, StaticVariable, Stmt, StmtKind,
        SwitchStatement, TryStatement,
    },
    diagnostic::{Diagnostic, DiagnosticCode as D},
    token::TokenKind as K,
};
impl Parser {
    pub(super) fn procedural(&mut self, kind: K) -> PResult<StmtKind> {
        match kind {
            K::If => self.if_statement(),
            K::While | K::Do => self.conditional_loop(kind),
            K::For => self.for_statement(),
            K::Foreach => self.foreach_statement(),
            K::Switch => self.switch_statement(),
            K::Try => self.try_statement(),
            K::Declare => self.declare_statement(),
            K::Global | K::Static | K::Unset => self.bindings(kind),
            K::Break | K::Continue => {
                self.bump()?;
                let value = if matches!(self.kind()?, K::Semicolon | K::CloseTag | K::Eof) {
                    None
                } else {
                    Some(*self.expression(0)?)
                };
                self.terminate()?;
                Ok(if kind == K::Break {
                    StmtKind::Break(value)
                } else {
                    StmtKind::Continue(value)
                })
            }
            K::Goto => {
                self.bump()?;
                let name = self.plain_identifier()?;
                self.terminate()?;
                Ok(StmtKind::Goto(name))
            }
            _ => unreachable!("procedural dispatch"),
        }
    }
    fn condition(&mut self) -> PResult<Box<Expr>> {
        self.expect(K::LeftParen)?;
        let expr = self.expression(0)?;
        self.expect(K::RightParen)?;
        Ok(expr)
    }
    /// A nested body always prevents halt activation, including a single unbraced statement.
    pub(super) fn nested_body(&mut self, end: Option<K>) -> PResult<Vec<Stmt>> {
        self.enter()?;
        self.block_depth += 1;
        let result = self.nested_body_inner(end);
        self.block_depth -= 1;
        self.depth -= 1;
        result
    }
    fn nested_body_inner(&mut self, end: Option<K>) -> PResult<Vec<Stmt>> {
        if let Some(end) = end {
            if self.eat(K::Colon)? {
                let body = self.statement_list(&[end], true)?;
                self.expect(end)?;
                self.terminate()?;
                return Ok(body);
            }
        }
        if self.eat(K::LeftBrace)? {
            return self.block();
        }
        if self.kind()? == K::Function
            && !(self.ahead(1)? == K::LeftParen
                || self.ahead(1)? == K::Ampersand && self.ahead(2)? == K::LeftParen)
        {
            let (_, token) = self.peek()?;
            self.record(Diagnostic::error(
                D::UnexpectedToken,
                token.span(),
                "function declaration requires a statement list or braced body",
            ))?;
        }
        Ok(match self.statement()? {
            Some(Stmt {
                kind: StmtKind::Block(body),
                ..
            }) => body,
            Some(stmt) => vec![stmt],
            None => Vec::new(),
        })
    }
    pub(super) fn braced_body(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(K::LeftBrace)?;
        self.enter()?;
        self.block_depth += 1;
        let result = self.block();
        self.block_depth -= 1;
        self.depth -= 1;
        result
    }
    /// Stops remain visible to the owner; recovery never eats an enclosing clause keyword.
    pub(super) fn statement_list(&mut self, stops: &[K], trailing: bool) -> PResult<Vec<Stmt>> {
        let mut body = Vec::new();
        loop {
            let kind = self.kind()?;
            if kind == K::Eof || stops.contains(&kind) {
                break;
            }
            if Self::clause_boundary(kind) {
                let (_, token) = self.bump()?;
                self.record(Diagnostic::error(
                    D::UnexpectedToken,
                    token.span(),
                    "unexpected clause boundary",
                ))?;
                continue;
            }
            let before = self.cursor.position();
            match self.statement() {
                Ok(Some(stmt)) => body.push(stmt),
                Ok(None) => {}
                Err(()) if !self.aborted => self.recover_statement(),
                Err(()) => return Err(()),
            }
            if !self.aborted && self.cursor.position() == before {
                self.bump()?;
            }
        }
        if trailing && self.kind()? != K::Eof {
            let index = self.peek()?.0;
            if let Some(stmt) = self.trailing_nop(index)? {
                body.push(stmt);
            }
        }
        Ok(body)
    }
    pub(super) fn clause_boundary(kind: K) -> bool {
        matches!(
            kind,
            K::Else
                | K::ElseIf
                | K::EndIf
                | K::EndWhile
                | K::EndFor
                | K::EndForeach
                | K::EndSwitch
                | K::EndDeclare
                | K::Catch
                | K::Finally
                | K::Case
                | K::Default
                | K::RightBrace
        )
    }
    fn alternate_branch(&mut self, stops: &[K]) -> PResult<Vec<Stmt>> {
        self.expect(K::Colon)?;
        self.enter()?;
        self.block_depth += 1;
        let result = self.statement_list(stops, true);
        self.block_depth -= 1;
        self.depth -= 1;
        result
    }
    fn if_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        let condition = self.condition()?;
        let alternate = self.kind()? == K::Colon;
        let stops = [K::ElseIf, K::Else, K::EndIf];
        let body = if alternate {
            self.alternate_branch(&stops)?
        } else {
            self.nested_body(None)?
        };
        let mut elseifs = Vec::new();
        while self.kind()? == K::ElseIf {
            let start = self.bump()?.0;
            let condition = self.condition()?;
            let body = if alternate {
                self.alternate_branch(&stops)?
            } else {
                self.nested_body(None)?
            };
            self.alloc()?;
            elseifs.push(ElseIf {
                meta: self.meta(start),
                condition,
                body,
            });
        }
        let otherwise = if self.kind()? == K::Else {
            let start = self.bump()?.0;
            let body = if alternate {
                self.alternate_branch(&[K::EndIf])?
            } else {
                self.nested_body(None)?
            };
            self.alloc()?;
            Some(Else {
                meta: self.meta(start),
                body,
            })
        } else {
            None
        };
        if alternate {
            self.expect(K::EndIf)?;
            self.terminate()?;
        }
        Ok(StmtKind::If(Box::new(IfStatement {
            condition,
            body,
            elseifs,
            otherwise,
        })))
    }
    fn conditional_loop(&mut self, kind: K) -> PResult<StmtKind> {
        self.bump()?;
        let (condition, body) = if kind == K::While {
            let condition = self.condition()?;
            (condition, self.nested_body(Some(K::EndWhile))?)
        } else {
            let body = self.nested_body(None)?;
            self.expect(K::While)?;
            let condition = self.condition()?;
            self.terminate()?;
            (condition, body)
        };
        let payload = Box::new(ConditionalLoop { condition, body });
        Ok(if kind == K::While {
            StmtKind::While(payload)
        } else {
            StmtKind::Do(payload)
        })
    }
    fn expression_list(&mut self, end: K) -> PResult<Vec<Expr>> {
        let mut values = Vec::new();
        if self.kind()? != end {
            loop {
                values.push(*self.expression(0)?);
                if !self.eat(K::Comma)? {
                    break;
                }
            }
        }
        self.expect(end)?;
        Ok(values)
    }
    fn for_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        self.expect(K::LeftParen)?;
        let init = self.expression_list(K::Semicolon)?;
        let conditions = self.expression_list(K::Semicolon)?;
        let steps = self.expression_list(K::RightParen)?;
        let body = self.nested_body(Some(K::EndFor))?;
        Ok(StmtKind::For(Box::new(ForStatement {
            init,
            conditions,
            steps,
            body,
        })))
    }
    fn foreach_target(&mut self) -> PResult<Box<Expr>> {
        let mut target = if self.kind()? == K::List {
            self.array()?
        } else {
            self.expression(0)?
        };
        Self::pattern(&mut target);
        if !self.writable(&target, true) {
            self.error_meta(D::InvalidContext, &target.meta, "invalid foreach target")?;
        }
        Ok(target)
    }
    fn foreach_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        self.expect(K::LeftParen)?;
        let iterable = self.expression(0)?;
        self.expect(K::As)?;
        let mut by_ref = self.eat(K::Ampersand)?;
        let first = self.foreach_target()?;
        let (key, value) = if self.eat(K::DoubleArrow)? {
            if !self.writable(&first, false) {
                self.error_meta(
                    D::InvalidContext,
                    &first.meta,
                    "foreach key requires a variable",
                )?;
            }
            if by_ref {
                self.error_meta(
                    D::InvalidContext,
                    &first.meta,
                    "foreach key cannot be a reference",
                )?;
            }
            by_ref = self.eat(K::Ampersand)?;
            (Some(first), self.foreach_target()?)
        } else {
            (None, first)
        };
        if by_ref && !self.writable(&value, false) {
            self.error_meta(
                D::InvalidContext,
                &value.meta,
                "foreach reference requires a variable",
            )?;
        }
        self.expect(K::RightParen)?;
        let body = self.nested_body(Some(K::EndForeach))?;
        Ok(StmtKind::Foreach(Box::new(ForeachStatement {
            iterable,
            key,
            value,
            by_ref,
            body,
        })))
    }
    fn switch_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        let condition = self.condition()?;
        let alternate = self.eat(K::Colon)?;
        if !alternate {
            self.expect(K::LeftBrace)?;
        }
        let end = if alternate {
            K::EndSwitch
        } else {
            K::RightBrace
        };
        self.eat(K::Semicolon)?;
        self.enter()?;
        self.block_depth += 1;
        let result = self.switch_cases(end);
        self.block_depth -= 1;
        self.depth -= 1;
        let cases = result?;
        self.expect(end)?;
        if alternate {
            self.terminate()?;
        }
        Ok(StmtKind::Switch(Box::new(SwitchStatement {
            condition,
            cases,
        })))
    }
    fn switch_cases(&mut self, end: K) -> PResult<Vec<Case>> {
        let mut cases = Vec::new();
        while self.kind()? != end && self.kind()? != K::Eof {
            let (start, token) = self.bump()?;
            let condition = match token.kind() {
                K::Case => Some(self.expression(0)?),
                K::Default => None,
                _ => {
                    self.record(Diagnostic::error(
                        D::UnexpectedToken,
                        token.span(),
                        "expected case or default",
                    ))?;
                    self.recover_statement();
                    continue;
                }
            };
            if !self.eat(K::Colon)? {
                self.expect(K::Semicolon)?;
            }
            let body = self.statement_list(&[K::Case, K::Default, end], false)?;
            self.alloc()?;
            cases.push(Case {
                meta: self.meta(start),
                condition,
                body,
            });
        }
        Ok(cases)
    }
    fn try_statement(&mut self) -> PResult<StmtKind> {
        let start = self.bump()?.0;
        let body = self.braced_body()?;
        let mut catches = Vec::new();
        while self.kind()? == K::Catch {
            let at = self.bump()?.0;
            let (types, variable) = match self.catch_header() {
                Ok(header) => header,
                Err(()) if !self.aborted => {
                    // Keep later catches/finally visible, and discard only the damaged clause.
                    while !matches!(
                        self.kind()?,
                        K::LeftBrace | K::Catch | K::Finally | K::RightBrace | K::Eof
                    ) {
                        self.bump()?;
                    }
                    if self.kind()? == K::LeftBrace {
                        self.braced_body()?;
                    }
                    continue;
                }
                Err(()) => return Err(()),
            };
            let body = self.braced_body()?;
            self.alloc()?;
            catches.push(Catch {
                meta: self.meta(at),
                types,
                variable,
                body,
            });
        }
        let finally = if self.kind()? == K::Finally {
            let at = self.bump()?.0;
            let body = self.braced_body()?;
            self.alloc()?;
            Some(Finally {
                meta: self.meta(at),
                body,
            })
        } else {
            None
        };
        if catches.is_empty() && finally.is_none() {
            let meta = self.meta(start);
            self.error_meta(
                D::InvalidContext,
                &meta,
                "cannot use try without catch or finally",
            )?;
        }
        Ok(StmtKind::Try(Box::new(TryStatement {
            body,
            catches,
            finally,
        })))
    }
    fn catch_header(&mut self) -> PResult<(Vec<crate::ast::Name>, Option<Box<Expr>>)> {
        self.expect(K::LeftParen)?;
        let mut types = vec![self.type_name()?];
        while self.eat(K::Pipe)? {
            types.push(self.type_name()?);
        }
        let variable = if self.kind()? == K::Variable {
            Some(self.variable()?)
        } else {
            None
        };
        self.expect(K::RightParen)?;
        Ok((types, variable))
    }
    fn bindings(&mut self, kind: K) -> PResult<StmtKind> {
        self.bump()?;
        if kind == K::Static {
            let mut vars = Vec::new();
            loop {
                let start = self.peek()?.0;
                self.require_simple_variable()?;
                let variable = self.variable()?;
                let default = if self.eat(K::Equal)? {
                    Some(self.expression(0)?)
                } else {
                    None
                };
                self.alloc()?;
                vars.push(StaticVariable {
                    meta: self.meta(start),
                    variable,
                    default,
                });
                if !self.eat(K::Comma)? {
                    break;
                }
            }
            self.terminate()?;
            return Ok(StmtKind::Static(vars));
        }
        if kind == K::Unset {
            self.expect(K::LeftParen)?;
        }
        let mut vars = Vec::new();
        loop {
            let expr = if kind == K::Global {
                self.variable()?
            } else {
                self.expression(0)?
            };
            if !self.writable(&expr, false) {
                self.error_meta(D::InvalidContext, &expr.meta, "expected variable")?;
            }
            vars.push(*expr);
            if !self.eat(K::Comma)? {
                break;
            }
            if kind == K::Unset && self.kind()? == K::RightParen {
                break;
            }
        }
        if kind == K::Unset {
            self.expect(K::RightParen)?;
        }
        self.terminate()?;
        Ok(if kind == K::Global {
            StmtKind::Global(vars)
        } else {
            StmtKind::Unset(vars)
        })
    }
    fn declare_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        self.expect(K::LeftParen)?;
        let mut items = Vec::new();
        loop {
            let start = self.peek()?.0;
            let key = self.plain_identifier()?;
            self.expect(K::Equal)?;
            let value = self.expression(0)?;
            self.alloc()?;
            items.push(DeclareItem {
                meta: self.meta(start),
                key,
                value,
            });
            if !self.eat(K::Comma)? {
                break;
            }
        }
        self.expect(K::RightParen)?;
        let body = if self.eat(K::Semicolon)? || self.eat(K::CloseTag)? {
            None
        } else {
            Some(self.nested_body(Some(K::EndDeclare))?)
        };
        Ok(StmtKind::Declare(Box::new(DeclareStatement {
            items,
            body,
        })))
    }
}
