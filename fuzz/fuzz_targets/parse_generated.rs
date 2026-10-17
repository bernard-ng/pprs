#![no_main]
use libfuzzer_sys::fuzz_target;
#[path = "../../tests/support/invariants.rs"]
mod checks;
#[path = "../../tests/support/generated.rs"]
mod generated;
fuzz_target!(|data: &[u8]| {
    let source = generated::program(&data[..data.len().min(128)]);
    let options = pprs::ParseOptions {
        version: pprs::PhpVersion::ALL[usize::from(data.first().copied().unwrap_or(0)) % 10],
        ..Default::default()
    };
    let parsed = pprs::parse(source, options).unwrap();
    assert_eq!(parsed.status(), pprs::ParseStatus::Clean);
    checks::invariants(&parsed, options);
    let program = parsed.program().unwrap().clone();
    drop(program);
});
