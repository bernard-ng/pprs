# PHP parser library: technical implementation plan

Date: 2026-10-16  
Status: P00–P02 complete; P03 onward pending; parser implementation has not started

Crate: `pprs`  
First stable PHP scope: **7.0–8.4**, confirmed by the user

## 1. Purpose and constraints

Build a Rust library that other Rust programs can use to turn PHP source into a useful, typed abstract syntax tree. Consumers should be able to inspect declarations and expressions, locate their original source, retain comments, and obtain useful partial results from malformed files. Success means documented compatibility, reliable source positions, predictable behavior on arbitrary input, and a small usable API.

The difficult work is PHP's context-sensitive lexing, version-dependent syntax interpretation, expression binding, and recovery. Generic scanning, serialization, snapshots, property testing, fuzzing, and benchmarking should reuse established Rust tooling. PHP-Parser's grammar, semantic actions, node definitions, and tests are the primary implementation reference; its PHP object hierarchy and generated tables do not dictate the Rust API.

### Required for the first stable release

- A library target usable through Cargo, without a shipped executable or PHP runtime dependency.
- Parsing for PHP 7.0–7.4 and 8.0–8.4, including mixed HTML/PHP files.
- A typed AST, original source bytes, complete token stream, comments, and precise locations.
- Configurable target PHP version, recovering parsing, and a strict convenience API.
- Structured lexical, syntax, contextual, and resource-limit diagnostics.
- Read-only AST traversal and useful node inspection.
- A reproducible compatibility suite derived from the checked-in reference, plus Rust-native tests.
- Documentation, examples, robustness checks, benchmarks, and downstream library validation.

### Separate later work

PHP 5.x, PHP 8.5+, incremental parsing/CST editing, an IDE/LSP, PHPDoc parsing, execution, type inference, whole-program analysis, bindings to other languages, and `no_std` support are outside the first stable release. Name resolution, pretty printing, formatting-preserving edits, mutation visitors, AST builders, constant evaluation, and public AST serialization have explicit later milestones below. The internal test exporter will use serialization earlier.

Do not infer a CLI requirement from the reference's `bin/php-parse`. Development-only test/export scripts and benchmark/fuzz targets are allowed; they are not product binaries.

## 2. Reference baseline and compatibility contract

Use `.references` at commit **`8d2b672aff65a1a4dbf7936cec27d19f51989412`**, described locally as `v5.9.0-7-g8d2b672a`.

The checkout is newer than its README suggests: `PhpVersion.php` selects 8.5, and `CHANGELOG.md` records experimental 8.6 partial application. Inventory the entire checkout, but classify PHP 8.5/8.6-only behavior as deferred. Never equate “all upstream cases” with “all PHP 7.0–8.4 cases.”

The reference is nikic's parser, not the PHP runtime itself. Follow its AST semantics and parser-level checks where applicable. Consult PHP's official documentation and versioned `php-src` grammar/scanner only to resolve language ambiguities or independently validate behavior; document any intentional difference.

### Version interpretation

The first release follows the reference's **best-effort target-version interpretation**, within this project's 7.0–8.4 feature ceiling. Target version controls relevant token classification, precedence, and historical syntax behavior. It is **not a promise to reject every construct introduced after the selected minor version**. A complete strict version validator is a separate capability. Likewise, parse success does not imply successful PHP compilation, type checking, or execution.

Use an explicit supported `PhpVersion` value. P00 fixes PHP 8.4 as the default; document it as a fixed release default, never “whatever PHP is installed.” Reject unsupported configuration values. There is no host PHP version at library runtime.

Maintain a feature/version matrix for every grammar family. Each rule records its upstream production, lexical requirements, introduction/removal or interpretation changes, Rust owner, fixture IDs, and compatibility status. Do not scatter unexplained numeric version checks through parser functions.

### What must match

- Valid-program AST meaning: node distinctions, ordered children, names, flags, literal values, and declaration/expression relationships.
- Raw bytes, spelling information, source spans, comments, and version-sensitive interpretation.
- Parser-level invalid-construct detection and useful recovery: report the problem and preserve unaffected statements.
- Deterministic results independent of the Rust machine's locale or installed PHP.

Rust type names, PHP class inheritance, numeric PHP token IDs, dynamic attribute dictionaries, and exact English parser error wording are not compatibility requirements. Recovery trees may differ in the damaged region; differences need explicit, tested expectations. See the [testing strategy](reference-and-testing-strategy.md) for the comparison rules.

Preserve the upstream BSD-3-Clause notice alongside copied or translated material and imported fixtures. Choose this project's own package license before release; do not silently infer it from the reference.

## 3. Architecture and public contracts

Start with one library crate. Internal modules establish boundaries without introducing a workspace of small crates prematurely.

```text
Caller supplies bytes + ParseOptions
             |
             v
      SourceText / LineIndex
             |
             v
       Stateful Lexer ------------> lexical diagnostics
             |
             v
    complete tokens + TokenCursor
             |
             v
   Parser + recovery + local checks -> syntax/context diagnostics
             |
             v
     AST normalization / comments
             |
             v
  ParseResult { source, tokens, AST, diagnostics, status }
             |
             v
       read-only visitors / consumer
```

### Ownership and data representation

| Component | Responsibility and chosen starting point |
|---|---|
| `SourceText` | Own/share immutable bytes, initially `Arc<[u8]>`; build a line-start index once. Convenience parsing from `&str` or `&[u8]` copies into owned storage; an owned-source entry point avoids that copy. |
| `Span` | Validated half-open byte range `[start, end)` within one source. Begin with `usize` offsets and checked construction; a compact representation requires measurements and an explicit file-size policy. |
| `Token` / `TokenKind` | Project-owned kinds and byte spans, including whitespace, comments, tags, invalid bytes, and EOF. Text is sliced from source; no per-token string allocation. |
| `TokenCursor` | Skip trivia for grammar decisions while retaining original token indices; provide bounded lookahead and progress tracking. |
| `Lexer` | Maintain HTML, PHP, string, interpolation, heredoc/nowdoc, and terminal raw-tail states; emit complete token coverage and lexical diagnostics. |
| `PhpVersion` / syntax rules | Centralize target-version interpretation and token/precedence decisions. |
| Typed AST | Rust enums and structs with `Box`/`Vec` for recursive children. Preserve distinct statements, expressions, names, identifiers, types, arguments, parameters, members, and patterns. |
| Node metadata | Source span, original token range, comment references, and typed spelling flags where needed. Empty/error origins must be representable without invalid ranges. |
| Parser | Recognize grammar, construct nodes, perform local contextual checks, and recover within the current delimiter/context boundary. |
| `Diagnostic` | Stable code/category, severity, primary byte span, optional secondary spans, expected/found token information, and human-readable message. No printing or global callbacks. |
| `ParseResult` | Keep source, tokens, AST and ordered diagnostics together; distinguish complete, recovered, and aborted parsing. A valid empty program differs from no recoverable root. |
| Visitors | Read-only, idiomatic traversal with explicit child order and short-circuiting; no parent pointer cycles. |

Use owned byte strings for semantic identifiers and decoded string values where needed, with raw spelling available through spans. This keeps the AST usable without borrowing the caller's input and permits non-UTF-8 values. Do not force PHP names or string literals through `String` or lossy UTF-8 conversion.

Retain raw numeric spelling and numeric kind. Specify a **64-bit PHP integer model** initially, independent of Rust pointer width; test integer overflow into floating-point representation and special float values against the recorded 64-bit oracle. A 32-bit PHP interpretation mode is deferred. Standard numeric conversion may be reused after PHP-specific prefix, separator, range, and escape rules are handled.

Keep source-derived metadata valid for an unmodified parse tree. Future transformed/synthetic nodes need an explicit origin policy; original token ranges must not pretend to describe regenerated code. Avoid arenas, interning, custom allocators, self-references, and public dependency-specific types until there is measured need.

### Intended API shape

Names below are design targets, not implemented signatures:

- `parse(source, options) -> ParseResult`: parse a complete file and collect diagnostics, preserving a partial AST where possible.
- `parse_strict(source, options) -> Result<ParsedFile, ParseError>`: succeed only when parsing completed without error diagnostics. Failure retains diagnostic information and can expose the recovered result.
- `lex(source, options) -> LexResult`: inspect tokens and lexical diagnostics without building an AST.
- `SourceText` accessors for raw slices and line/column mapping.
- Typed AST accessors and a read-only `Visit`/`walk_*` interface.

Validate options at construction; malformed PHP is a normal diagnostic outcome. Configure nesting, token/node, input-size and diagnostic budgets; choose defaults from stress measurements before release. A limit hit returns an aborted/recovered result with a specific diagnostic. Do not claim recoverable handling of system-wide allocation failure.

Byte offsets are authoritative. P00 decision D03 specifies one-based lines and byte columns, recognizing LF, CRLF and lone CR without changing source bytes. Normalize inconsistent upstream line/column helpers at the adapter boundary while preserving their raw attributes. UTF-16/editor display columns, if later needed, belong in adapters. EOF is the empty span at source length. Upstream AST ends are inclusive, but `Token::getEndPos()` is already exclusive; their conversions must remain separate.

### Walkthroughs that the design must support

1. **Ordinary file:** source ownership is established once; lexer emits trivia and semantic tokens; parser uses a cursor over that stream; AST keeps source origins; caller walks function declarations without knowing lexer implementation types.
2. **Interpolated string:** the lexer pushes interpolation state and tracks nested delimiters; parser builds string-part and expression nodes; returning to the string resumes the correct state and preserves raw bytes.
3. **Malformed function followed by valid code:** a missing operand emits a diagnostic/error node; recovery stops at the nearest owned delimiter; parsing continues to the next statement; result status is recovered, and strict parsing reports failure.
4. **`__halt_compiler()`:** validate the required syntax and scope before entering terminal raw-tail handling; retain all remaining bytes without interpreting them as PHP. Exact representation is settled in the lexer/parser spike.

## 4. Reuse and dependency policy

Add dependencies when their phase needs them, with compatible versions selected and tested at that time. Do not add every candidate up front. Verify MSRV, features, licenses, and API behavior before adoption; do not use wildcard version requirements.

| Task | Proposed tool | Decision |
|---|---|---|
| Regular byte token scanning | [Logos](https://logos.maciej.codes/attributes/logos.html) | Prototype in P03. Use explicit PHP states/callbacks; keep trivia. It supports byte input, but must prove the hard cases. |
| Yacc-compatible parser generation | [`lrpar`](https://docs.rs/lrpar/latest/lrpar/) / grmtools | Compare with recursive descent + Pratt in P03; upstream Yacc structure makes this a credible reuse option. |
| Byte-string ergonomics | [`bstr`](https://docs.rs/bstr/latest/bstr/) | Adopt when semantic byte strings are introduced; keep source/token storage simple. |
| Searching byte buffers | [`memchr`](https://docs.rs/memchr/latest/memchr/) | Use for delimiter/newline scanning where useful; no custom SIMD search implementation. |
| Conventional error traits | [`thiserror`](https://docs.rs/thiserror/latest/thiserror/) | Use if it removes real boilerplate; keep syntax diagnostics structured. |
| Test exchange/schema | [Serde](https://serde.rs/feature-flags.html), `serde_json` | Development dependencies initially; explicit encodings for arbitrary bytes/non-finite floats. |
| Regression snapshots | [Insta](https://insta.rs/docs/) | Development dependency; review snapshots independently of oracle comparison. |
| Property tests | [Proptest](https://proptest-rs.github.io/proptest/intro.html) | Development dependency from the lexer phase. |
| Fuzzing | [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) | Separate development target; begin with arbitrary-byte lexing, then parsing. |
| Benchmarks | [Criterion](https://bheisler.github.io/criterion.rs/book/) | Add once representative lex/parse operations exist. |
| Rich diagnostic display | [`miette`](https://docs.rs/miette/latest/miette/) | Later optional adapter; no terminal renderer in the core. |

The provisional parsing choice is **recursive descent for statements/declarations and Pratt parsing for expressions**, with a stateful byte lexer using Logos where it fits. It offers direct control over PHP context and diagnostics. P03 may replace this with `lrpar` if the grammar translation and recovery are materially simpler. Either outcome must reuse generic machinery where helpful and maintain a production-to-implementation map.

[LALRPOP](https://lalrpop.github.io/lalrpop/lexer_tutorial/006_error_recovery_custom_lexer.html) and [Chumsky](https://docs.rs/chumsky/latest/chumsky/) are fallback candidates, not simultaneous dependencies. [Tree-sitter PHP](https://github.com/tree-sitter/tree-sitter-php) is worth checking for reuse, but adopting it would introduce a separate syntax-tree and recovery contract requiring a substantial compatibility adapter. Do not assume its existence proves nikic-compatible behavior.

No async runtime, CLI framework, network client, parser-generator framework written in-house, custom serializer, custom snapshot framework, or generic regex engine is required.

## 5. Planned repository layout

Create files incrementally as their responsibilities are implemented. Parser paths may change after P03.

```text
src/
  lib.rs                  public entry points and exports
  source.rs               source ownership, Span, LineIndex
  version.rs              supported targets and interpretation rules
  token.rs                public token kinds and token metadata
  diagnostic.rs           structured diagnostics and parse status
  lexer/                  scanner states, strings, heredoc, numbers
  ast/                    expr, stmt, decl, types, names, metadata
  parser/                 cursor, expressions, statements, declarations,
                          types, normalization, checks, recovery
  visit.rs                read-only traversal
tests/
  support/                corpus loader and normalized comparison
  fixtures/upstream/      checked-in, exported, attributed fixtures
  fixtures/regressions/   minimized pprs-specific cases
  compatibility.rs        upstream corpus tests
  lexer.rs / parser.rs    focused behavior tests
tools/reference/          development-only PHP fixture/oracle exporter
benches/                  reproducible lexer and parser benchmarks
fuzz/                     isolated fuzz targets/corpus policy
docs/                     API, support matrix, decisions, compatibility
plans/                    this implementation sequence
```

If a generated parser is chosen, add source grammars and reproducible generation/build configuration without requiring PHP for downstream Cargo builds. Do not edit generated tables manually.

## 6. Ordered implementation phases

Every phase adds its tests and compatibility entries alongside behavior. Recovery, source fidelity, and version handling are ongoing work; P11/P12 close gaps rather than introducing them for the first time.

### P00 — Freeze scope, provenance, and behavior decisions

Status: **complete, 2026-10-16**. Evidence: [reference lock and verification](../reference/README.md), [compatibility inventories](../docs/compatibility/README.md), [behavior decisions](../docs/decisions/0001-parser-behavior.md), and [attribution](../THIRD_PARTY_NOTICES.md). Inventory: 192 productions, 193 node source files, 530 fixture cases; parser scope is 277 core / 9 deferred / 4 mixed. Implementation remains pending.

Dependencies: none.

- Record the pinned reference revision, source URL, clean/dirty state, and upstream license.
- Create the feature/version ledger from `grammar/php.y`, node definitions, `ParserAbstract.php`, lexer emulators, and fixtures.
- Classify PHP 5.x and 8.5/8.6-only cases as deferred; split mixed cases only with original IDs and a documented derivation.
- Write short decisions for byte-oriented input, half-open spans, 64-bit PHP numeric semantics, best-effort version interpretation, and diagnostic/recovery compatibility.
- Inventory every upstream grammar family, not only examples in the README.

Deliverables: reference manifest, compatibility/support matrix, initial design decisions, third-party notice.

Acceptance: no unclassified fixture families; support limits are explicit; scope is PHP 7.0–8.4 and library-only.

### P01 — Establish the library crate and quality checks

Status: **complete, 2026-10-16**. Evidence: [library scaffold](../src/lib.rs), [package metadata and lint policy](../Cargo.toml), [independent consumer](../tests/consumer/Cargo.toml), [CI workflow](../.github/workflows/ci.yml), and [development instructions](../README.md). Formatting and Clippy pass on stable Rust 1.99.0; library test targets, the import doctest, consumer compilation and documentation builds pass on stable and Rust 1.85.0. A clean copied consumer also builds offline without `.references`, and Cargo metadata exposes only a library target. The six P00 verifier tests and metadata verification pass. Hosted CI results await a push to GitHub. Original-code licensing remains a release decision; publication is disabled until then.

Dependencies: P00.

- Replace the current Hello World binary with `src/lib.rs`; remove the binary target from the product.
- Keep edition 2024 and Rust 1.85.0 as the validated baseline MSRV. Validate each dependency against it when introduced.
- Add package description, documentation metadata, and the agreed license once chosen.
- Configure formatting, Rust/Clippy warnings, and an unsafe-code policy; start with no project-authored unsafe code.
- Set up CI for stable Rust and the declared MSRV, unit/integration/doc tests, and feature combinations when features exist.
- Add a tiny downstream consumer test that imports the library. Keep CI independent of `.references` availability.

Deliverables: library scaffold, Cargo metadata, CI skeleton, initial README.

Acceptance: Cargo builds a library without a product binary; test/doc/format/lint commands pass; no PHP is required.

### P02 — Build the reference exporter and compatibility harness

Status: **complete, 2026-10-16**. Evidence: [exporter and oracle setup](../tools/reference/README.md), [corpus schema and comparison contract](../tests/fixtures/README.md), [manifest](../tests/fixtures/upstream/manifest.json), [coverage ledger](../tests/fixtures/coverage.json), and [Rust harness](../tests/compatibility.rs). All 290 upstream parser dumps reproduce under their original modes on the pinned 64-bit PHP 8.5.6 profile. Exported observations include four supported derivations and 19 project probes (all ten target versions), for 313 records / 300 comparable cases; parser implementation coverage remains zero. The remaining 240 printer cases stay inventoried and deferred. Offline integrity/adapter tests run in ordinary CI; oracle regeneration is an explicit development check.

Dependencies: P00–P01.

- Follow the [reference and testing strategy](reference-and-testing-strategy.md), including upstream macro expansion and normalization order.
- Export all 290 parser cases with stable IDs and provenance. Inventory the remaining 240 printer-related cases for later work; do not make printer execution a parser milestone.
- Preserve original expected dumps and build normalized AST/token/diagnostic records, with explicit reference target and host-runtime metadata.
- Establish fixture states and per-phase coverage reports; pending features are visible, previously covered behavior cannot silently regress.
- Add independent tests for importer fidelity and oracle normalization before using either to judge parser behavior.

Deliverables: development exporter, checked-in corpus, manifest, comparison adapters, coverage report.

Acceptance: fixture counts and source bytes reproduce the pinned corpus; explicit-version modes and macros survive; `cargo test` can read the exported corpus without PHP or a reference checkout.

### P03 — Resolve lexer/parser strategy with a bounded prototype

Dependencies: P01–P02. This is the only planned implementation-strategy fork.

- Prototype Logos regular scanning with explicit PHP states; compare a simple byte cursor where Logos complicates state boundaries.
- Compare recursive descent + Pratt with `lrpar` on the same representative grammar subset.
- Include mixed HTML/PHP, nested interpolation, non-UTF-8 input, heredoc labels/indentation, contextual names, `&`, PHP 7/8 precedence, call/member chains, a modern type, and malformed statements followed by valid code.
- Test how each approach preserves trivia, spans, partial nodes, and parser/lexer coordination for `__halt_compiler()`.
- Record grammar translation effort, dependency/build cost, implementation clarity, recovery quality, and basic throughput/memory observations. Do not conduct a broad parser-framework benchmark project.
- Keep one approach and write the decision, including how upstream productions remain traceable. Discard prototype code that is not part of the chosen implementation.

Deliverables: parser/lexer strategy decision and a small reusable hard-case test set.

Acceptance: a single chosen approach can express the hard cases, produce byte-accurate spans, and terminate on malformed input. No public API exposes framework internals.

### P04 — Implement source, token, diagnostic, and AST foundations

Dependencies: P03.

- Implement `SourceText`, checked spans/token ranges, line lookup, and byte-safe debugging.
- Establish token kinds and a cursor retaining all trivia; define EOF and invalid-token handling.
- Implement version options, parser budgets, diagnostic codes, result status, and strict/recovering contracts.
- Define typed AST families and common metadata, starting with nodes needed by the first vertical slice; map remaining node kinds in the ledger rather than filling the API with placeholders.
- Define names versus identifiers, string byte values versus spelling, literal numeric semantics, optional children versus empty lists, and explicit error nodes.

Deliverables: foundational modules and a testable public API skeleton.

Acceptance: spans remain valid for empty files, non-UTF-8, CRLF/LF, EOF and empty nodes; owned results outlive caller buffers; configuration has no hidden PHP-host dependence.

### P05 — Implement regular lexing and file boundaries

Dependencies: P04.

- Handle HTML/PHP transitions, open tags, short echo tags, closing-tag statement termination, and newline consumption.
- Implement the P00 short-open-tag decision: explicit option, default disabled; test hashbangs, BOM bytes, and text before the first tag against the reference profile.
- Implement whitespace, comments/doc comments, names, variables, keywords, contextual identifiers, punctuation, operators, and numeric tokens.
- Preserve original spelling and case; classify keywords using PHP's byte rules without Unicode identifier assumptions.
- Implement malformed-character/comment/number diagnostics and stop conditions; account for reference `&` disambiguation without forcing its internal token split on the public API.
- Add token differential tests, token reconstruction properties, and a lexer fuzz target.

Deliverables: regular scanner and file-mode behavior.

Acceptance: assigned lexer/unit cases pass; tokens cover source bytes exactly once, aside from the empty EOF sentinel; malformed inputs make progress and preserve byte offsets.

### P06 — Complete string lexing and literal interpretation

Dependencies: P05; parser support for embedded expressions is completed in P07.

- Add single/double-quoted and binary-prefixed strings, backticks, interpolation, nested interpolation delimiters, heredoc, and nowdoc.
- Distinguish raw text segments from embedded expression tokens. Keep nesting state explicit and bounded.
- Implement escapes, Unicode escape validation, flexible heredoc indentation, dynamic labels, and newline rules.
- Implement numeric conversion/overflow according to the selected PHP numeric model; cover separators, bases, malformed forms, infinity, and raw spellings.
- Implement raw-tail preservation for valid `__halt_compiler()` together with the parser boundary; invalid uses must not silently consume the rest of a file.
- Extend arbitrary-byte and truncation fuzzing to every lexical state.

Deliverables: complete PHP lexical modes and literal utilities.

Acceptance: scalar and lexer-state cases assigned to this phase pass at token/value level; unterminated constructs report bounded diagnostics without losing source bytes. Full scalar AST parity is completed in P07.

### P07 — Deliver the first complete parsing slice and expression grammar

Dependencies: P04–P06.

- Parse empty files, HTML, no-ops, expression statements, echo/return, and blocks to exercise the full public API early.
- Implement literals, names/constants, magic-constant nodes, variables/variable variables, grouping, arrays, array items, and destructuring forms. Preserve magic constants as syntax rather than evaluating them using the Rust process's file/environment.
- Implement unary/binary/assignment operators, references, exponentiation, ternary/coalesce, and short-circuit expressions with version-specific precedence/associativity.
- Implement calls, arguments, unpacking, named arguments, member/static/nullsafe access, array access, dynamic members, `new`, `clone`, and first-class callable syntax within scope.
- Add language constructs: include/require, eval, print, isset/empty, exit/die, casts, throw, match, yield/yield-from; callable bodies and generator contexts are completed in P08.
- Connect interpolated strings to expression parsing. Cover arrays versus patterns, writable contexts, and contextual restrictions through explicit rules rather than broad backtracking.
- Add expression error holes, delimiter-aware recovery, and precedence snapshots/differential checks.
- Add read-only traversal as AST variants arrive; extend it with P08/P09 nodes so it can receive the P12 deep-tree tests.

Deliverables: usable parse-to-AST vertical slice and core expression grammar; callable and anonymous-class expressions are completed in P08/P09.

Acceptance: assigned scalar/expression cases pass; the same ambiguous expressions have the expected PHP 7/8 trees; malformed expressions retain subsequent statements; strict parsing never treats a recovered tree as a clean success.

### P08 — Implement statements, functions, and control flow

Dependencies: P07.

- Add if/elseif/else, switch, loops, alternate colon/end syntax, break/continue, goto/labels, global/static declarations, unset, declare, try/catch/finally, and throw statements/expressions as appropriate.
- Add functions, closures and capture lists, arrow functions, parameters/defaults, references, variadics, return types, and generators.
- Implement shared named, built-in, nullable, union, intersection and DNF type parsing for callable signatures now; P09 reuses it in declarations and completes context-specific checks.
- Track grammar contexts such as loop/function/class/namespace boundaries only where parser-level behavior needs them; do not turn the parser into a compiler.
- Test dangling-else behavior, statement nesting, optional catch variables, multi-catch, trailing commas, and PHP closing tags in statements.
- Add recovery checkpoints for statement lists, callable bodies, parameter lists, and catch clauses.

Deliverables: procedural PHP parsing and callable ASTs.

Acceptance: assigned statement/function/generator/loop cases pass, including malformed cases; deep nesting stops at configured limits; no valid statement after a damaged one is discarded without a documented recovery reason.

### P09 — Implement namespaces, declarations, class members, and types

Dependencies: P08.

- Parse braced/unbraced namespaces, imports, aliases, function/constant imports, and grouped use statements.
- Parse classes, anonymous classes, interfaces, traits and adaptations, methods, properties, class/global constants, and modifiers.
- Implement attributes, constructor promotion, enums, readonly properties/classes, typed constants, PHP 8.4 property hooks, and asymmetric visibility.
- Reuse the P08 type parser for properties/constants and complete type-context constraints, preserving syntactic distinctions and target-sensitive built-in interpretation.
- Complete context checks for conflicting modifiers, invalid parameters/types, namespace placement/style, traits, enum forms and property hooks where the reference checks them.
- Cover semi-reserved keywords in names and stubs, empty bodies, multi-property declarations, and member-level recovery.

Deliverables: full in-scope declaration/type grammar.

Acceptance: every in-scope declaration/type production has an implementation owner and positive/negative coverage; class/property-hook errors do not prevent recovery of later members.

### P10 — Close version behavior and AST/source fidelity gaps

Dependencies: P07–P09.

- Audit every supported minor version, with targeted transition tests for 7.0/7.1, 7.3, 7.4, and 8.0–8.4 features and interpretation changes.
- Verify PHP 7 versus PHP 8 concatenation precedence, ternary handling, historical syntax acceptance/removal, keyword changes, and type-name interpretation against the pinned behavior.
- Finish AST normalization: semicolon namespace grouping, destructuring conversion, empty elements, comments/doc comments, trailing-comment no-ops, and source ranges.
- Preserve syntax distinctions needed by future printers: array/string kind, numeric raw spelling, quote/doc-label information, namespace form and cast/exit spelling.
- Check locations on every node, including damaged/empty nodes; keep byte spans, token ranges, and comment ownership independently testable.
- Audit PHP 8.5/8.6 fixtures for accidental inclusion. A best-effort older target does not lift the project's 8.4 feature ceiling.

Deliverables: completed support matrix and documented compatibility deviations.

Acceptance: all in-scope valid parser fixtures match normalized AST/value/comment/span expectations, or have an individually reviewed intentional difference. There are no unidentified version-dependent branches.

### P11 — Complete diagnostics, recovery, and parser-local validation

Dependencies: P07–P10; recovery tests have been added throughout.

- Review all invalid-input cases, not only the `errorHandling` directory.
- Standardize diagnostic categories, ordering, primary/secondary spans, unexpected/expected tokens, and suppression of cascading duplicates.
- Finish synchronization for delimiters, statements, arguments, parameters, class members, namespaces, interpolation, and EOF.
- Preserve useful valid structure outside damaged regions; represent skipped/error regions explicitly where the public contract needs them.
- Confirm each recovery loop consumes input, returns to its caller at an owned delimiter, or stops at a budget.
- Test strict versus recovering results, no-AST failures, diagnostic limits, parser reuse if offered, and deterministic results across repeated calls.

Deliverables: recovery behavior specification and closed invalid-input coverage.

Acceptance: all in-scope invalid fixtures have reviewed diagnostic and recovery expectations; no panic, infinite loop, invalid range, or false clean success occurs in the regression suite.

### P12 — Harden robustness and establish performance baselines

Dependencies: P10–P11; fuzz targets/measurements begin earlier.

- Run lexer/parser fuzzing with arbitrary bytes, truncations, mutation of imported cases, and bounded grammar-generated programs; minimize every failure into a regression.
- Stress long tokens/comments, huge argument/member lists, deep nesting, long operator chains, malformed delimiters, heredoc indentation, and error storms.
- Audit recursive parsing, AST traversal, debug/serialization and drop paths: a parser depth limit alone does not prevent a deeply nested AST built from a long left-associative chain.
- Measure lex-only and full-parse throughput, allocation/peak-memory behavior, result size, compile time, and dependency footprint on pinned corpora.
- Optimize only measured bottlenecks; consider arenas/interning/small vectors only if evidence justifies their complexity.
- Choose resource defaults and reproducible benchmark baselines from observations. Avoid invented throughput promises or noisy cross-language speed claims.

Deliverables: fuzz regressions, resource-limit defaults, benchmark dataset/commands, baseline report.

Acceptance: the agreed bounded fuzz/stress runs are clean; limits produce diagnostics before uncontrolled recursion; representative performance/memory results are recorded and repeatable. No unexplained compatibility regression accompanies optimization.

### P13 — Finish the consumer-facing library API and documentation

Dependencies: P10–P12; API examples begin at P07.

- Finalize exports, ownership/accessors, version configuration, lex/parse/strict entry points, diagnostic inspection and source slicing.
- Finalize and document the read-only visitors introduced with AST variants in P07–P09; verify early exit, well-defined ordering, and coverage of every variant. Rerun affected P12 depth checks if this changes traversal behavior.
- Add examples for parsing a file's bytes supplied by the caller, collecting function declarations, inspecting comments/spans, and handling partial ASTs.
- Document byte encoding, numeric policy, target-version limits, recovery guarantees, thread sharing, and resource limits. No hidden I/O or global mutable parser state.
- Decide enum extensibility and semver policy before stabilizing the API; compiler exhaustiveness checks must help maintain internal traversal as nodes evolve.
- Build an independent downstream Rust consumer. Verify package contents do not depend on ignored `.references`, generated local files, or PHP.

Deliverables: documented library API, examples/doc tests, downstream integration validation.

Acceptance: consumers can use the crate without internal modules or development tooling; examples compile; published documentation accurately describes the compatibility contract.

### P14 — Release readiness and ongoing reference updates

Dependencies: P00–P13.

- Close all in-scope coverage ledger entries; no pending or unclassified case remains for supported behavior.
- Run format/lint/tests/doc tests on MSRV and stable, required feature combinations, and representative supported OS targets.
- Run the reference differential job with explicitly recorded PHP runtime/version settings; verify generated fixtures are reproducible.
- Verify package metadata, project and third-party licenses, dependency requirements, changelog, release notes, and package contents with `cargo package`.
- Review public API changes and remaining intentional deviations. Deliver a release-ready package; publication is a later explicit action.
- Define an upstream-update procedure: pin new revision, inspect grammar/node/lexer/test diffs, reclassify new cases, regenerate snapshots, implement or defer features, rerun compatibility gates.

Deliverables: release checklist, packaged crate, compatibility/performance reports, upgrade procedure.

Acceptance: a clean downstream environment builds and tests the package without PHP or `.references`; first stable claims are supported by the ledger and verification evidence.

## 7. Milestones and work that can run in parallel

| Milestone | Completion gate | Consumer value |
|---|---|---|
| Foundation | P00–P04 | Reproducible specification, chosen architecture, library contracts |
| First usable parser | P05–P07 | Parse a documented expression/file subset with diagnostics |
| Grammar complete | P08–P10 | PHP 7.0–8.4 syntax families and metadata accounted for |
| Compatibility and robustness | P11–P12 | Reviewed malformed-input behavior and bounded resource use |
| Stable library candidate | P13–P14 | Documented, independently consumable, packaged library |

Within the dependency order, corpus export and library CI can proceed together after P00. After P04 contracts settle, AST definitions and regular lexer work can proceed separately. Statement/declaration families can be divided after expression and cursor interfaces stabilize. Diagnostics audits, version-matrix maintenance, fuzzing, and documentation run alongside feature implementation. Avoid simultaneous incompatible changes to AST metadata, token semantics, or the core parser cursor.

These are completion gates, not calendar estimates. Estimate duration after P03 establishes grammar translation cost and P07 establishes a fixture completion rate.

## 8. Later milestones for broader PHP-Parser functionality

Implement these only when prioritized; they do not block the parser-library release.

| Milestone | Technical work | Acceptance |
|---|---|---|
| E01: serialization and diagnostic adapters | Optional Serde support with a versioned representation, byte/non-finite-number policy; optional display adapters that preserve original byte offsets | Round trips preserve documented data; core builds without optional dependencies |
| E02: name resolution | Namespace context, imports, aliases, class/function/constant lookup rules; retain original names and distinguish lexical qualification from runtime fallback | Adapt upstream `NameContextTest`/`NameResolverTest`; no claim of whole-program symbol resolution |
| E03: AST transformation and construction | Mutable/folding visitors, replacement/removal rules, synthetic origins, ergonomic constructors/builders | Valid transformations preserve unaffected nodes and make metadata invalidation explicit |
| E04: standard pretty printing | Version-aware precedence/parentheses, string/number rendering, comments, indentation, PHP/HTML transitions | Adapt 95 printer cases; parse-print-parse structural equivalence with explicit target versions |
| E05: formatting-preserving edits | Original-node provenance, token mapping, unchanged-region copying, edit conflict/fallback rules | Adapt 145 mutation cases as Rust operations; unchanged input reproduces byte-for-byte |
| E06: constant expressions | Explicit supported expression subset and PHP value semantics, with typed unsupported/error outcomes | Adapt evaluator tests; evaluating expressions never executes arbitrary PHP or user calls |
| E07: newer PHP syntax | Re-audit a pinned reference for 8.5 pipe/void/clone/constant attributes and later features; keep experimental syntax separately identified | New version matrix and dedicated fixtures pass before extending advertised support |
| E08: other consumers | Decide separately on incremental CST, PHPDoc, editor/LSP, bindings or legacy PHP | Each requires its own scope/design/acceptance plan; retain the core library boundary |

## 9. Open decisions and principal risks

| Item | Planned resolution |
|---|---|
| Parser/lexer framework choice | P03 hard-case comparison, then one documented selection |
| Project license | Maintainer choice before package release; upstream notices retained from P00 |
| Exact MSRV/dependency versions | P01 validated Rust 1.85.0 with no dependencies; P02 test-only JSON/hash/hex dependencies also pass MSRV checks; validate future dependency versions as introduced |
| Short-open-tag profile | P00 fixes default disabled; implement explicit option and matching oracle settings in P05 |
| Default resource limits | Measurements and stress testing in P12, with safe interim bounds from P04 |
| Lexer complexity hidden by reference runtime | Treat standalone lexing as a first-class phase and test against token oracles |
| Reference/oracle drift or permissive version behavior | Pinned revisions, explicit targets, recorded host settings, separate strict-validation claims |
| Corrupted binary/float fixture values | Byte-safe structured export, tagged floats, importer tests |
| Recovery hides bugs or discards valid suffixes | Explicit status and error nodes, valid-prefix/suffix comparisons, progress/budget assertions |
| Manual grammar translation loses productions | Production ledger and differential coverage, reviewed on every upstream update |
| Public AST becomes difficult to evolve | Small initial API, internal framework types, semver/extensibility decision before stable release |

Scope and version decisions already confirmed by the user do not need reconfirmation. Unresolved implementation details should be settled by the planned evidence and recorded decisions.
