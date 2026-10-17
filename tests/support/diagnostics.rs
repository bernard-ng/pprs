//! Lossless diagnostic projection. Native categories and wording remain observable.
use pprs::{
    ParseResult,
    diagnostic::{Category, DiagnosticCode as D},
};
use serde_json::{Value, json};

pub fn diagnostics(result: &ParseResult) -> Vec<Value> {
    result.diagnostics().iter().map(|d| {
        let category = match d.code.category() {
            Category::Lexical => "lexical", Category::Syntax => "syntax",
            Category::Context => "contextual", Category::Resource => "resource",
        };
        let code = match d.code {
            D::InvalidByte if result.source().slice(d.primary) == Some(&[0][..]) => "lex.null-byte",
            D::InvalidByte => "lex.bad-character",
            D::Unterminated => "lex.unterminated-comment",
            D::UnexpectedToken => "reference.syntax",
            D::InvalidContext => "reference.contextual",
            other => other.as_str(),
        };
        let start = result.source().location(d.primary.start()).unwrap();
        let end = result.source().location(d.primary.end()).unwrap();
        json!({"category":category,"code":code,"message":{"bytes":hex::encode(d.message.as_bytes())},
            "span":[d.primary.start(),d.primary.end()],"start":[start.line,start.column],"end":[end.line,end.column]})
    }).collect()
}

pub fn native(result: &ParseResult) -> Value {
    json!(result.diagnostics().iter().map(|d| json!({
        "code":d.code.as_str(), "severity":format!("{:?}",d.severity),
        "span":[d.primary.start(),d.primary.end()], "message":d.message,
        "expected":d.expected.iter().map(|k|format!("{k:?}")).collect::<Vec<_>>(),
        "found":d.found.map(|k|format!("{k:?}")),
        "secondary":d.secondary.iter().map(|l|json!({"span":[l.span.start(),l.span.end()],"message":l.message})).collect::<Vec<_>>()
    })).collect::<Vec<_>>())
}
