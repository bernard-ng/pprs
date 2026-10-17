//! Recovering parser for expressions, statements, callables and declarations, with finite resource budgets.
// Grammar/semantics adapted from nikic/PHP-Parser, Copyright (c) 2011 Nikita Popov.
// See THIRD_PARTY_NOTICES.md and reference/LICENSE.nikic-PHP-Parser (BSD-3-Clause).
mod callables;
mod declarations;
mod expression;
mod namespaces;
mod operators;
mod statements;
mod strings;
use crate::halt::{self, HaltError, HaltScope};
use crate::{
    ParseOptions, ParseResult, ParsedFile, SourceText,
    ast::{
        Expr, ExprKind, Identifier, Name, NodeMeta, Origin, Program, Qualification, Stmt, StmtKind,
    },
    cursor::{CursorError, TokenCursor},
    diagnostic::{Diagnostic, DiagnosticCode as D},
    lexer::{InputTooLarge, LexStop, scanner::Scanner},
    options::Resource,
    outcome::Completion,
    token::{Token, TokenKind as K},
};

/// Strict parse failure, including preflight rejection before source allocation.
#[derive(Debug, thiserror::Error)]
pub enum StrictError {
    /// Input exceeds its configured byte budget.
    #[error(transparent)]
    Input(#[from] InputTooLarge),
    /// Syntax, lexical, contextual or resource failure retaining the parse result.
    #[error(transparent)]
    Parse(#[from] crate::ParseError),
}
/// Parse a whole file, starting outside PHP mode. Untagged input is inline HTML;
/// use `<?php` or `<?=` to enter PHP mode. There is no snippet/expression entry point.
///
/// Parse owned PHP syntax with recovery. Source, tokens and any partial tree are
/// retained; inspect status/diagnostics or call `into_strict()` before use as valid PHP.
/// PHP 7.0–8.4 grammar, source fidelity and reviewed malformed-input recovery are
/// implemented. Recovery trees and diagnostic wording may differ from PHP-Parser.
/// This is not a PHP compiler validator.
///
/// # Errors
/// Returns `InputTooLarge` before copying/indexing input beyond its byte budget.
pub fn parse(input: impl AsRef<[u8]>, options: ParseOptions) -> Result<ParseResult, InputTooLarge> {
    let bytes = input.as_ref();
    check_input(bytes.len(), options)?;
    parse_source(SourceText::from(bytes), options)
}
/// Parse shared source storage without copying bytes or line indices.
///
/// # Errors
/// Returns `InputTooLarge` when the source exceeds its byte budget.
pub fn parse_source(
    source: SourceText,
    options: ParseOptions,
) -> Result<ParseResult, InputTooLarge> {
    check_input(source.len(), options)?;
    Ok(Parser::new(source, options).run())
}
/// Require parsing without recovery, error diagnostics or resource aborts.
///
/// # Errors
/// Retains the recovering parse result on parse failure, or returns a preflight size error.
pub fn parse_strict(
    input: impl AsRef<[u8]>,
    options: ParseOptions,
) -> Result<ParsedFile, StrictError> {
    Ok(parse(input, options)?.into_strict()?)
}
fn check_input(length: usize, options: ParseOptions) -> Result<(), InputTooLarge> {
    let limit = options.limits.get(Resource::InputBytes);
    if length > limit {
        Err(InputTooLarge { length, limit })
    } else {
        Ok(())
    }
}
type PResult<T> = Result<T, ()>;
struct Parser {
    source: SourceText,
    cursor: TokenCursor<Scanner>,
    options: ParseOptions,
    diagnostics: Vec<Diagnostic>,
    aborted: bool,
    nodes: usize,
    depth: usize,
    block_depth: usize,
    last_end: usize,
}
impl Parser {
    fn new(source: SourceText, options: ParseOptions) -> Self {
        let scanner = Scanner::new(source.clone(), options);
        let mut cursor = TokenCursor::new(source.clone(), scanner, options.limits);
        cursor.skip_invalid_bytes();
        Self {
            cursor,
            source,
            options,
            diagnostics: Vec::new(),
            aborted: false,
            nodes: 1,
            depth: 1,
            block_depth: 0,
            last_end: 0,
        }
    }
    fn run(mut self) -> ParseResult {
        let mut statements = Vec::new();
        while !self.aborted {
            match self.peek() {
                Ok((index, t)) if t.kind() == K::Eof => {
                    if let Ok(Some(stmt)) = self.trailing_nop(index) {
                        statements.push(stmt);
                    }
                    break;
                }
                Err(()) => break,
                _ => {}
            }
            let before = self.cursor.position();
            match self.statement() {
                Ok(Some(stmt)) => statements.push(stmt),
                Err(()) if !self.aborted => {
                    self.recover_statement();
                }
                Ok(None) | Err(()) => {}
            }
            if !self.aborted && self.cursor.position() == before {
                let _ = self.bump();
            }
        }
        let mut program = Program {
            statements: self.normalize_namespaces(statements),
        };
        if crate::visit::walk(&program)
            .any(|entry| entry.depth > self.options.limits.get(Resource::Depth).min(128))
        {
            let _ = self.limit(Resource::Depth);
            program.statements.clear();
        }
        // Array-hole diagnostics are delayed until assignment has converted patterns.
        let holes: Vec<_> = crate::visit::walk(&program)
            .filter_map(|entry| match entry.node {
                crate::visit::Node::Expr(Expr {
                    kind: ExprKind::Array { items, .. },
                    ..
                }) => Some(
                    items
                        .iter()
                        .flatten()
                        .filter(|item| matches!(item.value.kind, ExprKind::Error))
                        .map(|item| item.meta.clone())
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        for meta in holes {
            if self
                .error_meta(
                    D::InvalidContext,
                    &meta,
                    "Cannot use empty array elements in arrays",
                )
                .is_err()
            {
                break;
            }
        }
        crate::ast::normalize_comments(&mut program);
        let completion = if self.aborted {
            Completion::Aborted(Some(program))
        } else {
            Completion::Complete(program)
        };
        let tokens = self
            .cursor
            .abort()
            .expect("parser retains contiguous lexer tokens");
        ParseResult::assemble(tokens, completion, self.diagnostics)
    }
    fn record(&mut self, mut diagnostic: Diagnostic) -> PResult<()> {
        if self.aborted {
            return Err(());
        }
        // A grammar diagnostic anchored to one retained token always identifies it.
        // Context/literal ranges may cover a complete construct and have no found token.
        if diagnostic.code == D::UnexpectedToken {
            if diagnostic.found.is_none() {
                diagnostic.found = self
                    .cursor
                    .retained()
                    .iter()
                    .rev()
                    .find(|token| token.span() == diagnostic.primary)
                    .map(|token| token.kind());
            }
            // One unexpected token can unwind several productions. Keep the first,
            // most local expectation; do not suppress independent contextual checks.
            if self.diagnostics.iter().rev().any(|previous| {
                previous.code == D::UnexpectedToken && previous.primary == diagnostic.primary
            }) {
                return Ok(());
            }
        }
        let limit = self.options.limits.get(Resource::Diagnostics);
        if self.diagnostics.len() >= limit - 1 && !matches!(diagnostic.code, D::ResourceLimit(_)) {
            self.diagnostics.push(Diagnostic::error(
                D::ResourceLimit(Resource::Diagnostics),
                diagnostic.primary,
                "diagnostic budget exhausted",
            ));
            self.aborted = true;
            return Err(());
        }
        if self.diagnostics.len() < limit {
            self.diagnostics.push(diagnostic);
        }
        if self
            .diagnostics
            .last()
            .is_some_and(|d| matches!(d.code, D::ResourceLimit(_)))
        {
            self.aborted = true;
            return Err(());
        }
        Ok(())
    }
    fn cursor_result(
        &mut self,
        result: Result<(usize, Token), CursorError>,
    ) -> PResult<(usize, Token)> {
        for diagnostic in self.cursor.take_diagnostics() {
            self.record(diagnostic)?;
        }
        match result {
            Ok(value) => Ok(value),
            Err(CursorError::TokenLimit) => {
                self.limit(Resource::Tokens)?;
                Err(())
            }
            Err(CursorError::Lexical(LexStop::Limit(resource))) => {
                self.limit(resource)?;
                Err(())
            }
            Err(error) => panic!("private parser/cursor invariant: {error}"),
        }
    }
    fn peek(&mut self) -> PResult<(usize, Token)> {
        if self.aborted {
            return Err(());
        }
        let result = self.cursor.peek(0);
        self.cursor_result(result)
    }
    fn kind(&mut self) -> PResult<K> {
        Ok(self.peek()?.1.kind())
    }
    fn ahead(&mut self, distance: usize) -> PResult<K> {
        if self.aborted {
            return Err(());
        }
        let result = self.cursor.peek(distance);
        Ok(self.cursor_result(result)?.1.kind())
    }
    fn bump(&mut self) -> PResult<(usize, Token)> {
        if self.aborted {
            return Err(());
        }
        let result = self.cursor.bump();
        let value = self.cursor_result(result)?;
        self.last_end = value.0 + usize::from(value.1.kind() != K::Eof);
        Ok(value)
    }
    fn eat(&mut self, kind: K) -> PResult<bool> {
        if self.kind()? == kind {
            self.bump()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn syntax(&mut self, token: Token, expected: &[K], message: impl Into<String>) -> PResult<()> {
        let mut diagnostic = Diagnostic::error(D::UnexpectedToken, token.span(), message);
        diagnostic.expected = expected.to_vec();
        diagnostic.found = Some(token.kind());
        self.record(diagnostic)
    }
    fn expect(&mut self, kind: K) -> PResult<(usize, Token)> {
        let next = self.peek()?;
        if next.1.kind() == kind {
            self.bump()
        } else {
            let mut d = Diagnostic::error(
                D::UnexpectedToken,
                next.1.span(),
                format!("expected {kind:?}, found {:?}", next.1.kind()),
            );
            d.expected.push(kind);
            d.found = Some(next.1.kind());
            let opening = match kind {
                K::RightParen => Some(K::LeftParen),
                K::RightBracket => Some(K::LeftBracket),
                K::RightBrace => Some(K::LeftBrace),
                _ => None,
            };
            if let Some(opening) = opening {
                let mut closed = 0;
                for token in self.cursor.retained()[..next.0].iter().rev() {
                    if token.kind() == kind {
                        closed += 1;
                    } else if token.kind() == opening
                        || kind == K::RightBrace
                            && matches!(token.kind(), K::CurlyOpen | K::DollarOpenCurly)
                        || kind == K::RightBracket && token.kind() == K::Attribute
                    {
                        if closed == 0 {
                            d.secondary.push(crate::diagnostic::Label {
                                span: token.span(),
                                message: "opening delimiter".into(),
                            });
                            break;
                        }
                        closed -= 1;
                    }
                }
            }
            self.record(d)?;
            Err(())
        }
    }
    fn limit(&mut self, resource: Resource) -> PResult<()> {
        let pos = self.cursor.scanned_end();
        self.record(Diagnostic::error(
            D::ResourceLimit(resource),
            self.source.span(pos..pos).unwrap(),
            format!("{resource} budget exhausted"),
        ))
    }
    fn alloc(&mut self) -> PResult<()> {
        if self.nodes >= self.options.limits.get(Resource::Nodes) {
            return self.limit(Resource::Nodes);
        }
        self.nodes += 1;
        Ok(())
    }
    fn enter(&mut self) -> PResult<()> {
        // Bound the native stack even when a caller requests an excessive depth.
        if self.depth >= self.options.limits.get(Resource::Depth).min(128) {
            return self.limit(Resource::Depth);
        }
        self.depth += 1;
        Ok(())
    }
    fn meta(&self, start: usize) -> NodeMeta {
        NodeMeta::prefix(
            &self.source,
            self.cursor.retained(),
            start,
            self.last_end.max(start),
        )
    }
    fn point(&self, index: usize) -> NodeMeta {
        NodeMeta::prefix(&self.source, self.cursor.retained(), index, index)
    }
    fn error_meta(&mut self, code: D, meta: &NodeMeta, message: &str) -> PResult<()> {
        let Origin::Parsed { span, .. } = meta.origin() else {
            unreachable!()
        };
        self.record(Diagnostic::error(code, span, message))
    }
    fn make(&mut self, start: usize, kind: ExprKind) -> PResult<Box<Expr>> {
        self.alloc()?;
        let expr = Box::new(Expr {
            meta: self.meta(start),
            kind,
        });
        if crate::visit::walk_node(crate::visit::Node::Expr(&expr)).any(|entry| {
            entry.depth
                > self
                    .options
                    .limits
                    .get(Resource::Depth)
                    .min(128)
                    .saturating_sub(self.block_depth + 2)
        }) {
            self.limit(Resource::Depth)?;
        }
        Ok(expr)
    }
    fn bytes(&self, token: Token) -> &[u8] {
        self.source.slice(token.span()).unwrap()
    }
    fn name(&mut self, index: usize, token: Token) -> PResult<Name> {
        self.alloc()?;
        let mut bytes = self.bytes(token);
        let qualification = if bytes.starts_with(b"\\") {
            bytes = &bytes[1..];
            Qualification::FullyQualified
        } else if token.kind() == K::NameRelative {
            bytes = &bytes[10..];
            Qualification::Relative
        } else {
            Qualification::Normal
        };
        Ok(Name::new(
            bytes.split(|b| *b == b'\\').map(Into::into).collect(),
            qualification,
            NodeMeta::prefix(&self.source, self.cursor.retained(), index, index + 1),
        )
        .expect("lexer name components"))
    }
    fn identifier(&mut self, index: usize, bytes: &[u8]) -> PResult<Identifier> {
        self.alloc()?;
        Ok(Identifier::new(bytes, self.meta(index)).expect("parser identifier spelling"))
    }
    fn statement(&mut self) -> PResult<Option<Stmt>> {
        self.enter()?;
        let result = self.statement_inner();
        self.depth -= 1;
        result
    }
    #[allow(clippy::too_many_lines, reason = "statement grammar dispatch")]
    fn statement_inner(&mut self) -> PResult<Option<Stmt>> {
        let (start, token) = self.peek()?;
        let kind = match token.kind() {
            K::InlineHtml => {
                self.bump()?;
                StmtKind::InlineHtml(self.bytes(token).into())
            }
            K::Semicolon | K::CloseTag => {
                self.bump()?;
                if self.meta(start).comments().is_empty() {
                    return Ok(None);
                }
                StmtKind::Nop
            }
            K::Echo | K::OpenEchoTag => {
                self.bump()?;
                let mut values = vec![*self.expression(0)?];
                while self.eat(K::Comma)? {
                    values.push(*self.expression(0)?);
                }
                self.terminate()?;
                StmtKind::Echo(values)
            }
            K::Return => {
                self.bump()?;
                let value = if matches!(self.kind()?, K::Semicolon | K::CloseTag | K::Eof) {
                    None
                } else {
                    Some(*self.expression(0)?)
                };
                self.terminate()?;
                StmtKind::Return(value)
            }
            K::LeftBrace => {
                self.bump()?;
                self.enter()?;
                self.block_depth += 1;
                let result = self.block();
                self.block_depth -= 1;
                self.depth -= 1;
                StmtKind::Block(result?)
            }
            K::HaltCompiler => {
                let result = halt::consume(
                    &mut self.cursor,
                    if self.block_depth == 0 {
                        HaltScope::Outermost
                    } else {
                        HaltScope::Nested
                    },
                );
                for diagnostic in self.cursor.take_diagnostics() {
                    self.record(diagnostic)?;
                }
                self.last_end = self.cursor.position();
                match result {
                    Ok(()) => {
                        let halt_end = self.last_end;
                        let tail = if self.kind()? == K::HaltTail {
                            let (_, t) = self.bump()?;
                            self.bytes(t).into()
                        } else {
                            crate::ByteString::default()
                        };
                        self.last_end = halt_end;
                        StmtKind::HaltCompiler(tail)
                    }
                    Err(HaltError::Cursor(error)) => {
                        self.cursor_result(Err(error))?;
                        unreachable!()
                    }
                    Err(HaltError::Scope) => {
                        self.record(Diagnostic::error(
                            D::InvalidContext,
                            token.span(),
                            "__HALT_COMPILER() can only be used from the outermost scope",
                        ))?;
                        self.aborted = true;
                        return Err(());
                    }
                    Err(HaltError::Syntax(expected)) => {
                        let (_, found) = self.peek()?;
                        let expected = if expected == K::Semicolon {
                            vec![K::Semicolon, K::CloseTag]
                        } else {
                            vec![expected]
                        };
                        self.syntax(found, &expected, "expected __halt_compiler();")?;
                        return Err(());
                    }
                }
            }
            K::If
            | K::While
            | K::Do
            | K::For
            | K::Foreach
            | K::Switch
            | K::Try
            | K::Declare
            | K::Global
            | K::Unset
            | K::Break
            | K::Continue
            | K::Goto => self.procedural(token.kind())?,
            K::Static if self.ahead(1)? == K::Variable => self.procedural(K::Static)?,
            K::Function
                if !(matches!(self.ahead(1)?, K::LeftParen)
                    || self.ahead(1)? == K::Ampersand && self.ahead(2)? == K::LeftParen) =>
            {
                self.function_statement(Vec::new())?
            }
            K::Identifier if self.ahead(1)? == K::Colon => {
                let name = self.plain_identifier()?;
                self.bump()?;
                StmtKind::Label(name)
            }
            K::Class | K::Interface | K::Trait | K::Enum | K::Abstract | K::Final | K::Readonly
                if token.kind() != K::Readonly || self.ahead(1)? != K::LeftParen =>
            {
                StmtKind::Class(self.class_declaration(Vec::new(), false)?.0)
            }
            K::Namespace | K::Use | K::Const => {
                if self.block_depth > 0 {
                    self.record(Diagnostic::error(
                        D::InvalidContext,
                        token.span(),
                        "declaration requires top-level scope",
                    ))?;
                }
                match token.kind() {
                    K::Namespace => self.namespace_statement()?,
                    K::Use => self.use_statement()?,
                    _ => self.constants_statement(
                        Vec::new(),
                        crate::ast::Modifiers::default(),
                        false,
                    )?,
                }
            }
            K::Attribute => self.attributed_statement()?,
            _ => {
                let expr = self.expression(0)?;
                self.terminate()?;
                StmtKind::Expression(*expr)
            }
        };
        self.alloc()?;
        let stmt = Stmt {
            meta: self.meta(start),
            kind,
        };
        if crate::visit::walk_node(crate::visit::Node::Stmt(&stmt)).any(|entry| {
            entry.depth
                > self
                    .options
                    .limits
                    .get(Resource::Depth)
                    .min(128)
                    .saturating_sub(self.block_depth + 1)
        }) {
            self.limit(Resource::Depth)?;
        }
        Ok(Some(stmt))
    }
    fn block(&mut self) -> PResult<Vec<Stmt>> {
        let body = self.statement_list(&[K::RightBrace], true)?;
        self.expect(K::RightBrace)?;
        Ok(body)
    }
    fn trailing_nop(&mut self, index: usize) -> PResult<Option<Stmt>> {
        let before = self.point(index);
        let Some(&last) = before.comments().last() else {
            return Ok(None);
        };
        let meta = self.point(last + 1);
        self.enter()?;
        let allocated = self.alloc();
        self.depth -= 1;
        allocated?;
        Ok(Some(Stmt {
            meta,
            kind: StmtKind::Nop,
        }))
    }
    fn terminate(&mut self) -> PResult<()> {
        if matches!(self.kind()?, K::Semicolon | K::CloseTag) {
            self.bump()?;
            Ok(())
        } else {
            let (_, token) = self.peek()?;
            self.syntax(
                token,
                &[K::Semicolon, K::CloseTag],
                "expected statement terminator",
            )?;
            if matches!(
                token.kind(),
                K::Identifier
                    | K::Variable
                    | K::Integer
                    | K::Float
                    | K::QuotedString
                    | K::Echo
                    | K::Return
                    | K::Use
                    | K::Namespace
                    | K::Const
                    | K::Class
                    | K::Interface
                    | K::Trait
                    | K::Enum
                    | K::Function
                    | K::Attribute
                    | K::RightBrace
                    | K::Eof
            ) {
                Ok(())
            } else {
                Err(())
            }
        }
    }
    /// Unambiguous starts that a damaged preceding statement must not consume.
    fn statement_anchor(kind: K) -> bool {
        matches!(
            kind,
            K::Echo
                | K::OpenEchoTag
                | K::Return
                | K::InlineHtml
                | K::Namespace
                | K::Class
                | K::Interface
                | K::Trait
                | K::Enum
                | K::Function
                | K::Const
                | K::If
                | K::While
                | K::For
                | K::Foreach
                | K::Switch
                | K::Try
                | K::Declare
                | K::Attribute
        )
    }
    fn recover_statement(&mut self) {
        let mut delimiters = Vec::new();
        while !self.aborted {
            let Ok(kind) = self.kind() else {
                break;
            };
            if kind == K::Eof {
                break;
            }
            if delimiters.is_empty() {
                if Self::clause_boundary(kind) {
                    break;
                }
                if kind == K::RightBrace && self.block_depth > 0 {
                    break;
                }
                if matches!(kind, K::Semicolon | K::CloseTag) {
                    let _ = self.bump();
                    break;
                }
                if Self::statement_anchor(kind) {
                    break;
                }
            }
            match kind {
                K::LeftParen => delimiters.push(K::RightParen),
                K::LeftBracket => delimiters.push(K::RightBracket),
                K::LeftBrace => delimiters.push(K::RightBrace),
                K::RightParen | K::RightBracket | K::RightBrace
                    if delimiters.last() == Some(&kind) =>
                {
                    delimiters.pop();
                }
                _ => {}
            }
            if delimiters.len() > self.options.limits.get(Resource::Depth).min(128) {
                let _ = self.limit(Resource::Depth);
                break;
            }
            if self.bump().is_err() {
                break;
            }
            if kind == K::RightBrace && delimiters.is_empty() {
                break;
            }
        }
    }
}
