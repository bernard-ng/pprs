use super::{PResult, Parser};
use crate::{
    ByteString,
    ast::{Expr, ExprKind, IntegerBase, Member, NodeMeta, StringKind, StringPart},
    diagnostic::{Diagnostic, DiagnosticCode as D},
    literal::{self, EscapeMode, NumericOffset},
    token::{Token, TokenKind as K},
};
enum Part {
    Text(usize, Token),
    Expr(Box<Expr>),
}
impl Parser {
    pub(super) fn decode_literal(&mut self, token: Token) -> PResult<Option<ByteString>> {
        match literal::decode_quoted(self.bytes(token)) {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                self.record(Diagnostic::error(
                    D::InvalidLiteral,
                    token.span(),
                    error.to_string(),
                ))?;
                Ok(None)
            }
        }
    }
    #[allow(
        clippy::too_many_lines,
        reason = "string-mode grammar and segment assembly"
    )]
    pub(super) fn string_expression(&mut self) -> PResult<Box<Expr>> {
        let (start, open) = self.bump()?;
        let mut invalid_escape = false;
        let doc = open.kind() == K::StartHeredoc;
        let nowdoc = doc && self.bytes(open).contains(&b'\'');
        let close = match open.kind() {
            K::StartHeredoc => K::EndHeredoc,
            K::Backtick => K::Backtick,
            _ => K::DoubleQuote,
        };
        let mut raw = Vec::new();
        while !matches!(self.kind()?, K::Eof) && self.kind()? != close {
            let (at, token) = self.peek()?;
            match token.kind() {
                K::StringPart => {
                    self.bump()?;
                    raw.push(Part::Text(at, token));
                }
                K::Variable => {
                    let mut expr = self.variable()?;
                    if self.eat(K::LeftBracket)? {
                        let index = self.peek()?.0;
                        let negative = self.eat(K::Minus)?;
                        let (_, offset) = self.peek()?;
                        let dim = if offset.kind() == K::Variable {
                            self.variable()?
                        } else {
                            self.bump()?;
                            let mut bytes = if negative { vec![b'-'] } else { Vec::new() };
                            bytes.extend_from_slice(self.bytes(offset));
                            let value = if offset.kind() == K::Identifier {
                                NumericOffset::String(bytes.into())
                            } else {
                                literal::decode_numeric_offset(&bytes)
                            };
                            self.make(
                                index,
                                match value {
                                    NumericOffset::Integer(value) => ExprKind::Integer {
                                        value,
                                        base: IntegerBase::Decimal,
                                    },
                                    NumericOffset::String(value) => ExprKind::String {
                                        value,
                                        kind: StringKind::SingleQuoted,
                                    },
                                },
                            )?
                        };
                        self.expect(K::RightBracket)?;
                        expr = self.make(
                            at,
                            ExprKind::ArrayDim {
                                array: expr,
                                dim: Some(dim),
                            },
                        )?;
                    } else if matches!(self.kind()?, K::ObjectArrow | K::NullsafeArrow) {
                        let nullsafe = self.bump()?.1.kind() == K::NullsafeArrow;
                        let (index, name) = self.expect(K::Identifier)?;
                        let bytes = self.bytes(name).to_vec();
                        let name = Member::Identifier(self.identifier(index, &bytes)?);
                        expr = self.make(
                            at,
                            ExprKind::Property {
                                object: expr,
                                name,
                                nullsafe,
                            },
                        )?;
                    }
                    raw.push(Part::Expr(expr));
                }
                K::CurlyOpen => {
                    self.bump()?;
                    let expr = self.expression(0)?;
                    if !self.writable(&expr, false) {
                        self.error_meta(
                            D::InvalidContext,
                            &expr.meta,
                            "curly interpolation requires a variable",
                        )?;
                    }
                    self.expect(K::RightBrace)?;
                    raw.push(Part::Expr(expr));
                }
                K::DollarOpenCurly => {
                    self.bump()?;
                    let expr = if self.kind()? == K::StringVarname {
                        let (index, name) = self.bump()?;
                        let bytes = self.bytes(name).to_vec();
                        let id = self.identifier(index, &bytes)?;
                        let mut expr = self.make(index, ExprKind::Variable(id))?;
                        if self.eat(K::LeftBracket)? {
                            let dim = self.expression(0)?;
                            self.expect(K::RightBracket)?;
                            expr = self.make(
                                at,
                                ExprKind::ArrayDim {
                                    array: expr,
                                    dim: Some(dim),
                                },
                            )?;
                        }
                        expr
                    } else {
                        let name = self.expression(0)?;
                        self.make(at, ExprKind::DynamicVariable(name))?
                    };
                    self.expect(K::RightBrace)?;
                    raw.push(Part::Expr(expr));
                }
                _ => {
                    self.record(Diagnostic::error(
                        D::UnexpectedToken,
                        token.span(),
                        "invalid string interpolation",
                    ))?;
                    self.bump()?;
                }
            }
        }
        let (_, end) = self.expect(close)?;
        let mut indentation = if doc {
            self.bytes(end)
                .iter()
                .copied()
                .take_while(|b| matches!(b, b' ' | b'\t'))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if indentation.contains(&b' ') && indentation.contains(&b'\t') {
            self.record(Diagnostic::error(
                D::InvalidLiteral,
                end.span(),
                "Invalid indentation - tabs and spaces cannot be mixed",
            ))?;
            indentation.clear();
        }
        let interpolated = raw.iter().any(|p| matches!(p, Part::Expr(_)));
        if doc
            && interpolated
            && matches!(raw.first(), Some(Part::Expr(_)))
            && !indentation.is_empty()
        {
            self.record(Diagnostic::error(
                D::InvalidLiteral,
                open.span(),
                "Invalid body indentation before interpolation",
            ))?;
        }
        let count = raw.len();
        let mut parts = Vec::new();
        let mut constant = Vec::new();
        for (index, part) in raw.into_iter().enumerate() {
            match part {
                Part::Expr(expr) => parts.push(StringPart::Expr(*expr)),
                Part::Text(at, token) => {
                    let value = if doc {
                        self.doc_part(token, &indentation, index == 0, index + 1 == count, nowdoc)?
                            .unwrap_or_else(|| {
                                invalid_escape = true;
                                ByteString::default()
                            })
                    } else {
                        match literal::decode_escapes(
                            self.bytes(token),
                            if close == K::Backtick {
                                EscapeMode::Backtick
                            } else {
                                EscapeMode::DoubleQuoted
                            },
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                invalid_escape = true;
                                self.record(Diagnostic::error(
                                    D::InvalidLiteral,
                                    token.span(),
                                    error.to_string(),
                                ))?;
                                ByteString::default()
                            }
                        }
                    };
                    if !interpolated && close != K::Backtick {
                        constant.extend_from_slice(value.as_bytes());
                    } else if !doc || !value.as_bytes().is_empty() {
                        self.alloc()?;
                        let meta =
                            NodeMeta::prefix(&self.source, self.cursor.retained(), at, at + 1);
                        parts.push(StringPart::Text { meta, value });
                    }
                }
            }
        }
        if invalid_escape {
            return self.make(start, ExprKind::Error);
        }
        let kind = if doc {
            if nowdoc {
                StringKind::Nowdoc
            } else {
                StringKind::Heredoc
            }
        } else {
            StringKind::DoubleQuoted
        };
        self.make(
            start,
            if close == K::Backtick {
                ExprKind::ShellExec(parts)
            } else if interpolated {
                ExprKind::Interpolated { parts, kind }
            } else {
                ExprKind::String {
                    value: constant.into(),
                    kind,
                }
            },
        )
    }
    fn doc_part(
        &mut self,
        token: Token,
        indent: &[u8],
        first: bool,
        last: bool,
        nowdoc: bool,
    ) -> PResult<Option<ByteString>> {
        let body = self.bytes(token).to_vec();
        if let Ok(value) = literal::decode_doc_segment(&body, indent, first, last, nowdoc) {
            return Ok(Some(value));
        }
        // Match the reference's recovery: report each invalid prefix, strip the
        // available prefix anyway, then interpret the recovered segment.
        let mut out = Vec::new();
        let mut at = 0;
        loop {
            if !indent.is_empty() && ((at == 0 && first) || (at > 0 && body[at - 1] == b'\n')) {
                let start = at;
                while at < body.len()
                    && at - start < indent.len()
                    && matches!(body[at], b' ' | b'\t')
                {
                    at += 1;
                }
                if body[start..at].iter().any(|b| *b != indent[0]) {
                    self.record(Diagnostic::error(
                        D::InvalidLiteral,
                        token.span(),
                        "Invalid indentation - tabs and spaces cannot be mixed",
                    ))?;
                } else if at - start < indent.len()
                    && !body.get(at).is_some_and(|b| matches!(b, b'\r' | b'\n'))
                    && !(at == body.len() && last)
                {
                    self.record(Diagnostic::error(
                        D::InvalidLiteral,
                        token.span(),
                        "Invalid body indentation level",
                    ))?;
                }
            }
            if at == body.len() {
                break;
            }
            out.push(body[at]);
            at += 1;
        }
        if last {
            if out.ends_with(b"\r\n") {
                out.truncate(out.len() - 2);
            } else if out.last().is_some_and(|b| matches!(b, b'\r' | b'\n')) {
                out.pop();
            }
        }
        match literal::decode_escapes(
            &out,
            if nowdoc {
                EscapeMode::Nowdoc
            } else {
                EscapeMode::Heredoc
            },
        ) {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                self.record(Diagnostic::error(
                    D::InvalidLiteral,
                    token.span(),
                    error.to_string(),
                ))?;
                Ok(None)
            }
        }
    }
}
