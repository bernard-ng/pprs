//! Regular byte patterns; no Logos type crosses the public boundary.
use crate::token::TokenKind;
use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(utf8 = false)]
pub(crate) enum Regular {
    #[regex(r"[ \t\r\n]+")]
    Whitespace,
    #[regex(r"(?-u:[a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*)")]
    Word,
    #[regex(r"(?-u:\$[a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*)")]
    Variable,
    #[regex(r"(?-u:\\?[a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*(\\[a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*)+)")]
    #[regex(r"(?-u:\\[a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*)")]
    Name,
    #[regex(r"0[xX][0-9a-fA-F]+(_[0-9a-fA-F]+)*")]
    #[regex(r"0[bB][01]+(_[01]+)*")]
    #[regex(r"0[oO][0-7]+(_[0-7]+)*")]
    #[regex(r"[0-9]+(_[0-9]+)*")]
    Integer,
    #[regex(
        r"([0-9]+(_[0-9]+)*\.([0-9]+(_[0-9]+)*)?|\.[0-9]+(_[0-9]+)*)([eE][+-]?[0-9]+(_[0-9]+)*)?"
    )]
    #[regex(r"[0-9]+(_[0-9]+)*[eE][+-]?[0-9]+(_[0-9]+)*")]
    Float,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("=")]
    Equal,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token(".")]
    Dot,
    #[token("%")]
    Percent,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("!")]
    Bang,
    #[token("~")]
    Tilde,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("@")]
    At,
    #[token("$")]
    Dollar,
    #[token("\\")]
    NamespaceSeparator,
    #[token("...")]
    Ellipsis,
    #[token("->")]
    ObjectArrow,
    #[token("?->")]
    NullsafeArrow,
    #[token("::")]
    DoubleColon,
    #[token("=>")]
    DoubleArrow,
    #[token("++")]
    Increment,
    #[token("--")]
    Decrement,
    #[token("**")]
    Power,
    #[token("<<")]
    ShiftLeft,
    #[token(">>")]
    ShiftRight,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    #[token("<>")]
    NotEqual,
    #[token("===")]
    Identical,
    #[token("!==")]
    NotIdentical,
    #[token("<=>")]
    Spaceship,
    #[token("&&")]
    BooleanAnd,
    #[token("||")]
    BooleanOr,
    #[token("??")]
    Coalesce,
    #[token("+=")]
    PlusEqual,
    #[token("-=")]
    MinusEqual,
    #[token("*=")]
    MultiplyEqual,
    #[token("/=")]
    DivideEqual,
    #[token(".=")]
    ConcatEqual,
    #[token("%=")]
    ModuloEqual,
    #[token("&=")]
    AndEqual,
    #[token("|=")]
    OrEqual,
    #[token("^=")]
    XorEqual,
    #[token("<<=")]
    ShiftLeftEqual,
    #[token(">>=")]
    ShiftRightEqual,
    #[token("**=")]
    PowerEqual,
    #[token("??=")]
    CoalesceEqual,
    #[token("#[")]
    Attribute,
    #[regex(r"(?i:\([ \t]*(int|integer)[ \t]*\))")]
    IntCast,
    #[regex(r"(?i:\([ \t]*(bool|boolean)[ \t]*\))")]
    BoolCast,
    #[regex(r"(?i:\([ \t]*(float|double|real)[ \t]*\))")]
    FloatCast,
    #[regex(r"(?i:\([ \t]*(string|binary)[ \t]*\))")]
    StringCast,
    #[regex(r"(?i:\([ \t]*(array)[ \t]*\))")]
    ArrayCast,
    #[regex(r"(?i:\([ \t]*(object)[ \t]*\))")]
    ObjectCast,
    #[regex(r"(?i:\([ \t]*(unset)[ \t]*\))")]
    UnsetCast,
}

impl Regular {
    pub(super) fn kind(self) -> TokenKind {
        match self {
            Self::Whitespace => TokenKind::Whitespace,
            Self::Variable => TokenKind::Variable,
            Self::Integer => TokenKind::Integer,
            Self::Float => TokenKind::Float,
            Self::Semicolon => TokenKind::Semicolon,
            Self::Comma => TokenKind::Comma,
            Self::LeftParen => TokenKind::LeftParen,
            Self::RightParen => TokenKind::RightParen,
            Self::LeftBrace => TokenKind::LeftBrace,
            Self::RightBrace => TokenKind::RightBrace,
            Self::LeftBracket => TokenKind::LeftBracket,
            Self::RightBracket => TokenKind::RightBracket,
            Self::Equal => TokenKind::Equal,
            Self::Plus => TokenKind::Plus,
            Self::Minus => TokenKind::Minus,
            Self::Star => TokenKind::Star,
            Self::Slash => TokenKind::Slash,
            Self::Dot => TokenKind::Dot,
            Self::Percent => TokenKind::Percent,
            Self::Ampersand => TokenKind::Ampersand,
            Self::Pipe => TokenKind::Pipe,
            Self::Caret => TokenKind::Caret,
            Self::Bang => TokenKind::Bang,
            Self::Tilde => TokenKind::Tilde,
            Self::Question => TokenKind::Question,
            Self::Colon => TokenKind::Colon,
            Self::Less => TokenKind::Less,
            Self::Greater => TokenKind::Greater,
            Self::At => TokenKind::At,
            Self::Dollar => TokenKind::Dollar,
            Self::NamespaceSeparator => TokenKind::NamespaceSeparator,
            Self::Ellipsis => TokenKind::Ellipsis,
            Self::ObjectArrow => TokenKind::ObjectArrow,
            Self::NullsafeArrow => TokenKind::NullsafeArrow,
            Self::DoubleColon => TokenKind::DoubleColon,
            Self::DoubleArrow => TokenKind::DoubleArrow,
            Self::Increment => TokenKind::Increment,
            Self::Decrement => TokenKind::Decrement,
            Self::Power => TokenKind::Power,
            Self::ShiftLeft => TokenKind::ShiftLeft,
            Self::ShiftRight => TokenKind::ShiftRight,
            Self::LessEqual => TokenKind::LessEqual,
            Self::GreaterEqual => TokenKind::GreaterEqual,
            Self::EqualEqual => TokenKind::EqualEqual,
            Self::NotEqual => TokenKind::NotEqual,
            Self::Identical => TokenKind::Identical,
            Self::NotIdentical => TokenKind::NotIdentical,
            Self::Spaceship => TokenKind::Spaceship,
            Self::BooleanAnd => TokenKind::BooleanAnd,
            Self::BooleanOr => TokenKind::BooleanOr,
            Self::Coalesce => TokenKind::Coalesce,
            Self::PlusEqual => TokenKind::PlusEqual,
            Self::MinusEqual => TokenKind::MinusEqual,
            Self::MultiplyEqual => TokenKind::MultiplyEqual,
            Self::DivideEqual => TokenKind::DivideEqual,
            Self::ConcatEqual => TokenKind::ConcatEqual,
            Self::ModuloEqual => TokenKind::ModuloEqual,
            Self::AndEqual => TokenKind::AndEqual,
            Self::OrEqual => TokenKind::OrEqual,
            Self::XorEqual => TokenKind::XorEqual,
            Self::ShiftLeftEqual => TokenKind::ShiftLeftEqual,
            Self::ShiftRightEqual => TokenKind::ShiftRightEqual,
            Self::PowerEqual => TokenKind::PowerEqual,
            Self::CoalesceEqual => TokenKind::CoalesceEqual,
            Self::Attribute => TokenKind::Attribute,
            Self::IntCast => TokenKind::IntCast,
            Self::BoolCast => TokenKind::BoolCast,
            Self::FloatCast => TokenKind::FloatCast,
            Self::StringCast => TokenKind::StringCast,
            Self::ArrayCast => TokenKind::ArrayCast,
            Self::ObjectCast => TokenKind::ObjectCast,
            Self::UnsetCast => TokenKind::UnsetCast,
            Self::Word | Self::Name => TokenKind::Identifier,
        }
    }
}
