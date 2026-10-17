//! Bounded strategy prototype scanner: strings are coarse atoms with explicit boundary events.
use logos::Logos;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Word,
    Variable,
    Number,
    Trivia,
    Html,
    Open,
    Close,
    Template,
    Heredoc,
    Arrow,
    NullArrow,
    Ellipsis,
    Symbol(u8),
    Bad,
    Tail,
    Eof,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub span: Range<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: &'static str,
    pub span: Range<usize>,
}

pub fn name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 128
}
pub fn name_continue(byte: u8) -> bool {
    name_start(byte) || byte.is_ascii_digit()
}

#[derive(Logos, Debug, PartialEq)]
#[logos(utf8 = false)]
enum Regular {
    #[regex(r"[ \t\r\n\x0b\x0c]+")]
    Trivia,
    #[regex(r"[0-9]+")]
    Number,
    #[regex(r"(?-u:[a-zA-Z_\x80-\xff][a-zA-Z_0-9\x80-\xff]*)")]
    Word,
    #[regex(r"(?-u:\$[a-zA-Z_\x80-\xff][a-zA-Z_0-9\x80-\xff]*)")]
    Variable,
    #[token("?->")]
    NullArrow,
    #[token("->")]
    Arrow,
    #[token("...")]
    Ellipsis,
}
fn regular(source: &[u8]) -> (Kind, usize) {
    let mut lexer = Regular::lexer(source);
    let result = lexer.next().unwrap();
    let kind = match result {
        Ok(Regular::Trivia) => Kind::Trivia,
        Ok(Regular::Number) => Kind::Number,
        Ok(Regular::Word) => Kind::Word,
        Ok(Regular::Variable) => Kind::Variable,
        Ok(Regular::NullArrow) => Kind::NullArrow,
        Ok(Regular::Arrow) => Kind::Arrow,
        Ok(Regular::Ellipsis) => Kind::Ellipsis,
        Err(()) => {
            if source[0] < 32 || source[0] == 127 {
                Kind::Bad
            } else {
                Kind::Symbol(source[0])
            }
        }
    };
    (kind, lexer.span().end)
}

pub struct Scanner<'a> {
    pub source: &'a [u8],
    pub cursor: usize,
    pub events: Vec<Event>,
    pub errors: Vec<Range<usize>>,
    html: bool,
}
impl<'a> Scanner<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            cursor: 0,
            html: true,
            events: vec![],
            errors: vec![],
        }
    }
    fn starts(&self, bytes: &[u8]) -> bool {
        self.source[self.cursor..].starts_with(bytes)
    }
    pub fn following_kind(&self) -> Kind {
        let mut probe = Self::new(self.source);
        probe.cursor = self.cursor;
        probe.html = self.html;
        loop {
            let kind = probe.next().kind;
            if kind != Kind::Trivia {
                return kind;
            }
        }
    }
    fn open_len(&self) -> usize {
        if self.starts(b"<?=") {
            3
        } else if self.source[self.cursor..]
            .get(..5)
            .is_some_and(|s| s.eq_ignore_ascii_case(b"<?php"))
            && self
                .source
                .get(self.cursor + 5)
                .is_some_and(u8::is_ascii_whitespace)
        {
            5
        } else {
            0
        }
    }
    pub fn next(&mut self) -> Token {
        let start = self.cursor;
        let kind = if start == self.source.len() {
            Kind::Eof
        } else if self.html {
            let len = self.open_len();
            if len != 0 {
                self.cursor += len;
                self.html = false;
                Kind::Open
            } else {
                self.cursor += 1;
                while self.cursor < self.source.len() && self.open_len() == 0 {
                    self.cursor += 1;
                }
                Kind::Html
            }
        } else if self.starts(b"?>") {
            self.cursor += 2;
            self.html = true;
            Kind::Close
        } else if self.starts(b"/*") {
            self.cursor += 2;
            while self.cursor < self.source.len() && !self.starts(b"*/") {
                self.cursor += 1;
            }
            if self.cursor < self.source.len() {
                self.cursor += 2;
            } else {
                self.errors.push(start..self.cursor);
            }
            Kind::Trivia
        } else if self.starts(b"//") || self.starts(b"#") {
            while self.cursor < self.source.len()
                && !matches!(self.source[self.cursor], b'\r' | b'\n')
                && !self.starts(b"?>")
            {
                self.cursor += 1;
            }
            Kind::Trivia
        } else if matches!(self.source[start], b'\'' | b'"' | b'`') {
            self.quoted(0);
            Kind::Template
        } else if self.starts(b"<<<") {
            self.heredoc();
            Kind::Heredoc
        } else {
            let (kind, len) = regular(&self.source[start..]);
            assert!(len > 0 && len <= self.source.len() - start);
            self.cursor += len;
            if kind == Kind::Bad {
                self.errors.push(start..self.cursor);
            }
            kind
        };
        Token {
            kind,
            span: start..self.cursor,
        }
    }
    fn quoted(&mut self, depth: usize) {
        let start = self.cursor;
        let quote = self.source[self.cursor];
        self.cursor += 1;
        if depth >= 64 {
            self.errors.push(start..self.cursor);
            return;
        }
        while self.cursor < self.source.len() {
            let byte = self.source[self.cursor];
            if byte == b'\\' {
                self.cursor = (self.cursor + 2).min(self.source.len());
            } else if byte == quote {
                self.cursor += 1;
                self.events.push(Event {
                    kind: "quoted",
                    span: start..self.cursor,
                });
                return;
            } else if quote != b'\'' && (self.starts(b"{$") || self.starts(b"${")) {
                let interpolation = self.cursor;
                self.cursor += if self.starts(b"${") { 2 } else { 1 };
                self.interpolation(depth + 1);
                self.events.push(Event {
                    kind: "interpolation",
                    span: interpolation..self.cursor,
                });
            } else {
                self.cursor += 1;
            }
        }
        self.errors.push(start..self.cursor);
    }
    fn interpolation(&mut self, depth: usize) {
        let mut braces = 1;
        while self.cursor < self.source.len() {
            match self.source[self.cursor] {
                b'\'' | b'"' | b'`' => self.quoted(depth),
                b'{' => {
                    braces += 1;
                    self.cursor += 1;
                }
                b'}' => {
                    braces -= 1;
                    self.cursor += 1;
                    if braces == 0 {
                        return;
                    }
                }
                _ => self.cursor += 1,
            }
        }
    }
    fn heredoc(&mut self) {
        let start = self.cursor;
        self.cursor += 3;
        while matches!(self.source.get(self.cursor), Some(b' ' | b'\t')) {
            self.cursor += 1;
        }
        let quote = self
            .source
            .get(self.cursor)
            .copied()
            .filter(|b| matches!(b, b'\'' | b'"'));
        if quote.is_some() {
            self.cursor += 1;
        }
        let label_start = self.cursor;
        while self
            .source
            .get(self.cursor)
            .is_some_and(|b| name_continue(*b))
        {
            self.cursor += 1;
        }
        let label_end = self.cursor;
        if label_start == label_end {
            self.errors.push(start..self.cursor);
            return;
        }
        if let Some(quote) = quote {
            if self.source.get(self.cursor) != Some(&quote) {
                self.errors.push(start..self.cursor);
                return;
            }
            self.cursor += 1;
        }
        if self.source.get(self.cursor) == Some(&b'\r') {
            self.cursor += 1;
        }
        if self.source.get(self.cursor) != Some(&b'\n') {
            self.errors.push(start..self.cursor);
            return;
        }
        self.cursor += 1;
        let body = self.cursor;
        while self.cursor < self.source.len() {
            let line = self.cursor;
            while matches!(self.source.get(self.cursor), Some(b' ' | b'\t')) {
                self.cursor += 1;
            }
            let label = &self.source[label_start..label_end];
            if self.starts(label)
                && !self
                    .source
                    .get(self.cursor + label.len())
                    .is_some_and(|b| name_continue(*b))
            {
                let indent = &self.source[line..self.cursor];
                if indent.contains(&b' ') && indent.contains(&b'\t') {
                    self.errors.push(line..self.cursor);
                }
                for text in self.source[body..line].split(|b| *b == b'\n') {
                    if text.iter().any(|b| !b.is_ascii_whitespace()) && !text.starts_with(indent) {
                        self.errors.push(body..line);
                        break;
                    }
                }
                self.events.push(Event {
                    kind: if quote == Some(b'\'') {
                        "nowdoc-body"
                    } else {
                        "heredoc-body"
                    },
                    span: body..line,
                });
                self.events.push(Event {
                    kind: "heredoc-label",
                    span: self.cursor..self.cursor + label.len(),
                });
                self.cursor += label.len();
                return;
            }
            while self.cursor < self.source.len() && self.source[self.cursor] != b'\n' {
                self.cursor += 1;
            }
            if self.cursor < self.source.len() {
                self.cursor += 1;
            }
        }
        self.errors.push(start..self.cursor);
    }
    pub fn halt_tail(&mut self) -> Token {
        let start = self.cursor;
        self.cursor = self.source.len();
        Token {
            kind: Kind::Tail,
            span: start..self.cursor,
        }
    }
}
