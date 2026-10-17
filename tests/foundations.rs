//! Public P04 contracts, independent of PHP and the production lexer/parser.

use pprs::{
    ByteString, Limits, ParseOptions, PhpVersion, SourceText,
    ast::{
        Expr, ExprKind, Identifier, IntegerBase, MetadataError, Name, NodeMeta, Origin,
        Qualification, Stmt, StmtKind, StringKind,
    },
    diagnostic::{Category, Diagnostic, DiagnosticCode},
    options::Resource,
    source::Location,
    token::{Token, TokenKind, TokenStream, TokenStreamError},
};
use std::{collections::BTreeSet, sync::Arc};

#[test]
fn coordinates_cover_all_line_endings_and_byte_offsets() {
    let source = SourceText::from(&b"a\r\nb\rc\n\xff\0"[..]);
    assert_eq!(source.line_starts(), &[0, 3, 5, 7]);
    let expected = [
        (1, 1),
        (1, 2),
        (1, 3),
        (2, 1),
        (2, 2),
        (3, 1),
        (3, 2),
        (4, 1),
        (4, 2),
        (4, 3),
    ];
    for (offset, (line, column)) in expected.into_iter().enumerate() {
        assert_eq!(source.location(offset), Some(Location { line, column }));
    }
    assert_eq!(source.location(10), None);
    assert_eq!(source.location(usize::MAX), None);
    for bytes in [&b""[..], &b"\n"[..], &b"\r"[..], &b"\r\n"[..]] {
        let source = SourceText::from(bytes);
        assert_eq!(source.location(source.len()).unwrap().column, 1);
        assert_eq!(
            source.location(source.len()).unwrap().line,
            if bytes.is_empty() { 1 } else { 2 }
        );
    }
    // A UTF-8 character occupies two byte columns, not one display column.
    assert_eq!(
        SourceText::from("é").location(2),
        Some(Location { line: 1, column: 3 })
    );
}

#[test]
fn spans_check_order_bounds_and_foreign_sources_without_overflow() {
    let source = SourceText::from("abc");
    for start in 0..=3 {
        for end in start..=3 {
            let span = source.span(start..end).unwrap();
            assert_eq!(span.len(), end - start);
            assert_eq!(span.is_empty(), start == end);
            assert_eq!(source.slice(span), Some(&source.as_bytes()[start..end]));
        }
    }
    let reversed = std::ops::Range { start: 2, end: 1 };
    for range in [reversed, 0..4, usize::MAX..usize::MAX] {
        assert!(source.span(range).is_err());
    }
    let empty = SourceText::from("");
    assert_eq!(empty.slice(empty.eof()), Some(&b""[..]));
    assert_eq!(empty.slice(source.span(0..1).unwrap()), None);
    assert_eq!(source.eof().range(), 3..3);
}

#[test]
fn shared_sources_and_semantic_bytes_keep_exact_storage() {
    let bytes: Arc<[u8]> = Arc::from(&b"\xff\0\\\r\n"[..]);
    let pointer = bytes.as_ptr();
    let source = SourceText::from_shared(Arc::clone(&bytes));
    let clone = source.clone();
    assert_eq!(source.as_bytes().as_ptr(), pointer);
    assert_eq!(clone.as_bytes().as_ptr(), pointer);
    assert_eq!(source.line_starts().as_ptr(), clone.line_starts().as_ptr());
    drop(bytes);
    drop(source);
    assert_eq!(clone.as_bytes(), b"\xff\0\\\r\n");
    let semantic = ByteString::from(clone.as_bytes());
    assert_eq!(semantic.as_bytes(), clone.as_bytes());
    let debug = format!("{semantic:?}");
    assert!(debug.contains("\\xff"), "{debug}");
    assert!(!debug.contains('\u{fffd}'));
    assert!(format!("{clone:?}").contains("\\xff"));
    for byte in 0..=u8::MAX {
        let source = SourceText::from(vec![byte]);
        assert_eq!(source.slice(source.span(0..1).unwrap()), Some(&[byte][..]));
        assert_eq!(ByteString::from(vec![byte]).as_bytes(), &[byte]);
    }
}

fn stream(input: &[u8], parts: &[(TokenKind, usize)]) -> TokenStream {
    let source = SourceText::from(input);
    let mut start = 0;
    let mut tokens = Vec::new();
    for &(kind, end) in parts {
        tokens.push(Token::new(kind, source.span(start..end).unwrap()));
        start = end;
    }
    tokens.push(Token::new(TokenKind::Eof, source.eof()));
    TokenStream::new(source, tokens).unwrap()
}

#[test]
fn streams_reconstruct_bytes_and_handle_empty_origins_and_eof() {
    let stream = stream(
        b" x\xff",
        &[
            (TokenKind::Whitespace, 1),
            (TokenKind::Identifier, 2),
            (TokenKind::Invalid, 3),
        ],
    );
    let reconstructed: Vec<u8> = (0..stream.tokens().len())
        .flat_map(|index| stream.text(index).unwrap())
        .copied()
        .collect();
    assert_eq!(reconstructed, stream.source().as_bytes());
    assert!(stream.is_complete()); // Invalid is scanned, unlike Unscanned.
    assert_eq!(stream.text(3), Some(&b""[..]));
    assert_eq!(stream.text(4), None);
    assert_eq!(
        stream.span(stream.range(1..1).unwrap()).unwrap().range(),
        1..1
    );
    assert_eq!(
        stream.span(stream.range(4..4).unwrap()).unwrap(),
        stream.source().eof()
    );
    assert_eq!(
        stream.span(stream.range(1..3).unwrap()).unwrap().range(),
        1..3
    );
    assert!(stream.range(0..5).is_err());
    let reversed = std::ops::Range { start: 2, end: 1 };
    assert!(stream.range(reversed).is_err());
    assert!(stream.range(usize::MAX..usize::MAX).is_err());
    let empty = TokenStream::new(
        SourceText::from(""),
        vec![Token::new(TokenKind::Eof, SourceText::from("").eof())],
    )
    .unwrap();
    assert!(empty.span(stream.range(0..3).unwrap()).is_err());
}

#[test]
fn stream_validation_rejects_gaps_overlap_empty_tokens_and_misplaced_tails() {
    let source = SourceText::from("ab");
    let token = |kind, range| Token::new(kind, source.span(range).unwrap());
    let eof = Token::new(TokenKind::Eof, source.eof());
    let longer = SourceText::from("abcd");
    let invalid = [
        vec![],
        vec![token(TokenKind::Identifier, 0..2)],
        vec![token(TokenKind::Identifier, 1..2), eof],
        vec![
            token(TokenKind::Identifier, 0..1),
            token(TokenKind::Identifier, 0..2),
            eof,
        ],
        vec![
            token(TokenKind::Invalid, 0..0),
            token(TokenKind::Identifier, 0..2),
            eof,
        ],
        vec![token(TokenKind::Identifier, 0..2), eof, eof],
        vec![token(TokenKind::Eof, 0..2)],
        vec![
            token(TokenKind::Unscanned, 0..1),
            token(TokenKind::Identifier, 1..2),
            eof,
        ],
        vec![
            token(TokenKind::HaltTail, 0..1),
            token(TokenKind::Identifier, 1..2),
            eof,
        ],
        vec![
            Token::new(TokenKind::Identifier, longer.span(0..4).unwrap()),
            eof,
        ],
    ];
    for tokens in invalid {
        assert!(TokenStream::new(source.clone(), tokens).is_err());
    }
    assert_eq!(
        TokenStream::new(source.clone(), vec![eof]).unwrap_err(),
        TokenStreamError::InvalidToken(0)
    );
    for kind in [TokenKind::Unscanned, TokenKind::HaltTail] {
        let stream = TokenStream::new(source.clone(), vec![token(kind, 0..2), eof]).unwrap();
        assert_eq!(stream.is_complete(), kind == TokenKind::HaltTail);
    }
}

#[test]
fn metadata_retains_comments_and_distinguishes_empty_and_synthetic_origins() {
    let stream = stream(
        b"# x",
        &[
            (TokenKind::Comment, 1),
            (TokenKind::Whitespace, 2),
            (TokenKind::Identifier, 3),
        ],
    );
    let meta = NodeMeta::parsed(&stream, stream.range(2..3).unwrap(), vec![0]).unwrap();
    assert_eq!(meta.comments(), &[0]);
    let Origin::Parsed { span, tokens } = meta.origin() else {
        panic!("parsed origin");
    };
    assert_eq!(span.range(), 2..3);
    assert_eq!(tokens.range(), 2..3);
    let empty = NodeMeta::parsed(&stream, stream.range(3..3).unwrap(), vec![]).unwrap();
    let Origin::Parsed { span, .. } = empty.origin() else {
        panic!("empty parsed origin");
    };
    assert_eq!(span, stream.source().eof());
    assert_eq!(NodeMeta::synthetic().origin(), Origin::Synthetic);
    for comments in [vec![1], vec![20], vec![0, 0]] {
        assert!(matches!(
            NodeMeta::parsed(&stream, tokens, comments),
            Err(MetadataError::Comment(_))
        ));
    }
    let unscanned = TokenStream::new(
        stream.source().clone(),
        vec![
            Token::new(TokenKind::Unscanned, stream.source().span(0..3).unwrap()),
            Token::new(TokenKind::Eof, stream.source().eof()),
        ],
    )
    .unwrap();
    assert_eq!(
        NodeMeta::parsed(&unscanned, unscanned.range(0..1).unwrap(), vec![]),
        Err(MetadataError::Unscanned)
    );
}

#[test]
fn ast_values_are_owned_bytes_and_preserve_literal_spelling() {
    let meta = NodeMeta::synthetic();
    let identifier = Identifier::new(ByteString::from(&b"\xff"[..]), meta.clone()).unwrap();
    assert_eq!(identifier.bytes().as_bytes(), &[0xff]);
    for invalid in ["", "A\\B"] {
        assert!(Identifier::new(invalid, meta.clone()).is_err());
    }
    let name = Name::new(
        vec!["A".into(), ByteString::from(&b"\xff"[..])],
        Qualification::FullyQualified,
        meta.clone(),
    )
    .unwrap();
    assert_eq!(name.parts().len(), 2);
    assert_eq!(name.qualification(), Qualification::FullyQualified);
    assert!(Name::new(vec![], Qualification::Normal, meta.clone()).is_err());
    assert!(Name::new(vec!["".into()], Qualification::Relative, meta.clone()).is_err());
    let source = stream(b"'a\\n'", &[(TokenKind::QuotedString, 5)]);
    let literal = Expr {
        meta: NodeMeta::parsed(&source, source.range(0..1).unwrap(), vec![]).unwrap(),
        kind: ExprKind::String {
            value: "a\\n".into(),
            kind: StringKind::SingleQuoted,
        },
    };
    let ExprKind::String { value, .. } = literal.kind else {
        panic!("string");
    };
    assert_eq!(value.as_bytes(), b"a\\n");
    let Origin::Parsed { span, .. } = literal.meta.origin() else {
        panic!("spelling");
    };
    assert_eq!(source.source().slice(span), Some(&b"'a\\n'"[..]));
    let integer = ExprKind::Integer {
        value: i64::MAX,
        base: IntegerBase::Hexadecimal,
    };
    assert!(matches!(
        integer,
        ExprKind::Integer {
            value: i64::MAX,
            ..
        }
    ));
    let float = ExprKind::Float(f64::INFINITY);
    assert!(matches!(float, ExprKind::Float(value) if value.is_infinite()));
    let no_value = Stmt {
        meta: meta.clone(),
        kind: StmtKind::Return(None),
    };
    let bad_value = Stmt {
        meta: meta.clone(),
        kind: StmtKind::Return(Some(Expr {
            meta,
            kind: ExprKind::Error,
        })),
    };
    assert_ne!(no_value, bad_value);
}

#[test]
fn versions_and_budgets_are_explicit_validated_and_host_independent() {
    assert_eq!(PhpVersion::ALL.len(), 10);
    for version in PhpVersion::ALL {
        assert_eq!(version.as_str().parse(), Ok(version));
        assert_eq!(version.to_string(), version.as_str());
    }
    for invalid in [
        "", "5.6", "7.5", "8.5", "8.6", "8", "8.4.0", " 8.4", "PHP8.4",
    ] {
        assert!(invalid.parse::<PhpVersion>().is_err());
    }
    let options = ParseOptions::default();
    assert_eq!(options.version, PhpVersion::Php84);
    assert!(!options.short_open_tag);
    assert_eq!(options.limits.get(Resource::InputBytes), 16 * 1024 * 1024);
    assert!(Limits::new(0, 1, 2, 1, 1).is_ok());
    for limits in [
        Limits::new(0, 0, 2, 1, 1),
        Limits::new(0, 1, 1, 1, 1),
        Limits::new(0, 1, 2, 0, 1),
        Limits::new(0, 1, 2, 1, 0),
    ] {
        assert!(limits.is_err());
    }
    for resource in [
        Resource::InputBytes,
        Resource::Depth,
        Resource::Tokens,
        Resource::Nodes,
        Resource::Diagnostics,
    ] {
        let bound = options.limits.get(resource);
        assert_eq!(options.limits.check(resource, bound), Ok(()));
        assert_eq!(options.limits.check(resource, bound + 1), Err(resource));
        assert_eq!(options.limits.check(resource, usize::MAX), Err(resource));
    }
}

#[test]
fn diagnostic_codes_are_unique_and_expected_eof_differs_from_absence() {
    let codes = [
        DiagnosticCode::InvalidByte,
        DiagnosticCode::Unterminated,
        DiagnosticCode::UnterminatedString,
        DiagnosticCode::UnexpectedToken,
        DiagnosticCode::InvalidContext,
        DiagnosticCode::ResourceLimit(Resource::InputBytes),
        DiagnosticCode::ResourceLimit(Resource::Depth),
        DiagnosticCode::ResourceLimit(Resource::Tokens),
        DiagnosticCode::ResourceLimit(Resource::Nodes),
        DiagnosticCode::ResourceLimit(Resource::Diagnostics),
    ];
    assert_eq!(
        codes
            .map(DiagnosticCode::as_str)
            .into_iter()
            .collect::<BTreeSet<_>>()
            .len(),
        codes.len()
    );
    assert_eq!(DiagnosticCode::InvalidByte.category(), Category::Lexical);
    let mut diagnostic = Diagnostic::error(
        DiagnosticCode::UnexpectedToken,
        SourceText::from("").eof(),
        "missing expression",
    );
    assert_eq!(diagnostic.found, None);
    diagnostic.found = Some(TokenKind::Eof);
    diagnostic.expected.push(TokenKind::Variable);
    assert_eq!(diagnostic.found, Some(TokenKind::Eof));
}

#[test]
fn live_node_ledger_preserves_inventory_and_requires_evidence_for_parser_progress() {
    let parse = |input: &str| -> Vec<serde_json::Value> {
        input
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    };
    let frozen = parse(include_str!("../docs/compatibility/nodes.jsonl"));
    let live = parse(include_str!("../docs/implementation/nodes.jsonl"));
    assert_eq!(live.len(), frozen.len());
    let paths: BTreeSet<_> = live
        .iter()
        .map(|row| row["reference_path"].as_str().unwrap())
        .collect();
    assert_eq!(paths.len(), live.len());
    for baseline in &frozen {
        let row = live
            .iter()
            .find(|row| row["reference_path"] == baseline["path"])
            .unwrap();
        assert_eq!(row["scope"], baseline["scope"]);
        assert_ne!(row["note"].as_str().unwrap(), "");
        if row["scope"] == "deferred" {
            assert_eq!(row["model_status"], "deferred");
            assert_eq!(row["parser_status"], "deferred");
            assert!(row["parser_phase"].is_null());
            assert!(row["rust_model"].is_null());
        } else {
            assert!(["pending", "slice"].contains(&row["parser_status"].as_str().unwrap()));
            if row["parser_status"] == "slice" {
                assert!(["P07", "P08", "P09"].contains(&row["parser_phase"].as_str().unwrap()));
                for path in row["evidence"].as_array().unwrap() {
                    assert!(
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join(path.as_str().unwrap())
                            .is_file()
                    );
                }
                assert_eq!(row["model_status"], "foundation");
            }
            assert!(["P07", "P08", "P09"].contains(&row["parser_phase"].as_str().unwrap()));
            match row["model_status"].as_str().unwrap() {
                "foundation" => assert_ne!(row["rust_model"].as_str().unwrap(), ""),
                "pending" => assert!(row["rust_model"].is_null()),
                other => panic!("unknown model state {other}"),
            }
        }
    }
}
