//! Test-only normalization contract and comparison gate; no public parser API.

use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const CHANNELS: [&str; 4] = ["tokens", "structure", "positions", "diagnostics"];

pub fn bytes(value: &Value) -> Vec<u8> {
    let object = value.as_object().expect("tagged byte object");
    assert_eq!(object.len(), 1, "byte object has unexpected fields");
    hex::decode(value["bytes"].as_str().expect("hex bytes")).expect("valid hex")
}

pub fn coordinate(source: &[u8], offset: usize) -> [usize; 2] {
    assert!(offset <= source.len());
    let mut starts = vec![0];
    let mut cursor = 0;
    while cursor < source.len() {
        match source[cursor] {
            b'\r' if source.get(cursor + 1) == Some(&b'\n') => {
                cursor += 2;
                starts.push(cursor);
            }
            b'\n' | b'\r' => {
                cursor += 1;
                starts.push(cursor);
            }
            _ => cursor += 1,
        }
    }
    let line = starts.partition_point(|&start| start <= offset) - 1;
    [line + 1, offset - starts[line] + 1]
}

fn range(value: &Value, length: usize) -> [usize; 2] {
    let values = value.as_array().expect("span array");
    assert_eq!(values.len(), 2);
    let start = usize::try_from(values[0].as_u64().expect("start offset")).unwrap();
    let end = usize::try_from(values[1].as_u64().expect("end offset")).unwrap();
    assert!(
        start <= end && end <= length,
        "span {start}..{end}/{length}"
    );
    [start, end]
}

fn check_coordinates(value: &Value, source: &[u8], start: usize, end: usize) {
    assert_eq!(value["start"], json!(coordinate(source, start)));
    assert_eq!(value["end"], json!(coordinate(source, end)));
}

fn validate_origin(origin: &Value, source: &[u8], token_count: usize) {
    let [start, end] = range(&origin["span"], source.len());
    range(&origin["token_span"], token_count);
    assert_eq!(origin["empty"], start == end);
    check_coordinates(origin, source, start, end);
}

fn ast_paths(value: &Value, path: &str, paths: &mut BTreeSet<String>) {
    match value {
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                ast_paths(child, &format!("{path}/{index}"), paths);
            }
        }
        Value::Object(object) if object.contains_key("kind") => {
            assert_eq!(object.len(), 2);
            assert!(value["kind"].is_string());
            assert!(paths.insert(path.to_owned()));
            for (name, child) in value["fields"].as_object().expect("node fields") {
                ast_paths(child, &format!("{path}/fields/{name}"), paths);
            }
        }
        Value::Object(object) => {
            assert_eq!(object.len(), 1);
            if object.contains_key("bytes") {
                bytes(value);
            } else if let Some(integer) = object.get("integer") {
                integer
                    .as_str()
                    .unwrap()
                    .parse::<i64>()
                    .expect("64-bit integer");
            } else {
                let bits = hex::decode(value["float64"].as_str().expect("binary64 tag")).unwrap();
                assert_eq!(bits.len(), 8);
            }
        }
        Value::Null | Value::Bool(_) => {}
        _ => panic!("untagged semantic value: {value}"),
    }
}

pub fn validate_observation(observation: &Value, source: &[u8]) {
    let tokens = observation["tokens"].as_array().expect("tokens");
    assert_ne!(tokens.len(), 0);
    let mut cursor = 0;
    for (index, token) in tokens.iter().enumerate() {
        let [start, end] = range(&token["span"], source.len());
        assert_eq!(start, cursor);
        assert_eq!(bytes(&token["text"]), source[start..end]);
        check_coordinates(token, source, start, end);
        if index + 1 == tokens.len() {
            assert_eq!(token["kind"], "EOF");
            assert_eq!([start, end], [source.len(), source.len()]);
        } else {
            assert_ne!(token["kind"], "EOF");
            assert!(start < end);
        }
        cursor = end;
    }
    let mut paths = BTreeSet::new();
    ast_paths(&observation["ast"], "", &mut paths);
    let metadata = observation["metadata"].as_array().expect("metadata");
    assert_eq!(paths.len(), metadata.len());
    for entry in metadata {
        assert!(
            paths.remove(entry["path"].as_str().unwrap()),
            "duplicate/missing node metadata"
        );
        validate_origin(&entry["origin"], source, tokens.len());
        for comment in entry["comments"].as_array().unwrap() {
            validate_origin(&comment["origin"], source, tokens.len());
            let [start, end] = range(&comment["origin"]["span"], source.len());
            assert_eq!(bytes(&comment["text"]), source[start..end]);
        }
    }
    let diagnostics = observation["diagnostics"].as_array().expect("diagnostics");
    for diagnostic in diagnostics {
        assert!(
            ["lexical", "syntax", "contextual"].contains(&diagnostic["category"].as_str().unwrap())
        );
        assert!(diagnostic["code"].is_string());
        bytes(&diagnostic["message"]);
        if !diagnostic["span"].is_null() {
            let [start, end] = range(&diagnostic["span"], source.len());
            check_coordinates(diagnostic, source, start, end);
        }
    }
    let expected_status = if observation["ast"].is_null() {
        assert_ne!(diagnostics.len(), 0);
        "aborted"
    } else if diagnostics.is_empty() {
        "clean"
    } else {
        "recovered"
    };
    assert_eq!(observation["status"], expected_status);
}

// Only these explicitly non-comparable provenance attributes are removed.
// Spelling, comments, coordinates, optional fields and recovery status remain.
fn comparable(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| key.as_str() != "raw_attributes")
                .map(|(key, value)| (key.clone(), comparable(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(comparable).collect()),
        _ => value.clone(),
    }
}

pub fn project(observation: &Value, channel: &str) -> Result<Value, String> {
    let fields: &[&str] = match channel {
        "tokens" => &["tokens"],
        "structure" => &["status", "ast"],
        "positions" => &["metadata"],
        "diagnostics" => &["status", "diagnostics"],
        _ => return Err(format!("unknown comparison channel {channel}")),
    };
    let mut projected = serde_json::Map::new();
    for &field in fields {
        let value = observation
            .get(field)
            .ok_or_else(|| format!("missing observation field {field}"))?;
        projected.insert(field.to_owned(), comparable(value));
    }
    Ok(Value::Object(projected))
}

fn first_difference(expected: &Value, actual: &Value, path: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    match (expected, actual) {
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => left
            .iter()
            .zip(right)
            .enumerate()
            .find_map(|(i, (a, b))| first_difference(a, b, &format!("{path}/{i}"))),
        (Value::Object(left), Value::Object(right)) if left.keys().eq(right.keys()) => {
            left.iter().find_map(|(key, value)| {
                first_difference(value, &right[key], &format!("{path}/{key}"))
            })
        }
        _ => Some(path.to_owned()),
    }
}

pub fn compare(expected: &Value, actual: &Value, channel: &str) -> Result<(), String> {
    let target = expected
        .get("target")
        .and_then(Value::as_str)
        .ok_or("reference observation has no target")?;
    if actual.get("target").and_then(Value::as_str) != Some(target) {
        return Err("comparison target mismatch".into());
    }
    let left = project(expected, channel)?;
    let right = project(actual, channel)?;
    match first_difference(&left, &right, "") {
        Some(path) => Err(format!("{channel} mismatch at {path}")),
        None => Ok(()),
    }
}

pub fn gate(
    expected: &Value,
    actual: Option<&Value>,
    channel: &str,
    state: &Value,
    difference: Option<&Value>,
) -> Result<(), String> {
    match state["state"].as_str() {
        Some("pending" | "deferred") => Ok(()),
        Some("passing") => compare(
            expected,
            actual.ok_or("passing case has no implementation adapter")?,
            channel,
        ),
        Some("intentional-difference") => {
            if state["reason"].as_str().is_none_or(str::is_empty) {
                return Err("intentional difference needs a reason".into());
            }
            let difference =
                difference.ok_or("intentional difference needs an exact expectation")?;
            if difference.get("target") != expected.get("target") {
                return Err("intentional difference cannot change the target".into());
            }
            if compare(expected, difference, channel).is_ok() {
                return Err("intentional difference does not differ from reference".into());
            }
            compare(
                difference,
                actual.ok_or("intentional difference has no implementation adapter")?,
                channel,
            )
        }
        _ => Err("unknown coverage state".into()),
    }
}
