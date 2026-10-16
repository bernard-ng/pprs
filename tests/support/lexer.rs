//! Test-only conversion from public Rust tokens to the frozen oracle schema.
#[path = "token_names.rs"]
mod token_names;
use pprs::{LexResult, diagnostic::DiagnosticCode, token::TokenKind};
use serde_json::{Value, json};

pub fn observation(result: &LexResult, target: &str) -> Value {
    let stream = result.tokens();
    let tokens: Vec<_> = stream.tokens().iter().enumerate().map(|(index, token)| {
        let mut name = token_names::kind_name(token.kind());
        if token.kind() == TokenKind::Ampersand {
            // The pinned PHP 8.5 split skips whitespace and comments. Keep the public kind
            // unified; parser reference/type disambiguation can inspect context.
            let next = stream.tokens()[index + 1..].iter().find(|next| !matches!(next.kind(), TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment));
            name = if next.is_some_and(|next| matches!(next.kind(), TokenKind::Variable | TokenKind::Dollar | TokenKind::Ellipsis)) {
                "T_AMPERSAND_FOLLOWED_BY_VAR_OR_VARARG"
            } else { "T_AMPERSAND_NOT_FOLLOWED_BY_VAR_OR_VARARG" };
        }
        let start = result.source().location(token.span().start()).unwrap();
        let end = result.source().location(token.span().end()).unwrap();
        json!({"kind":name,"text":{"bytes":hex::encode(stream.text(index).unwrap())},
            "span":[token.span().start(),token.span().end()], "start":[start.line,start.column], "end":[end.line,end.column]})
    }).collect();
    json!({"target":target,"tokens":tokens})
}

#[allow(
    dead_code,
    reason = "the P02 channel gate uses tokens only until parser diagnostics arrive"
)]
pub fn diagnostics(result: &LexResult) -> Vec<Value> {
    result
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            let code = match diagnostic.code {
                DiagnosticCode::InvalidByte
                    if result.source().slice(diagnostic.primary) == Some(&[0][..]) =>
                {
                    "lex.null-byte"
                }
                DiagnosticCode::InvalidByte => "lex.bad-character",
                DiagnosticCode::Unterminated => "lex.unterminated-comment",
                other => panic!("unexpected lexical code {other:?}"),
            };
            json!({"code":code,"span":[diagnostic.primary.start(),diagnostic.primary.end()]})
        })
        .collect()
}
