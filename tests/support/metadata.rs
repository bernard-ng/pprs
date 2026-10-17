//! Independent source metadata projection. Field order follows the pinned node API.
use pprs::{
    ParseResult,
    ast::{NodeMeta, Origin},
    token::TokenKind as K,
};
use serde_json::{Value, json};
use std::sync::OnceLock;
static FIELDS: OnceLock<Value> = OnceLock::new();
#[derive(Clone, Default)]
struct Context {
    property: Option<Value>,
    doc: Option<(usize, usize, Vec<u8>)>,
}
pub fn raw(meta: &NodeMeta) -> Value {
    match meta.origin() {
        Origin::Parsed { span, tokens } => {
            json!({"span":[span.start(),span.end()],"tokens":[tokens.start(),tokens.end()],"comments":meta.comments()})
        }
        Origin::Synthetic => panic!("parser produced synthetic origin"),
    }
}
fn bytes(value: &[u8]) -> Value {
    json!({"bytes":hex::encode(value)})
}
fn integer(value: u8) -> Value {
    json!({"integer":value.to_string()})
}
fn coordinates(source: &[u8], offset: usize) -> [usize; 2] {
    let (mut line, mut start, mut at) = (1, 0, 0);
    while at < offset {
        if source[at] == b'\n' || source[at] == b'\r' && source.get(at + 1) != Some(&b'\n') {
            line += 1;
            start = at + 1;
        }
        at += 1;
    }
    [line, offset - start + 1]
}
fn origin(source: &[u8], start: usize, end: usize, first: usize, last: usize) -> Value {
    json!({"span":[start,end],"token_span":[first,last],"empty":start==end,"start":coordinates(source,start),"end":coordinates(source,end)})
}
pub fn extract(ast: &mut Value, result: &ParseResult) -> Vec<Value> {
    let mut out = Vec::new();
    collect(ast, "", result, &mut out, Context::default());
    out
}
fn collect(
    value: &mut Value,
    path: &str,
    result: &ParseResult,
    out: &mut Vec<Value>,
    mut context: Context,
) {
    if let Value::Array(values) = value {
        for (i, v) in values.iter_mut().enumerate() {
            collect(v, &format!("{path}/{i}"), result, out, context.clone());
        }
        return;
    }
    let Some(kind) = value.get("kind").and_then(Value::as_str).map(str::to_owned) else {
        return;
    };
    let meta = value
        .as_object_mut()
        .unwrap()
        .remove("_meta")
        .unwrap_or_else(|| panic!("missing metadata: {kind} at {path}"));
    let n = |key: &str, i: usize| usize::try_from(meta[key][i].as_u64().unwrap()).unwrap();
    let (start, end, first, last) = (n("span", 0), n("span", 1), n("tokens", 0), n("tokens", 1));
    let source = result.source().as_bytes();
    let comments=meta["comments"].as_array().unwrap().iter().map(|i|{
        let i=usize::try_from(i.as_u64().unwrap()).unwrap();let token=result.tokens().tokens()[i];let span=token.span();
        json!({"kind":if token.kind()==K::DocComment {"doc"}else{"comment"},"text":bytes(&source[span.start()..span.end()]),"origin":origin(source,span.start(),span.end(),i,i+1)})
    }).collect::<Vec<_>>();
    if matches!(
        kind.as_str(),
        "Scalar_InterpolatedString" | "Expr_ShellExec"
    ) {
        context.doc = if result.tokens().tokens()[first].kind() == K::StartHeredoc {
            let closing = result.tokens().text(last - 1).unwrap();
            let count = closing
                .iter()
                .take_while(|b| matches!(b, b' ' | b'\t'))
                .count();
            Some((first, last - 1, closing[..count].to_vec()))
        } else {
            None
        };
    }
    let mut spelling = spelling(&kind, result, first, last, start, end, &context);
    if kind == "PropertyHook" {
        if let Some(p) = context.property.clone() {
            spelling["propertyName"] = p;
        }
    }
    out.push(json!({"path":path,"origin":origin(source,start,end,first,last),"comments":comments,"spelling":spelling}));
    context.property = match kind.as_str() {
        "Stmt_Property" => {
            Some(value["fields"]["props"][0]["fields"]["name"]["fields"]["name"].clone())
        }
        "Param" => Some(value["fields"]["var"]["fields"]["name"].clone()),
        _ => context.property,
    };
    let fields = FIELDS.get_or_init(|| {
        serde_json::from_str(include_str!("../fixtures/fidelity/fields.json")).unwrap()
    });
    for key in fields[&kind]
        .as_array()
        .unwrap_or_else(|| panic!("unknown node {kind}"))
        .iter()
        .map(|v| v.as_str().unwrap())
    {
        collect(
            &mut value["fields"][key],
            &format!("{path}/fields/{key}"),
            result,
            out,
            context.clone(),
        );
    }
}
#[allow(
    clippy::too_many_lines,
    reason = "exhaustive reference spelling projection"
)]
fn spelling(
    kind: &str,
    result: &ParseResult,
    first: usize,
    last: usize,
    start: usize,
    end: usize,
    context: &Context,
) -> Value {
    let raw = &result.source().as_bytes()[start..end];
    let lower = raw.to_ascii_lowercase();
    let mut out = json!({});
    let tokens = &result.tokens().tokens()[first..last];
    let token_kind = tokens.first().map(|t| t.kind());
    match kind {
        "Scalar_Int" | "Scalar_Float" if matches!(token_kind, Some(K::Integer | K::Float)) => {
            if pprs::literal::decode_number(raw).is_err() {
                return out;
            }
            out["rawValue"] = bytes(raw);
            if kind == "Scalar_Int" {
                out["kind"] = integer(if lower.starts_with(b"0x") {
                    16
                } else if lower.starts_with(b"0b") {
                    2
                } else if lower.len() > 1 && lower[0] == b'0' {
                    8
                } else {
                    10
                });
            }
        }
        "Expr_Array" | "Expr_List" => {
            out["kind"] = integer(if raw.first() == Some(&b'[') { 2 } else { 1 });
        }
        "Expr_Exit" => {
            out["kind"] = integer(if lower.starts_with(b"die") { 2 } else { 1 });
        }
        "Expr_Cast_Int" => {
            out["kind"] = integer(if lower.windows(7).any(|w| w == b"integer") {
                2
            } else {
                1
            });
        }
        "Expr_Cast_Bool" => {
            out["kind"] = integer(if lower.windows(7).any(|w| w == b"boolean") {
                2
            } else {
                1
            });
        }
        "Expr_Cast_String" => {
            out["kind"] = integer(if lower.windows(6).any(|w| w == b"binary") {
                2
            } else {
                1
            });
        }
        "Expr_Cast_Double" => {
            out["kind"] = integer(if lower.windows(5).any(|w| w == b"float") {
                2
            } else if lower.windows(4).any(|w| w == b"real") {
                3
            } else {
                1
            });
        }
        "Stmt_Namespace" => {
            out["kind"] = integer(
                if tokens
                    .iter()
                    .find(|t| matches!(t.kind(), K::LeftBrace | K::Semicolon))
                    .is_some_and(|t| t.kind() == K::LeftBrace)
                {
                    2
                } else {
                    1
                },
            );
        }
        "Stmt_InlineHTML" => {
            out["hasLeadingNewline"] = json!(
                first == 0
                    || result
                        .tokens()
                        .text(first - 1)
                        .is_some_and(|t| t.contains(&b'\n') || t.contains(&b'\r'))
            );
        }
        "InterpolatedStringPart" => {
            let text = context.doc.as_ref().map_or_else(
                || raw.to_vec(),
                |(open, close, indent)| doc_raw(raw, indent, first == open + 1, last == *close),
            );
            out["rawValue"] = bytes(&text);
        }
        "Scalar_String" | "Scalar_InterpolatedString" => {
            if token_kind == Some(K::StartHeredoc) {
                let opening = result.tokens().text(first).unwrap();
                let closing = result.tokens().text(last - 1).unwrap();
                out["kind"] = integer(if opening.contains(&b'\'') { 4 } else { 3 });
                let label = opening.iter().position(|b| *b == b'<').unwrap() + 3;
                let label = &opening[label..];
                let label = label.trim_ascii();
                let label = label
                    .strip_prefix(b"'")
                    .or_else(|| label.strip_prefix(b"\""))
                    .unwrap_or(label);
                let label = label
                    .strip_suffix(b"'")
                    .or_else(|| label.strip_suffix(b"\""))
                    .unwrap_or(label);
                out["docLabel"] = bytes(label);
                let indent = closing
                    .iter()
                    .take_while(|b| matches!(b, b' ' | b'\t'))
                    .count();
                out["docIndentation"] = bytes(&closing[..indent]);
                if kind == "Scalar_String" {
                    let body_start = tokens[0].span().end();
                    let body_end = tokens.last().unwrap().span().start();
                    let body = &result.source().as_bytes()[body_start..body_end];
                    out["rawValue"] = bytes(&doc_raw(body, &closing[..indent], true, true));
                }
            } else if token_kind == Some(K::QuotedString) {
                let quote = raw
                    .iter()
                    .copied()
                    .find(|b| matches!(b, b'\'' | b'"'))
                    .unwrap();
                out["kind"] = integer(if quote == b'\'' { 1 } else { 2 });
                out["rawValue"] = bytes(raw);
            } else if token_kind == Some(K::DoubleQuote) {
                out["kind"] = integer(2);
            }
        }
        _ => {}
    }
    out
}
fn doc_raw(body: &[u8], indent: &[u8], first: bool, last: bool) -> Vec<u8> {
    let indent = if indent.contains(&b' ') && indent.contains(&b'\t') {
        &[][..]
    } else {
        indent
    };
    let mut out = Vec::new();
    let mut at = 0;
    while at < body.len() {
        if (at == 0 && first) || (at > 0 && body[at - 1] == b'\n') {
            let mut n = 0;
            while n < indent.len() && body.get(at).is_some_and(|b| matches!(b, b' ' | b'\t')) {
                at += 1;
                n += 1;
            }
        }
        if let Some(b) = body.get(at) {
            out.push(*b);
            at += 1;
        }
    }
    if last {
        if out.ends_with(b"\r\n") {
            out.truncate(out.len() - 2);
        } else if out.last().is_some_and(|b| matches!(b, b'\r' | b'\n')) {
            out.pop();
        }
    }
    out
}
