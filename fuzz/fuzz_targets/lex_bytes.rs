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
        limits: Limits::new(65542, 128, 4096, 4096, 32).unwrap(),
    };
    // Exercise both file mode and PHP mode; arbitrary HTML alone rarely reaches
    // the regular scanner without a seed containing an opening tag.
    let php = [b"<?php ".as_slice(), data].concat();
    for input in [data, php.as_slice()] {
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
        assert_eq!(result.is_complete(), result.tokens().is_complete());
        assert!(result.tokens().tokens().len() <= 4096);
        assert!(result.diagnostics().len() <= 32);
        for diagnostic in result.diagnostics() {
            assert!(result.source().slice(diagnostic.primary).is_some());
        }
    }
});
