# 0011 — Robustness and performance baselines

Accepted: 2026-10-17. Records the robustness and performance results. The PHP 7.0–8.4 scope, frozen reference, runtime dependencies and compatibility exceptions are unchanged.

## Bounded robustness contract

The default budgets remain 16 MiB input, depth 128, 1,000,000 tokens, 250,000 AST nodes and 100 diagnostics. The effective depth ceiling remains 128 even when callers request more. The robustness review retains these values after exercising long tokens, strings and comments, heredocs, wide lists and class bodies, malformed boundaries, error storms, long operator chains and budget exhaustion. They are configurable work and structure bounds, not an RSS quota. Source line indexes, decoded strings, AST storage and allocator retention can substantially amplify input size; memory-constrained callers should lower limits. The [baseline](../benchmarks/robustness-performance.md) records both retained result heap and process memory observations, including the newline-heavy boundary case.

The [deterministic stress suite](../../tests/robustness.rs) runs in ordinary CI. A 2 MiB worker stack exercises parsing, iterative traversal, derived cloning/equality/Debug, the independent test JSON projection, JSON serialization and teardown. Shapes include grouping, unary nesting, left-associative and assignment chains, arrays and statements at sizes 16 through 4,096, including limit crossings. Long chains must hit a diagnostic rather than produce an unbounded recursive AST. These observations apply to parser-produced trees with the effective depth ceiling; public manually constructed trees and arbitrary consumer callbacks are outside that guarantee. There is no production serialization API or parser reuse API to audit.

The campaign runner seeds byte fuzzing from all 300 core fixtures, sampled truncations and byte deletion/replacement, plus every byte value. A separate bounded grammar generator requires clean parsing and checks source/AST/diagnostic invariants; ordinary tests exercise 256 generated inputs on all ten targets. Grammar fuzzing complements the frozen PHP differential corpus and does not replace it. Instrumented campaigns are explicitly bounded development commands, separate from CI's deterministic tests and target compilation.

## Minimized stack regression

The stress suite found a debug-build stack overflow in the test-only recursive JSON oracle projection. On this host, `<?php ` followed by 110 `!` operators and `$x;` reproduced it on a 2 MiB stack; 109 passed. Removing the projection allowed production parsing, cloning, formatting and drop to complete at all tested depths. An intermediate probe localized failure to AST JSON construction before metadata extraction. The production parser was not the failing recursion.

`full_observation` now checks depth with the iterative walker and projects trees deeper than 64 in a scoped worker with an 8 MiB stack. The 110-operator regression and neighboring boundaries remain in the ordinary lifecycle test. Projection output is serialized and dropped back on the original 2 MiB worker. This change adds no runtime dependency or stack-growth behavior to the library. Temporary reproduction logging was removed.

## Measured optimization

The initial allocation baseline found 160,030 allocation calls when parsing a 20,000-argument call. Internal AST walks used for parser accounting allocated a fresh child vector at each node. `Walk` now drains and reuses one child buffer, preserving its explicit stack, source-order preorder and depth values. The same case uses 80,025 allocation calls after the change; retained result heap is unchanged. The valid fixture batch and wide member case also improve. Full before/after measurements and limitations are in the baseline. No arena, interning, unsafe code, custom product allocator or new runtime dependency is introduced.

## Acceptance evidence

Three sequential ASan/coverage campaigns completed with no crashes, timeouts, invariant failures or sanitizer findings: lexer 796,287 executions, byte parser 101,419, generated parser 210,231. Each ran for 121 reported seconds with fixed seeds and fresh initial corpora. Nightly installation failed DNS resolution; the recorded run used the explicit `RUSTC_BOOTSTRAP=1` fallback with installed Rust 1.99. ASan linkage and inline coverage counters were verified. This is not a claim of nightly execution. Commands, source hashes and compressed full logs are retained in the [campaign evidence](../benchmarks/p12-fuzz.json).

Release baselines cover lexing and full parse/drop for 13 datasets, including all 188 valid and 112 invalid core fixtures. They separately report normal-allocator timing, allocation counts/bytes, inline and retained result size, single-batch and repeated-process peak RSS, cold/warm build time and dependency footprint. They are descriptive measurements on one host, not timing gates or performance promises.

Validation: 86 Rust tests and five doctests pass on stable and Rust 1.85.0; two explicit audit exporters remain ignored. Formatting, Clippy, rustdoc, consumer checks, fuzz compilation, both measurement allocator configurations, 13 Python tooling tests, reference integrity and coverage non-demotion pass locally. Every existing compatibility expectation remains enforced; no exception was added or demoted. Hosted CI execution and wider/longer fuzz campaigns remain future evidence, not claims of this checkpoint.
