//! Bounded recursive-descent / Pratt experiment, not the PHP implementation.
use super::scanner::{Kind, Scanner, Token};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub tag: &'static str,
    pub span: Range<usize>,
    pub children: Vec<Tree>,
}
impl Tree {
    fn leaf(tag: &'static str, span: Range<usize>) -> Self {
        Self {
            tag,
            span,
            children: vec![],
        }
    }
    fn join(tag: &'static str, left: Self, right: Self) -> Self {
        Self {
            tag,
            span: left.span.start..right.span.end,
            children: vec![left, right],
        }
    }
}
pub struct Parser<'a> {
    pub scanner: Scanner<'a>,
    pub tokens: Vec<Token>,
    pub errors: Vec<Range<usize>>,
    look: Option<Token>,
    php8: bool,
    type_mode: bool,
}
impl<'a> Parser<'a> {
    pub fn new(source: &'a [u8], php8: bool) -> Self {
        Self {
            scanner: Scanner::new(source),
            tokens: vec![],
            errors: vec![],
            look: None,
            php8,
            type_mode: false,
        }
    }
    fn peek(&mut self) -> Token {
        if self.look.is_none() {
            loop {
                let token = self.scanner.next();
                self.tokens.push(token.clone());
                if !matches!(
                    token.kind,
                    Kind::Trivia | Kind::Html | Kind::Open | Kind::Close
                ) {
                    self.look = Some(token);
                    break;
                }
            }
        }
        self.look.clone().unwrap()
    }
    fn take(&mut self) -> Token {
        let token = self.peek();
        self.look = None;
        token
    }
    fn at(&mut self, byte: u8) -> bool {
        self.peek().kind == Kind::Symbol(byte)
    }
    fn expect(&mut self, byte: u8) -> bool {
        if self.at(byte) {
            self.take();
            true
        } else {
            let span = self.peek().span;
            self.errors.push(span);
            false
        }
    }
    fn word(&mut self, word: &[u8]) -> bool {
        let token = self.peek();
        token.kind == Kind::Word && self.scanner.source[token.span].eq_ignore_ascii_case(word)
    }
    pub fn program(&mut self, scope: usize) -> Vec<Tree> {
        if scope > 64 {
            let span = self.peek().span;
            self.errors.push(span);
            self.synchronize();
            return vec![];
        }
        let mut statements = vec![];
        while self.peek().kind != Kind::Eof && !(scope > 0 && self.at(b'}')) {
            let before = self.peek().span.start;
            if self.at(b'{') {
                let start = self.take().span.start;
                if scope >= 64 {
                    self.errors.push(start..start + 1);
                    self.synchronize();
                    continue;
                }
                let children = self.program(scope + 1);
                let end = self.peek().span.end;
                self.expect(b'}');
                statements.push(Tree {
                    tag: "block",
                    span: start..end,
                    children,
                });
            } else if self.word(b"__halt_compiler") {
                let start = self.take().span.start;
                let valid = self.expect(b'(') && self.expect(b')') && self.expect(b';');
                if valid && scope == 0 {
                    let token = self.scanner.halt_tail();
                    let end = token.span.end;
                    self.tokens.push(token);
                    statements.push(Tree::leaf("halt", start..end));
                } else {
                    self.errors.push(start..self.scanner.cursor);
                    if !valid {
                        self.synchronize();
                    }
                }
            } else if self.word(b"function") {
                statements.push(self.function(scope));
            } else {
                let tree = self.expression(0, 0);
                if !self.expect(b';') {
                    self.synchronize();
                }
                statements.push(tree);
            }
            if self.peek().span.start == before {
                let span = self.take().span;
                self.errors.push(span);
            }
        }
        statements
    }
    fn synchronize(&mut self) {
        while !matches!(self.peek().kind, Kind::Eof | Kind::Symbol(b';' | b'}')) {
            self.take();
        }
        if self.at(b';') {
            self.take();
        }
    }
    fn function(&mut self, scope: usize) -> Tree {
        let start = self.take().span.start;
        self.take();
        self.expect(b'(');
        self.type_mode = true;
        let ty = self.expression(0, 0);
        self.type_mode = false;
        if self.at(b'&') {
            self.take();
        }
        if self.peek().kind == Kind::Variable {
            self.take();
        }
        self.expect(b')');
        self.expect(b'{');
        let children = self.program(scope + 1);
        let end = self.peek().span.end;
        self.expect(b'}');
        let mut all = vec![ty];
        all.extend(children);
        Tree {
            tag: "function",
            span: start..end,
            children: all,
        }
    }
    pub fn expression(&mut self, min: u8, depth: usize) -> Tree {
        let token = self.peek();
        if depth >= 64 {
            self.errors.push(token.span.clone());
            return Tree::leaf("error", token.span.start..token.span.start);
        }
        let mut left = match token.kind {
            Kind::Word | Kind::Variable | Kind::Number | Kind::Template | Kind::Heredoc => {
                self.take();
                Tree::leaf("atom", token.span)
            }
            Kind::Symbol(b'(') => {
                self.take();
                let mut inner = self.expression(0, depth + 1);
                self.expect(b')');
                inner.span = token.span.start..self.scanner.cursor;
                inner
            }
            _ => {
                self.errors.push(token.span.clone());
                return Tree::leaf("error", token.span.start..token.span.start);
            }
        };
        loop {
            let token = self.peek();
            if !self.type_mode && token.kind == Kind::Symbol(b'(') && min <= 50 {
                self.take();
                let mut children = vec![left];
                while !self.at(b')') && self.peek().kind != Kind::Eof {
                    let before = self.peek().span.start;
                    children.push(self.expression(0, depth + 1));
                    if !self.at(b',') {
                        break;
                    }
                    self.take();
                    if self.peek().span.start == before {
                        break;
                    }
                }
                let end = self.peek().span.end;
                self.expect(b')');
                left = Tree {
                    tag: "call",
                    span: children[0].span.start..end,
                    children,
                };
                continue;
            }
            if !self.type_mode && token.kind == Kind::Symbol(b'[') && min <= 50 {
                self.take();
                let index = self.expression(0, depth + 1);
                let end = self.peek().span.end;
                self.expect(b']');
                left = Tree::join("index", left, index);
                left.span.end = end;
                continue;
            }
            if !self.type_mode && matches!(token.kind, Kind::Arrow | Kind::NullArrow) && min <= 50 {
                self.take();
                let member = self.take();
                if member.kind != Kind::Word {
                    self.errors.push(member.span.clone());
                }
                left = Tree::join(
                    if token.kind == Kind::Arrow {
                        "member"
                    } else {
                        "null-member"
                    },
                    left,
                    Tree::leaf("atom", member.span),
                );
                continue;
            }
            let (power, tag) = match token.kind {
                Kind::Symbol(b'=') if !self.type_mode => (5, "assign"),
                Kind::Symbol(b'.') if !self.type_mode => {
                    (if self.php8 { 10 } else { 20 }, "concat")
                }
                Kind::Symbol(b'+') if !self.type_mode => (20, "add"),
                Kind::Symbol(b'*') if !self.type_mode => (30, "mul"),
                Kind::Symbol(b'|') if self.type_mode => (10, "union"),
                Kind::Symbol(b'&') if self.type_mode => {
                    // A reference marker before a variable is not an intersection.
                    if self.scanner.following_kind() == Kind::Variable {
                        break;
                    }
                    (20, "intersection")
                }
                _ => break,
            };
            if power < min {
                break;
            }
            self.take();
            let right = self.expression(if tag == "assign" { power } else { power + 1 }, depth + 1);
            left = Tree::join(tag, left, right);
        }
        left
    }
}
