# 0010 — Diagnostics, recovery and parser-local validation

Accepted: 2026-10-17. Completes the diagnostic and recovery checks identified in the expression, statement, declaration and source fidelity decisions. PHP targets remain 7.0–8.4; the pinned reference and original observations are unchanged.

## Compatibility contract

Malformed PHP is an ordinary parse result. Strict parsing preserves the recovering result on failure. A partial or recovered tree is never a clean success. Input-size rejection happens before copying/indexing the input and returns `InputTooLarge`, without constructing a parse result. Resource or nested-halt aborts retain available structure and an explicit unscanned suffix. The result model also supports an absent tree; its assembly/strict-conversion tests cover that distinction. The current parser retains a root, possibly empty, after construction starts. There is no parser reuse API or shared parser state.

Exact English wording and LR recovery choices within damaged syntax were excluded from the public compatibility promise in decision 0001. This review documents each difference from LR recovery. All 112 invalid core fixtures have an individual [review and native diagnostic expectation](../../tests/fixtures/recovery/core-review.json). Missing-name placeholders, partial trees after unmatched delimiters, Unicode decoding failures, malformed lists and namespace grouping remain explicit differences. Skipped bytes remain visible through lossless tokens; missing expressions use `ExprKind::Error` where the grammar can preserve a useful enclosing construct. No generic skipped-region AST wrapper is introduced.

## Diagnostic rules

- Stable codes determine lexical, syntax, context or resource categories. Literal decoding remains lexical even where the reference calls it contextual. Unsupported future syntax has its own syntax code. Messages supply local detail; the library never prints them.
- Primary and related locations are checked half-open byte spans. EOF is an empty primary range with an explicit `found: Some(Eof)` when a token was expected. Tokens and source allow clients to recover exact bytes, including non-UTF-8 input.
- Diagnostics are in deterministic discovery/emission order. Lazy bounded lookahead can discover lexical errors before a preceding production reduces. Delayed array-hole checks run after assignment-pattern conversion; whole-file namespace checks run during grouping. We do not sort by position or imitate the reference's separate initial lexer pass.
- `expected` contains useful grammar token expectations at explicit expectations, terminators, parameter variables, callable bodies and halt boundaries. An empty list means the parser has no useful finite expectation to offer. `found` is populated for single-token syntax locations; whole-construct and context diagnostics may leave it absent.
- Missing closing delimiters identify the matching unmatched opening delimiter when available. Repeated/conflicting modifiers identify the earlier conflicting modifier. These related locations supplement the primary error; they do not replace it.
- Multiple grammar productions unwinding at the same primary range produce one syntax diagnostic, preserving the first local expectation. Context errors are independent: a static readonly constructor produces two errors, and static/abstract/readonly class constants produce three. A forbidden hook modifier and a conflict at the same token both remain visible.
- The diagnostic budget reserves a terminal limit entry. Exhaustion aborts immediately, never exceeds the configured count, and does not scan the suffix to collect more errors. Every resource diagnostic is terminal.

## Recovery corrections

The parser cursor skips lexically rejected bytes while retaining them and their lexical diagnostics. This removes fabricated syntax cascades between otherwise valid statements, restores three upstream source/AST cases, and restores all ten DEL-byte probes in the source fidelity suite to exact reference structure and metadata.

Halt validation now checks before consuming each token. A malformed halt diagnoses the actual unexpected token and leaves it for recovery; raw tail mode is authorized only after a complete outermost halt statement. Statement synchronization recognizes declaration and control-flow starts. A damaged expression yields at an unambiguous statement start instead of consuming it; named function declarations are distinguished from closure expressions. Following echo statements, functions, classes, namespaces and class methods have direct survival regressions.

The [recovery and validation ledger](../implementation/recovery.md) records ownership and progress rules for every recovery boundary. Scope restrictions remain parser-local: this does not add compiler checks for symbol resolution, duplicate parameters, undefined labels, loop scope or runtime type consistency. Clean control probes lock down those exclusions.

## Evidence and comparison policy

All 300 eligible core cases now exercise all four channels: tokens 296 exact / 4 differences; structure/status 266 / 34; metadata 265 / 35; diagnostics/status 239 / 61. All 188 clean reference cases match AST values, source metadata and diagnostics exactly. No core channel is pending. Thirteen out-of-scope originals remain deferred under the original scope policy.

The test-only diagnostic adapter maps native unexpected-token/context codes and the three known lexer codes to the established oracle schema. Other native codes and categories remain visible. It preserves diagnostic order, wording, ranges and coordinates; comparison ignores only the pre-existing `raw_attributes` provenance field. Each differing channel has an exact alternative observation and explanation. Native snapshots independently check stable codes, severity, expected/found tokens and related spans. Mutation tests reject omitted, reordered or changed diagnostics. No production component reads fixtures or JSON.

The [87 focused probes](../../tests/fixtures/recovery/README.md) add all-target parser-local checks, compiler-only controls, malformed boundaries and compound validation failures: 57 exact reference diagnostic observations, 28 explicit differences and two recorded reference assertion failures. The frozen reference asserts in `inlineHtmlHasLeadingNewline()` for malformed halt probes lacking a closing parenthesis or terminator before `echo`. The exporter requires assertions enabled, admits only those two named failures with the exact assertion text, and never presents them as successful oracle observations. Rust survival and non-clean results are checked independently.

Every core invalid case is parsed repeatedly and through the strict entry point. Focused probes run every byte prefix and tiny depth, token, node and diagnostic budgets. Shared checks verify all diagnostic/related spans, unique expected kinds, found-token consistency, cascade suppression, terminal resource errors, source retention and AST metadata invariants. Error storms explicitly check bounded output and the unscanned suffix. The later [robustness report](../benchmarks/robustness-performance.md) records instrumented fuzz campaigns and measured performance baselines.

Validation: 82 Rust tests and five doctests pass on stable and Rust 1.85.0; the two explicit development audit exporters are ignored in ordinary runs. Formatting, Clippy, rustdoc, downstream-consumer and fuzz-target compilation, 13 Python tooling tests, reference integrity, coverage non-demotion and every oracle reproduction check pass locally. No hosted CI run or instrumented fuzz campaign is claimed.
