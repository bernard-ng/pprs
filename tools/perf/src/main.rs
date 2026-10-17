//! Development-only measurements; never part of the product crate.
use pprs::{ParseOptions, ParseStatus};
use serde_json::{Value, json};
#[cfg(not(feature = "allocations"))]
use std::{hint::black_box, time::Instant};

#[cfg(feature = "allocations")]
#[global_allocator]
static ALLOCATOR: &stats_alloc::StatsAlloc<std::alloc::System> = &stats_alloc::INSTRUMENTED_SYSTEM;

fn inputs(dataset: &str) -> Vec<(Vec<u8>, ParseOptions)> {
    if dataset == "valid" || dataset == "invalid" {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        return std::fs::read_to_string(root.join("tests/fixtures/upstream/cases.jsonl"))
            .unwrap()
            .lines()
            .filter_map(|line| {
                let row: Value = serde_json::from_str(line).unwrap();
                if row["scope"] != "core"
                    || (row["oracle"]["status"] == "clean") != (dataset == "valid")
                {
                    return None;
                }
                Some((
                    hex::decode(row["source"]["bytes"].as_str().unwrap()).unwrap(),
                    ParseOptions {
                        version: row["oracle"]["target"].as_str().unwrap().parse().unwrap(),
                        ..Default::default()
                    },
                ))
            })
            .collect();
    }
    let source = match dataset {
        "statements" => format!("<?php {}", "$value = foo(1, 2) + 3;\n".repeat(10_000)),
        "members" => format!(
            "<?php class C {{ {} }}",
            (0..10_000)
                .map(|i| format!("public $p{i} = 1;\n"))
                .collect::<String>()
        ),
        "arguments" => format!("<?php f({});", vec!["1"; 20_000].join(",")),
        "comment" => format!("<?php /*{}*/ echo 1;", "x".repeat(1024 * 1024)),
        "heredoc" => format!("<?php $x = <<<DOC\n{}DOC;", "    text\n".repeat(50_000)),
        "chain" => format!("<?php {};", vec!["$x"; 10_000].join(" + ")),
        "node-limit" => format!("<?php {}", "$x=1;".repeat(100_000)),
        "token-limit" => format!("<?php {}", "$x;".repeat(500_000)),
        "errors" => format!("<?php {}", "$a = ;\n".repeat(10_000)),
        "newlines" => format!("<?php {}echo 1;", "\n".repeat(16 * 1024 * 1024 - 13)),
        "whitespace" => format!("<?php {}echo 1;", " ".repeat(16 * 1024 * 1024 - 13)),
        _ => panic!("unknown dataset: {dataset}"),
    };
    vec![(source.into_bytes(), ParseOptions::default())]
}

#[cfg(not(feature = "allocations"))]
fn run(inputs: &[(Vec<u8>, ParseOptions)], mode: &str) {
    for (source, options) in inputs {
        if mode == "lex" {
            black_box(pprs::lex(black_box(source), *options).unwrap());
        } else {
            black_box(pprs::parse(black_box(source), *options).unwrap());
        }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let dataset = args.get(1).map_or("valid", String::as_str);
    let mode = args.get(2).map_or("parse", String::as_str);
    assert!(matches!(mode, "parse" | "lex"));
    let inputs = inputs(dataset);
    let bytes: usize = inputs.iter().map(|(s, _)| s.len()).sum();
    let mut metadata = Vec::new();
    for (source, options) in &inputs {
        #[cfg(feature = "allocations")]
        let region = stats_alloc::Region::new(ALLOCATOR);
        let parsed = (mode == "parse").then(|| pprs::parse(source, *options).unwrap());
        let lexical = (mode == "lex").then(|| pprs::lex(source, *options).unwrap());
        #[cfg(feature = "allocations")]
        let stats = region.change();
        let (tokens, diagnostics, status, nodes, depth) = if let Some(parsed) = &parsed {
            (
                parsed.tokens().tokens().len(),
                parsed.diagnostics().len(),
                match parsed.status() {
                    ParseStatus::Clean => "clean",
                    ParseStatus::Recovered => "recovered",
                    ParseStatus::Aborted => "aborted",
                },
                parsed.program().map_or(0, |p| pprs::visit::walk(p).count()),
                parsed.program().map_or(0, |p| {
                    pprs::visit::walk(p).map(|n| n.depth).max().unwrap_or(0)
                }),
            )
        } else {
            let lexical = lexical.as_ref().unwrap();
            (
                lexical.tokens().tokens().len(),
                lexical.diagnostics().len(),
                if !lexical.is_complete() {
                    "aborted"
                } else if lexical.diagnostics().is_empty() {
                    "clean"
                } else {
                    "recovered"
                },
                0,
                0,
            )
        };
        let mut row = json!({"bytes":source.len(),"tokens":tokens,"nodes":nodes,"depth":depth,"diagnostics":diagnostics,"status":status});
        #[cfg(feature = "allocations")]
        {
            row["allocations"] = json!(stats.allocations);
            row["reallocations"] = json!(stats.reallocations);
            row["allocated_bytes"] = json!(stats.bytes_allocated);
            row["retained_heap_bytes"] = json!(stats.bytes_allocated - stats.bytes_deallocated);
        }
        let stream = parsed
            .as_ref()
            .map_or_else(|| lexical.as_ref().unwrap().tokens(), |p| p.tokens());
        row["scanned_bytes"] = json!(
            stream
                .tokens()
                .iter()
                .find(|t| t.kind() == pprs::token::TokenKind::Unscanned)
                .map_or(source.len(), |t| t.span().start())
        );
        row["inline_result_bytes"] = json!(if mode == "parse" {
            std::mem::size_of::<pprs::ParseResult>()
        } else {
            std::mem::size_of::<pprs::LexResult>()
        });
        row["source_bytes"] = json!(source.len());
        metadata.push(row);
    }
    // Allocation builds report single-operation allocation/result measurements only.
    // Throughput uses the normal allocator and includes source indexing, parsing and drop.
    #[cfg(feature = "allocations")]
    let (iterations, samples) = (0, Vec::<u128>::new());
    #[cfg(not(feature = "allocations"))]
    let (iterations, samples) = {
        run(&inputs, mode);
        let start = Instant::now();
        run(&inputs, mode);
        let iterations = (50_000_000 / start.elapsed().as_nanos().max(1)).clamp(1, 10000) as usize;
        let samples = (0..9)
            .map(|_| {
                let start = Instant::now();
                for _ in 0..iterations {
                    run(&inputs, mode);
                }
                start.elapsed().as_nanos()
            })
            .collect::<Vec<_>>();
        (iterations, samples)
    };
    println!(
        "{}",
        json!({"dataset":dataset,"mode":mode,"input_bytes":bytes,"inputs":inputs.len(),"iterations_per_sample":iterations,"sample_ns":samples,"results":metadata})
    );
}
