//! Namespace/import syntax and whole-file namespace grouping.
use super::{PResult, Parser};
use crate::{
    ast::{
        Name, Namespace, NamespaceStyle, NodeMeta, Origin, Qualification, Stmt, StmtKind, UseItem,
        UseKind, UseStatement,
    },
    diagnostic::{Diagnostic, DiagnosticCode as D},
    token::TokenKind as K,
};
impl Parser {
    pub(super) fn namespace_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        let name = if self.kind()? == K::LeftBrace {
            None
        } else {
            let (at, token) = self.peek()?;
            let name = if token.kind() == K::NameQualified {
                self.bump()?;
                self.name(at, token)?
            } else {
                let id = self.reserved_identifier()?;
                // One reference Name node, not an Identifier plus a Name.
                Name::new(
                    vec![id.bytes().clone()],
                    Qualification::Normal,
                    id.meta().clone(),
                )
                .unwrap()
            };
            Some(name)
        };
        let (style, body) = if self.eat(K::LeftBrace)? {
            self.enter()?;
            let result = self.statement_list(&[K::RightBrace], true);
            self.depth -= 1;
            let body = result?;
            self.expect(K::RightBrace)?;
            for stmt in &body {
                if matches!(stmt.kind, StmtKind::Namespace(_)) {
                    self.error_meta(
                        D::InvalidContext,
                        &stmt.meta,
                        "namespace declarations cannot be nested",
                    )?;
                }
            }
            (NamespaceStyle::Braced, Some(body))
        } else {
            self.terminate()?;
            (NamespaceStyle::Semicolon, None)
        };
        Ok(StmtKind::Namespace(Box::new(Namespace {
            name,
            style,
            body,
        })))
    }
    fn import_name(&mut self, allow_leading: bool) -> PResult<Name> {
        let name = self.type_name()?;
        if name.qualification() == Qualification::Relative
            || name.qualification() == Qualification::FullyQualified && !allow_leading
        {
            self.error_meta(D::UnexpectedToken, name.meta(), "invalid import name")?;
        }
        Ok(Name::new(
            name.parts().to_vec(),
            Qualification::Normal,
            name.meta().clone(),
        )
        .unwrap())
    }
    fn use_kind(&mut self) -> PResult<Option<UseKind>> {
        Ok(if self.eat(K::Function)? {
            Some(UseKind::Function)
        } else if self.eat(K::Const)? {
            Some(UseKind::Constant)
        } else {
            None
        })
    }
    fn use_item(&mut self, start: usize, name: Name, kind: UseKind) -> PResult<UseItem> {
        let alias = if self.eat(K::As)? {
            Some(self.plain_identifier()?)
        } else {
            None
        };
        if let Some(alias) = &alias {
            let alias_bytes = alias.bytes().as_bytes();
            if matches!(
                alias_bytes.to_ascii_lowercase().as_slice(),
                b"self" | b"parent" | b"static"
            ) {
                let imported = name
                    .parts()
                    .iter()
                    .map(|part| String::from_utf8_lossy(part.as_bytes()))
                    .collect::<Vec<_>>()
                    .join("\\");
                let alias_text = String::from_utf8_lossy(alias_bytes);
                self.error_meta(D::InvalidContext, alias.meta(), &format!(
                    "Cannot use {imported} as {alias_text} because '{alias_text}' is a special class name"))?;
            }
        }
        self.alloc()?;
        Ok(UseItem {
            meta: self.meta(start),
            name,
            alias,
            kind,
        })
    }
    pub(super) fn use_statement(&mut self) -> PResult<StmtKind> {
        self.bump()?;
        let explicit = self.use_kind()?;
        let at = self.peek()?.0;
        let first = self.import_name(true)?;
        let mut items = Vec::new();
        let (kind, prefix) = if self.eat(K::NamespaceSeparator)? {
            self.expect(K::LeftBrace)?;
            loop {
                let inline = self.use_kind()?;
                let start = self.peek()?.0;
                if explicit.is_some() && inline.is_some() {
                    let (_, token) = self.peek()?;
                    self.record(Diagnostic::error(
                        D::UnexpectedToken,
                        token.span(),
                        "typed group cannot contain typed entries",
                    ))?;
                }
                let name = self.import_name(false)?;
                let kind = if explicit.is_some() {
                    UseKind::Inherited
                } else {
                    inline.unwrap_or(UseKind::Normal)
                };
                items.push(self.use_item(start, name, kind)?);
                if !self.eat(K::Comma)? || self.kind()? == K::RightBrace {
                    break;
                }
            }
            self.expect(K::RightBrace)?;
            (explicit.unwrap_or(UseKind::Inherited), Some(first))
        } else {
            items.push(self.use_item(at, first, UseKind::Inherited)?);
            while self.eat(K::Comma)? {
                if matches!(self.kind()?, K::Semicolon | K::CloseTag) {
                    let (_, token) = self.peek()?;
                    self.record(Diagnostic::error(
                        D::UnexpectedToken,
                        token.span(),
                        "trailing comma is not allowed in imports",
                    ))?;
                    break;
                }
                let start = self.peek()?.0;
                let name = self.import_name(true)?;
                items.push(self.use_item(start, name, UseKind::Inherited)?);
            }
            (explicit.unwrap_or(UseKind::Normal), None)
        };
        self.terminate()?;
        Ok(StmtKind::Use(Box::new(UseStatement {
            kind,
            prefix,
            items,
        })))
    }
    pub(super) fn normalize_namespaces(&mut self, statements: Vec<Stmt>) -> Vec<Stmt> {
        let mut style = None;
        let mut invalid_before = false;
        let mut outside_error = false;
        let mut mixed = false;
        for (index, stmt) in statements.iter().enumerate() {
            if let StmtKind::Namespace(ns) = &stmt.kind {
                if style.is_none() {
                    style = Some(ns.style);
                    if invalid_before {
                        let _ = self.error_meta(
                            D::InvalidContext,
                            &stmt.meta,
                            "namespace must be the first statement",
                        );
                    }
                } else if style != Some(ns.style) {
                    let _ = self.error_meta(
                        D::InvalidContext,
                        &stmt.meta,
                        "cannot mix namespace styles",
                    );
                    mixed = true;
                }
            } else {
                if style == Some(NamespaceStyle::Braced)
                    && !matches!(stmt.kind, StmtKind::HaltCompiler(_) | StmtKind::Nop)
                    && !outside_error
                {
                    let _ = self.error_meta(
                        D::InvalidContext,
                        &stmt.meta,
                        "code outside a braced namespace",
                    );
                    outside_error = true;
                }
                if !matches!(
                    stmt.kind,
                    StmtKind::Declare(_) | StmtKind::HaltCompiler(_) | StmtKind::Nop
                ) {
                    let hashbang = index == 0
                        && matches!(&stmt.kind,StmtKind::InlineHtml(b) if b.as_bytes().starts_with(b"#!") && b.as_bytes().ends_with(b"\n") && !b.as_bytes()[..b.as_bytes().len()-1].contains(&b'\n'));
                    if !hashbang {
                        invalid_before = true;
                    }
                }
            }
        }
        if style != Some(NamespaceStyle::Semicolon) && !mixed {
            return statements;
        }
        let mut result: Vec<Stmt> = Vec::new();
        let mut active = None;
        for mut stmt in statements {
            if let StmtKind::Namespace(ns) = &mut stmt.kind {
                active = if ns.style == NamespaceStyle::Semicolon {
                    ns.body = Some(Vec::new());
                    Some(result.len())
                } else {
                    None
                };
                result.push(stmt);
            } else if !matches!(stmt.kind, StmtKind::HaltCompiler(_)) {
                if let Some(index) = active {
                    let node = &mut result[index];
                    if let StmtKind::Namespace(ns) = &mut node.kind {
                        if let (
                            Origin::Parsed { tokens: start, .. },
                            Origin::Parsed { tokens: end, .. },
                        ) = (node.meta.origin(), stmt.meta.origin())
                        {
                            node.meta = NodeMeta::prefix(
                                &self.source,
                                self.cursor.retained(),
                                start.start(),
                                end.end(),
                            );
                        }
                        ns.body.as_mut().unwrap().push(stmt);
                    }
                } else {
                    result.push(stmt);
                }
            } else {
                result.push(stmt);
            }
        }
        result
    }
}
