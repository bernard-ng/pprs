#![no_main]
use libfuzzer_sys::fuzz_target;
use pprs::{Limits, ParseOptions, PhpVersion, lex, token::TokenKind};

fuzz_target!(|data: &[u8]| {
    if data.len() > 65536 {
        return;
    }
    let selector = data.first().copied().unwrap_or(0);
    let options = ParseOptions {
        version: PhpVersion::ALL[usize::from(selector) % 10],
        short_open_tag: selector & 16 != 0,
        limits: Limits::new(65600, 1 + usize::from(selector % 64), 4096, 4096, 32).unwrap(),
    };
    // Seed each lexical mode; random file bytes alone rarely reach interpolation.
    for prefix in [
        b"".as_slice(),
        b"<?php ",
        b"<?php '",
        b"<?php \"",
        b"<?php `",
        b"<?php <<<X\n",
        b"<?php <<<'X'\n",
        b"<?php \"{$x[",
        b"<?php \"${",
        b"<?php \"$x[",
    ] {
        let source = [prefix, data].concat();
        let input = source.as_slice();
        let result = lex(input, options).unwrap();
        let mut offset = 0;
        for (index, token) in result.tokens().tokens().iter().enumerate() {
            assert_eq!(token.span().start(), offset);
            offset = token.span().end();
            assert_eq!(
                result.tokens().text(index).unwrap(),
                &input[token.span().range()]
            );
            assert!(token.kind() == TokenKind::Eof || !token.span().is_empty());
        }
        assert_eq!(offset, input.len());
        assert_eq!(
            result.tokens().tokens().last().unwrap().kind(),
            TokenKind::Eof
        );
        if result.is_complete() {
            assert!(result.tokens().is_complete());
        }
        assert!(result.tokens().tokens().len() <= 4096);
        assert!(result.diagnostics().len() <= 32);
        for diagnostic in result.diagnostics() {
            assert!(result.source().slice(diagnostic.primary).is_some());
        }
    }
    for mode in [
        pprs::literal::EscapeMode::DoubleQuoted,
        pprs::literal::EscapeMode::Heredoc,
        pprs::literal::EscapeMode::Nowdoc,
    ] {
        match pprs::literal::decode_escapes(data, mode) {
            Ok(value) => assert!(value.as_bytes().len() <= data.len()),
            Err(error) => {
                assert!(error.range.start <= error.range.end && error.range.end <= data.len())
            }
        }
    }
    let _ = pprs::literal::decode_number(data);
});
