use crate::{
    PhpVersion,
    ast::{BinaryOp as B, CastKind as C, UnaryOp as U},
    token::TokenKind as K,
};
pub(super) fn binary(kind: K, version: PhpVersion) -> Option<(B, u8, bool)> {
    Some(match kind {
        K::LogicalOr => (B::LogicalOr, 10, false),
        K::LogicalXor => (B::LogicalXor, 12, false),
        K::LogicalAnd => (B::LogicalAnd, 14, false),
        K::Coalesce => (B::Coalesce, 28, true),
        K::BooleanOr => (B::BooleanOr, 30, false),
        K::BooleanAnd => (B::BooleanAnd, 32, false),
        K::Pipe => (B::BitwiseOr, 34, false),
        K::Caret => (B::BitwiseXor, 36, false),
        K::Ampersand => (B::BitwiseAnd, 38, false),
        K::EqualEqual => (B::Equal, 40, false),
        K::NotEqual => (B::NotEqual, 40, false),
        K::Identical => (B::Identical, 40, false),
        K::NotIdentical => (B::NotIdentical, 40, false),
        K::Spaceship => (B::Spaceship, 40, false),
        K::Less => (B::Less, 42, false),
        K::LessEqual => (B::LessEqual, 42, false),
        K::Greater => (B::Greater, 42, false),
        K::GreaterEqual => (B::GreaterEqual, 42, false),
        K::Dot => (
            B::Concat,
            if version < PhpVersion::Php80 { 48 } else { 44 },
            false,
        ),
        K::ShiftLeft => (B::ShiftLeft, 46, false),
        K::ShiftRight => (B::ShiftRight, 46, false),
        K::Plus => (B::Add, 48, false),
        K::Minus => (B::Subtract, 48, false),
        K::Star => (B::Multiply, 50, false),
        K::Slash => (B::Divide, 50, false),
        K::Percent => (B::Modulo, 50, false),
        K::Power => (B::Power, 58, true),
        _ => return None,
    })
}
pub(super) fn assignment(kind: K) -> Option<B> {
    Some(match kind {
        K::PlusEqual => B::Add,
        K::MinusEqual => B::Subtract,
        K::MultiplyEqual => B::Multiply,
        K::DivideEqual => B::Divide,
        K::ModuloEqual => B::Modulo,
        K::ConcatEqual => B::Concat,
        K::AndEqual => B::BitwiseAnd,
        K::OrEqual => B::BitwiseOr,
        K::XorEqual => B::BitwiseXor,
        K::ShiftLeftEqual => B::ShiftLeft,
        K::ShiftRightEqual => B::ShiftRight,
        K::PowerEqual => B::Power,
        K::CoalesceEqual => B::Coalesce,
        _ => return None,
    })
}
pub(super) fn unary(kind: K) -> Option<(U, u8)> {
    Some(match kind {
        K::Plus => (U::Plus, 56),
        K::Minus => (U::Minus, 56),
        K::Bang => (U::Not, 52),
        K::Tilde => (U::BitwiseNot, 56),
        K::At => (U::Suppress, 56),
        K::Increment => (U::PreIncrement, 56),
        K::Decrement => (U::PreDecrement, 56),
        K::Clone => (U::Clone, 70),
        K::Print => (U::Print, 16),
        K::Throw => (U::Throw, 1),
        K::YieldFrom => (U::YieldFrom, 22),
        _ => return None,
    })
}
pub(super) fn cast(kind: K) -> Option<C> {
    Some(match kind {
        K::IntCast => C::Integer,
        K::FloatCast => C::Float,
        K::StringCast => C::String,
        K::ArrayCast => C::Array,
        K::ObjectCast => C::Object,
        K::BoolCast => C::Boolean,
        K::UnsetCast => C::Unset,
        _ => return None,
    })
}
pub(super) fn boundary(kind: K) -> bool {
    matches!(
        kind,
        K::Semicolon
            | K::CloseTag
            | K::Eof
            | K::RightParen
            | K::RightBracket
            | K::RightBrace
            | K::Comma
            | K::Colon
            | K::DoubleArrow
            | K::EndHeredoc
    )
}
