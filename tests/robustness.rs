//! P12 bounded adversarial workloads and result lifecycle checks.
#[path = "support/parser.rs"]
mod adapter;
#[path = "support/invariants.rs"]
mod checks;
#[path = "support/generated.rs"]
mod generated;
use pprs::{Limits, ParseOptions, ParseStatus, diagnostic::DiagnosticCode as D, options::Resource};
use std::fmt::Write;

#[derive(Default)]
struct Sink(usize);
impl std::fmt::Write for Sink {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0 += s.len();
        Ok(())
    }
}

#[test]
fn deep_results_are_safe_to_visit_clone_format_project_and_drop() {
    // A real 2 MiB worker stack exercises parser, derived traits and teardown.
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for size in [16, 64, 110, 120, 128, 256, 4096] {
                for source in [
                    format!("<?php {}$x{};", "(".repeat(size), ")".repeat(size)),
                    format!("<?php {}$x;", "!".repeat(size)),
                    format!("<?php {};", vec!["$x"; size].join(" + ")),
                    format!("<?php {};", vec!["$x"; size].join(" = ")),
                    format!("<?php {}1{};", "[".repeat(size), "]".repeat(size)),
                    format!(
                        "<?php {}echo 1;{}",
                        "if ($x) {".repeat(size),
                        "}".repeat(size)
                    ),
                ] {
                    let options = ParseOptions {
                        limits: Limits::new(1_000_000, usize::MAX, 100_000, 100_000, 100).unwrap(),
                        ..Default::default()
                    };
                    let result = pprs::parse(&source, options).unwrap();
                    checks::invariants(&result, options);
                    if let Some(program) = result.program() {
                        assert!(pprs::visit::walk(program).all(|n| n.depth <= 128));
                        let copy = program.clone();
                        assert_eq!(&copy, program);
                        let mut sink = Sink::default();
                        write!(&mut sink, "{copy:?}").unwrap();
                        assert!(sink.0 > 0);
                        let observation = adapter::full_observation(&result, "8.4");
                        serde_json::to_writer(std::io::sink(), &observation).unwrap();
                        drop(observation);
                        drop(copy);
                    }
                    drop(result);
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn long_tokens_comments_strings_and_heredocs_remain_lossless() {
    let text = "x".repeat(1024 * 1024);
    for source in [
        format!("<?php /*{text}*/ echo 1;"),
        format!("<?php ${text}=1;"),
        format!("<?php '{text}';"),
        format!("<?php {}echo 1;", "\n".repeat(1024 * 1024)),
        format!("<?php $x=<<<DOC\n{text}\nDOC;"),
        format!("<?php $x=<<<'DOC'\n{text}\nDOC;"),
    ] {
        let options = ParseOptions::default();
        let result = pprs::parse(&source, options).unwrap();
        assert_eq!(result.status(), ParseStatus::Clean);
        checks::invariants(&result, options);
    }
}

#[test]
fn wide_lists_members_and_default_resource_exhaustion_are_bounded() {
    for source in [
        format!("<?php f({});", vec!["1"; 20_000].join(",")),
        format!(
            "<?php class C {{ {} }}",
            (0..10_000).fold(String::new(), |mut text, i| {
                write!(&mut text, "public $p{i};").unwrap();
                text
            })
        ),
        format!("<?php {}", "$x=1;".repeat(100_000)),
        format!("<?php {}echo 1;", " ".repeat(16 * 1024 * 1024 - 13)),
        format!("<?php {}", "$x=;".repeat(10_000)),
        format!("<?php {}", "()".repeat(20_000)),
    ] {
        let options = ParseOptions::default();
        let result = pprs::parse(&source, options).unwrap();
        checks::invariants(&result, options);
        if result.status() == ParseStatus::Aborted {
            assert!(matches!(
                result.diagnostics().last().unwrap().code,
                D::ResourceLimit(_)
            ));
        }
    }
    let over = vec![b' '; 16 * 1024 * 1024 + 1];
    assert!(pprs::parse(&over, ParseOptions::default()).is_err());
    let options = ParseOptions {
        limits: Limits::new(1_000_000, 128, 64, 250_000, 100).unwrap(),
        ..Default::default()
    };
    let result = pprs::parse(format!("<?php {}", "$x=1;".repeat(1000)), options).unwrap();
    assert_eq!(
        result.diagnostics().last().unwrap().code,
        D::ResourceLimit(Resource::Tokens)
    );
}

#[test]
fn deterministic_grammar_is_clean_on_every_supported_target() {
    for seed in 0u8..=255 {
        let data = [seed, seed.wrapping_mul(17), seed / 2, 255 - seed].repeat(8);
        let source = generated::program(&data);
        for version in pprs::PhpVersion::ALL {
            let options = ParseOptions {
                version,
                ..Default::default()
            };
            let result = pprs::parse(&source, options).unwrap();
            assert_eq!(
                result.status(),
                ParseStatus::Clean,
                "{source}: {:?}",
                result.diagnostics()
            );
            checks::invariants(&result, options);
        }
    }
}
