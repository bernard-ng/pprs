# P03 — Lexer and parser strategy

Accepted 2026-10-16. Choose **Logos for regular byte tokens, explicit byte-oriented lexical states, recursive descent for statements/declarations/types, and Pratt parsing for expressions**. Keep all framework types private. P04 defines the production contracts; P05 onward implements them. The retained P03 implementation is test-only.

## Evidence and experiment boundary

The shared input set has [17 hard cases](../../tools/strategy/inputs.json), exported against the frozen nikic commit using the P02 normalizer. It includes mixed HTML/PHP, invalid UTF-8 names/strings, nested quoted interpolation, flexible heredoc/nowdoc endings and indentation, contextual member names, intersection/reference ampersands, PHP7/PHP8 precedence, call/member/index chains, a DNF parameter type, recovery, and valid/invalid/nested halt directives.

Two regular-scanning backends (a byte cursor and Logos 0.16.1) ran behind the same PHP state controller and parser. They produced identical tokens, events, lexical diagnostics and prototype trees for all 17 inputs. Logos required both `#[logos(utf8 = false)]` and explicitly byte-oriented `(?-u:...)` identifier regexes: byte input alone did not make the default Unicode regex classes accept arbitrary high bytes. The retained implementation enables `forbid_unsafe`.

A separate grmtools 0.14.0 experiment used `YaccGrammar`, Pager table construction and `lrpar::RTParserBuilder::parse_map`, with the same scanner and grammar subset. Its clean semantic tree projections matched recursive descent/Pratt and the pinned oracle projections. Both approaches can express this subset; LR was not rejected for failing ordinary PHP syntax.

The comparison LR grammar had 13 nonterminals in the PHP7 variant and 14 in PHP8: program/list/statement, assignment, version-dependent concatenation/addition, multiplication, postfix operations, argument lists, atoms, and union/intersection/parenthesized types. The PHP8 variant adds the separate addition level. A single typed by-reference function parameter exercises type/expression context. This is a small experimental translation, not a translation of the 192 upstream nonterminals.

The retained selected probe additionally passes all **495 byte prefixes** of the inputs, all 256 possible repeated byte values in a statement envelope, comments between a reference ampersand and variable, and depth-limit checks. These are deterministic progress checks, not fuzzing or full-language coverage.

## Why this combination

| Concern | Observation | Decision |
|---|---|---|
| Regular tokens | Logos and cursor agree, including arbitrary high bytes. Logos supplies maintained regex/DFA machinery. | Reuse Logos; do not maintain a second regular scanner. |
| Strings, interpolation and heredoc | Both backends still require nested-state and delimiter/indentation handling. | Keep state transitions and boundary scanning in project-owned byte code. Regex rules handle only regular spans. |
| Contextual names and `&` | Member names must survive keyword spelling; intersections and reference markers depend on grammatical position and significant lookahead. | Preserve raw spelling; classify in context. Skip comments as well as whitespace during lookahead. |
| PHP7/PHP8 precedence | LR needs two generated grammar/table variants or an equivalent preprocessing branch. Pratt selects binding powers at parse time. | Centralize target rules and use Pratt for expressions. |
| Byte source with LR | `NonStreamingLexer` exposes `&str` span access and character-oriented coordinates. The experiment used a one-byte ASCII surrogate for that interface and retained real byte spans separately. | Avoid maintaining this additional source/diagnostic adapter. It proves byte support is possible, not impossible in LR. |
| Halt coordination | The runtime LR entry point collects lexer output before reductions. Unrestricted scanning of the raw tail failed; clean halt parsing required a lexical prepass validating the empty call, statement boundary and outer scope. | Let the parser commit the terminal raw-tail state before requesting another token. Avoid duplicating this grammar boundary in a lexer prepass. |
| Recovery | CPCT+ returned repaired lists ending in an expression shape on the tested malformed cases; RD separately asserted the surviving suffix bytes. LR repaired-value/span fidelity was not established by this projection. The LR subset accepted nested halt without a contextual error and still needs a separate scope-checking action. | Own synchronization and partial-tree construction alongside declaration contexts and local checks. CPCT+ is a viable alternative, but not the selected recovery contract. |
| Dependencies/MSRV | Logos 0.16.1 passes Rust 1.85. grmtools 0.14.0 also builds/runs there. The attempted 0.15.0 resolution failed on dependencies requiring Rust 1.88/1.89. | Keep 1.85. Only Logos is retained, currently as a test dependency. Do not raise MSRV for an unselected candidate. |

The LR probe's parser-error counts exclude shared scanner errors. For example, bad heredoc indentation is detected by the scanner even though the LR grammar accepts the resulting atom. This is not evidence that LR cannot diagnose indentation. The recorded nested-halt acceptance likewise identifies a required contextual action, not a fundamental LR limitation.

## Measured costs and limits

[Raw observations](../experiments/p03-results.json) record toolchain, architecture, package versions, candidate source/lock hashes, three process runs and per-case LR/RD results. The combined candidate dependency graph resolved to 69 packages. Its initial release dependency build with an empty main took 15.14 seconds on the local machine with downloaded caches; this is not a controlled clean-network build comparison. The retained Logos addition brought eight new lockfile packages to the existing test graph.

A 57-byte input was repeated 20,000 times per stage, release mode, on aarch64 macOS with Rust 1.85.0. Median elapsed times across three runs:

| Stage | Prototype | Median elapsed |
|---|---|---:|
| Lex only | Byte cursor + state controller | 5,626 µs |
| Lex only | Logos + same state controller | 3,252 µs |
| Lex + parse | Cursor + RD/Pratt | 12,193 µs |
| Lex + parse | Cursor + LR adapter/generic tree | 157,007 µs |

These timings are observations of these prototypes, not intrinsic framework speed ratios or release throughput targets. LR includes token-vector, surrogate and generic-tree `String` allocation; RD uses static tree tags. Table construction (about 1.9 ms for both variants) is excluded from parsing timings. A generated parser and typed actions would change LR cost; the selected Logos+Pratt combination was not separately benchmarked in this comparison. The choice primarily follows state coordination and compatibility requirements.

The combined experiment process used 5.88–5.95 MB maximum resident memory. This is a process-level observation including both candidates, not per-parser allocation evidence. On this host the prototype token is 24 bytes and a tree header 56 bytes; vector capacities and children are additional. Tokens borrow source via byte ranges. Allocation profiling, large-source memory, generated-table size and production benchmarking remain P12 work.

The comparison cursor and LR implementation were discarded after recording results. Only the selected backend/controller/parser probe and reusable cases are retained. The discarded comparison is not an ongoing supported benchmark; retained correctness checks are reproducible through Cargo.

## Required production interfaces

1. `SourceText` owns immutable bytes; all scanners and grammar decisions use byte spans. No UTF-8 conversion is required for PHP values.
2. A project-owned token buffer requests tokens lazily from the stateful lexer, preserves trivia and original indices, and supplies significant lookahead. It yields a complete retained stream when parsing finishes, including uninterpreted halt tails and EOF. P04 resolved abort finalization as an explicit unscanned suffix with no further lexing; see [decision 0003](0003-foundation-contracts.md). P11 audits integration with recovery.
3. The lexer owns HTML/PHP, quoted string, interpolation and heredoc/nowdoc states. P06 will expose string parts and embedded PHP tokens; P03's coarse template atoms are not the final token model.
4. The parser can commit terminal tail handling only after recognizing a complete `__halt_compiler();` at an allowed scope and statement boundary. No tail token is requested first. Invalid syntax or scope must leave subsequent input available.
5. Statements/declarations own their synchronization delimiters and contextual checks. Expression parsing takes explicit target-dependent binding powers. Type parsing is a separate production context, not a public boolean mode.
6. Public ASTs, diagnostics and token kinds are project-owned types. Logos enums and prototype `Tree` tags never cross the library boundary. The prototype's span-bearing dynamic tree is replaced by P04's typed contracts.

## Traceability and staged implementation

`tools/strategy/inputs.json` lists frozen upstream nonterminals for every case; tests reject unknown production names. Keep the existing P00 grammar ledger immutable and add a live production-to-implementation mapping as production code arrives.

| Frozen reference family | Prototype responsibility | Production phase |
|---|---|---|
| `top_statement`, `statement` | `Parser::program`, scope/synchronization, halt boundary | P08/P11; halt lexing in P06 |
| `expr` and top-of-file precedence declarations | `Parser::expression`, PHP7/PHP8 binding powers | P07 |
| `callable_variable`, `member_name`, `argument_list` | Postfix loop and argument delimiters | P07 |
| `type`, `parameter`, `ampersand` | Type context and reference lookahead | P09; general declaration contracts P04 |
| `scalar`, `encaps_list`, `encaps_var` | `Scanner::quoted`, interpolation and heredoc boundaries | P06/P07 |

## Known limits and oracle finding

P03 proves architecture on a bounded subset. It does not implement PHP tokens/ASTs generally, full keyword/version handling, complete string escapes, all heredoc cases, declaration rules or production diagnostics. Templates and heredocs are coarse atoms; nested interpolation is checked as boundary events. The oracle projection deliberately maps those literal values, names and integers to atoms, drops inline-HTML AST statements, and compares semantic operator/postfix/type shape. Full literal/value/comment/token parity remains P05–P11 work. The 300 P02 compatibility cases remain pending; none were promoted by prototype success.

Under PHP 8.5.6 with assertions enabled, the pinned reference throws `AssertionError: assert($prevToken->id == \T_CLOSE_TAG)` while parsing `<?php __halt_compiler(1); 7;` (`ParserAbstract.php::inlineHtmlHasLeadingNewline`, line 998). A direct native-tokenizer probe classifies the remaining `; 7;` as `T_INLINE_HTML`, so the tokenization/halt boundary has already changed how subsequent input is classified. The fixture records this exact oracle failure rather than turning it into a clean/recovered oracle observation. No assertion was disabled and no reference code was modified. This is not a full upstream issue analysis.

P00's behavior decision still applies: the selected probe diagnoses the malformed directive and preserves the following `7;`. It also diagnoses a nested halt directive while retaining the following statement. These are explicit project recovery requirements, not claims of matching an oracle tree that was never returned.

## Reproduce retained checks

```sh
cargo test --locked --test strategy
cargo +1.85.0 test --locked --test strategy
php -n -d short_open_tag=0 -d zend.assertions=1 tools/strategy/export.php --check
```

Only the final command requires the pinned PHP environment and `.references`. Use `--write` only for an intentional reviewed oracle-input update. Ordinary CI discovers the test through `cargo test --all-targets`; the library and downstream consumers do not depend on Logos yet.

Primary API references used in the experiment: [Logos byte-source configuration](https://logos.maciej.codes/attributes/logos.html), [Logos 0.16.1](https://docs.rs/logos/0.16.1/logos/), [lrpar lexer interface](https://docs.rs/lrpar/0.14.0/lrpar/trait.NonStreamingLexer.html), [runtime parser builder](https://docs.rs/lrpar/0.14.0/lrpar/struct.RTParserBuilder.html), and [grmtools guide](https://softdevteam.github.io/grmtools/master/book/quickstart.html).
