//! Compile-check the public library dependency from an independent Cargo package.

pub use pprs;

use pprs::{
    ParseOptions, SourceText,
    ast::{Identifier, NodeMeta},
    source::RangeError,
};

/// Copy caller bytes into a source that outlives the input, then inspect EOF.
///
/// # Errors
/// Propagates a range-construction error.
pub fn retain_source(input: &[u8]) -> Result<SourceText, RangeError> {
    let source = SourceText::from(input);
    let eof = source.span(input.len()..input.len())?;
    assert_eq!(source.slice(eof), Some(&b""[..]));
    Ok(source)
}

/// Build options and a synthetic identifier through public types only.
///
/// # Panics
/// Panics only if the fixed, valid identifier is rejected by the library.
#[must_use]
pub fn configure() -> (ParseOptions, Identifier) {
    let identifier = Identifier::new("consumer", NodeMeta::synthetic())
        .expect("nonempty identifier without namespace separators");
    (ParseOptions::default(), identifier)
}

/// Tokenize a file using the production lexer, retaining owned output.
///
/// # Errors
/// Returns an input-size error before allocation for oversized input.
pub fn tokenize(input: &[u8]) -> Result<pprs::LexResult, pprs::lexer::InputTooLarge> {
    pprs::lex(input, ParseOptions::default())
}

/// Decode a constant quoted token without exposing a dependency's byte type.
///
/// # Errors
/// Returns an input-relative literal error for malformed spelling or escapes.
pub fn decode_string(raw: &[u8]) -> Result<pprs::ByteString, pprs::literal::LiteralError> {
    pprs::literal::decode_quoted(raw)
}

/// Parse PHP and inspect a clean, owned syntax tree through public APIs.
///
/// # Errors
/// Returns preflight, syntax or resource failures with their available context.
pub fn parse_and_count(input: &[u8]) -> Result<usize, pprs::StrictError> {
    let parsed = pprs::parse_strict(input, ParseOptions::default())?;
    Ok(parsed
        .result()
        .program()
        .map_or(0, |program| pprs::visit::walk(program).count()))
}

/// Inspect declared property hook names through the downstream public API.
///
/// # Errors
/// Returns preflight, syntax or resource failures.
pub fn property_hook_names(input: &[u8]) -> Result<Vec<pprs::ByteString>, pprs::StrictError> {
    let parsed = pprs::parse_strict(input, ParseOptions::default())?;
    Ok(parsed.result().program().map_or_else(Vec::new, |program| {
        pprs::visit::walk(program)
            .filter_map(|entry| match entry.node {
                pprs::visit::Node::PropertyHook(hook) => Some(hook.name.bytes().clone()),
                _ => None,
            })
            .collect()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pprs::{DiagnosticCode, Limits, ParseStatus, StrictError, ast::StmtKind, visit::Node};

    #[test]
    fn caller_reads_a_file_and_collects_function_names_and_comments() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/example.php");
        let bytes = std::fs::read(path).unwrap();
        let file = pprs::parse_strict(bytes, ParseOptions::default()).unwrap();
        let mut names = Vec::new();
        for entry in pprs::visit::walk(file.program()) {
            if let Node::Stmt(stmt) = entry.node {
                if let StmtKind::Function(function) = &stmt.kind {
                    names.push(function.name.bytes().as_bytes());
                    let meta = entry.node.meta().unwrap();
                    assert_eq!(meta.comments().len(), 1);
                    assert_eq!(
                        file.result().tokens().text(meta.comments()[0]),
                        Some(b"/** Entry point. */".as_slice())
                    );
                    let pprs::ast::Origin::Parsed { span, tokens } = meta.origin() else {
                        panic!()
                    };
                    assert_eq!(file.result().tokens().span(tokens).unwrap(), span);
                    assert!(
                        file.result()
                            .source()
                            .slice(span)
                            .unwrap()
                            .starts_with(b"function greet(")
                    );
                }
            }
        }
        assert_eq!(names, [b"greet".as_slice()]);
    }

    #[test]
    fn owned_results_and_shared_source_outlive_caller_buffers() {
        let result = {
            let bytes = b"<?php $\xff = '\xfe';".to_vec();
            pprs::parse(&bytes, ParseOptions::default()).unwrap()
        };
        assert_eq!(result.status(), ParseStatus::Clean);
        assert_eq!(result.source().as_bytes(), b"<?php $\xff = '\xfe';");
        let source = result.source().clone();
        let second = pprs::parse_source(source.clone(), ParseOptions::default()).unwrap();
        assert_eq!(
            source.as_bytes().as_ptr(),
            second.source().as_bytes().as_ptr()
        );
        let strict = second.into_strict().unwrap();
        assert_eq!(strict.program().statements.len(), 1);
        assert_eq!(strict.into_result().status(), ParseStatus::Clean);
        assert!(retain_source(b"\xff").is_ok());
        assert_eq!(decode_string(b"'text'").unwrap().as_bytes(), b"text");
        assert_eq!(configure().1.bytes().as_bytes(), b"consumer");
    }

    #[test]
    fn strict_failures_keep_recovery_but_preflight_has_no_tree() {
        let error =
            pprs::parse_strict("<?php $x = ; echo 2;", ParseOptions::default()).unwrap_err();
        let StrictError::Parse(error) = error else {
            panic!()
        };
        assert_eq!(error.result().status(), ParseStatus::Recovered);
        assert!(
            error
                .result()
                .program()
                .unwrap()
                .statements
                .iter()
                .any(|s| matches!(s.kind, StmtKind::Echo(_)))
        );
        let recovered = error.into_result();
        let d = recovered
            .diagnostics()
            .iter()
            .find(|d| d.code == DiagnosticCode::UnexpectedToken)
            .unwrap();
        assert_eq!(d.code.as_str(), "PPRS-S001");
        assert_eq!(d.code.category(), pprs::Category::Syntax);
        assert!(recovered.source().slice(d.primary).is_some());
        assert!(d.found.is_some());
        let options = ParseOptions {
            limits: Limits::new(0, 1, 2, 1, 1).unwrap(),
            ..Default::default()
        };
        assert!(matches!(
            pprs::parse_strict("x", options),
            Err(StrictError::Input(pprs::InputTooLarge {
                length: 1,
                limit: 0
            }))
        ));
        assert!(
            pprs::parse_strict("", options)
                .unwrap()
                .program()
                .statements
                .is_empty()
        );
    }

    #[test]
    fn resource_abort_and_lexical_completion_are_distinct() {
        let options = ParseOptions {
            limits: Limits::new(1024, 128, 8, 250_000, 100).unwrap(),
            ..Default::default()
        };
        let result = pprs::parse("<?php $a=1; $b=2; $c=3;", options).unwrap();
        assert_eq!(result.status(), ParseStatus::Aborted);
        assert!(matches!(
            result.diagnostics().last().unwrap().code,
            DiagnosticCode::ResourceLimit(_)
        ));
        assert!(!result.tokens().is_complete());
        let lexed = tokenize(b"<?php \0 echo 1;").unwrap();
        assert!(lexed.is_complete());
        assert!(!lexed.diagnostics().is_empty());
        assert_eq!(
            lexed.tokens().tokens().last().unwrap().kind(),
            pprs::token::TokenKind::Eof
        );
    }

    #[test]
    fn results_support_shared_readers_and_independent_parsers() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<pprs::ParseResult>();
        send_sync::<pprs::ParsedFile>();
        send_sync::<pprs::SourceText>();
        send_sync::<pprs::LexResult>();
        send_sync::<pprs::ast::Program>();
        send_sync::<pprs::ParseOptions>();
        send_sync::<pprs::Diagnostic>();
        send_sync::<pprs::StrictError>();
        let file = std::sync::Arc::new(
            pprs::parse_strict("<?php function f() {}", ParseOptions::default()).unwrap(),
        );
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let file = file.clone();
                std::thread::spawn(move || {
                    assert_eq!(
                        parse_and_count(file.result().source().as_bytes()).unwrap(),
                        pprs::visit::walk(file.program()).count()
                    );
                    assert_eq!(file.program().statements.len(), 1);
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[test]
    fn targets_and_property_hooks_use_only_public_api() {
        for version in pprs::PhpVersion::ALL {
            assert_eq!(
                version.as_str().parse::<pprs::PhpVersion>().unwrap(),
                version
            );
            let options = ParseOptions {
                version,
                ..Default::default()
            };
            assert_eq!(
                pprs::parse("<?php echo 1;", options).unwrap().status(),
                ParseStatus::Clean
            );
        }
        assert!("8.4.0".parse::<pprs::PhpVersion>().is_err());
        let hooks =
            property_hook_names(b"<?php class C { public string $p { get => 'x'; } }").unwrap();
        assert_eq!(hooks[0].as_bytes(), b"get");
    }
}
