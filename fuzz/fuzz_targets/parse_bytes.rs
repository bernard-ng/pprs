#![no_main]
use libfuzzer_sys::fuzz_target;
use pprs::{Limits, ParseOptions, ParseStatus, PhpVersion, parse, visit};
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
    for prefix in [
        b"".as_slice(),
        b"<?php ",
        b"<?php $a = ",
        b"<?php f(",
        b"<?php function f(",
        b"<?php function f() {",
        b"<?php try {} catch (",
        b"<?php if ($x):",
        b"<?php switch ($x) {case 1:",
        b"<?php \"{$a[",
    ] {
        let input = [prefix, data].concat();
        let result = parse(&input, options).unwrap();
        let mut offset = 0;
        for (index, token) in result.tokens().tokens().iter().enumerate() {
            assert_eq!(token.span().start(), offset);
            offset = token.span().end();
            assert_eq!(
                result.tokens().text(index).unwrap(),
                &input[token.span().range()]
            );
        }
        assert_eq!(offset, input.len());
        assert!(result.tokens().tokens().len() <= 4096);
        assert!(result.diagnostics().len() <= 32);
        if let Some(program) = result.program() {
            assert!(visit::walk(program).count() <= 4096);
        }
        let clean = result.status() == ParseStatus::Clean;
        assert_eq!(result.into_strict().is_ok(), clean);
    }
});
