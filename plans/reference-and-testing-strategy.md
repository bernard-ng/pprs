# Reference and testing strategy

Date: 2026-10-16  
Scope: PHP 7.0–8.4 parser library  
Companion: [technical implementation plan](technical-implementation-plan.md)

P00 is complete: the [reference lock](../reference/README.md), [scope inventories](../docs/compatibility/README.md) and [behavior decisions](../docs/decisions/0001-parser-behavior.md) now exist and verify. Parser cases are classified as 277 core, nine deferred and four mixed. P02 is also complete: the [exporter](../tools/reference/README.md) reproduces all 290 original parser dumps and produces the [313-case corpus](../tests/fixtures/README.md), including four derived and 19 project cases. The offline harness and regression gates exist; all 300 comparable cases remain pending because no Rust parser is implemented. The strategy below remains the contract for later implementation.

## 1. Reference identity and relevant source

- Repository: [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser).
- Local checkout: `.references/`.
- Pinned commit: `8d2b672aff65a1a4dbf7936cec27d19f51989412`.
- Local description: `v5.9.0-7-g8d2b672a`.
- License: BSD-3-Clause; retain `.references/LICENSE` with reused material.
- Inspection found a clean reference checkout. The parent project ignores `.references`, so ordinary test execution must not depend on it.

| Reference path | How to use it |
|---|---|
| `grammar/php.y` | Primary grammar, precedence, version branches, error productions, and node-construction actions |
| `grammar/README.md`, `grammar/rebuildParsers.php`, `grammar/phpyLang.php` | Understand preprocessing and generated PHP 7/PHP 8 parsers before considering a grammar port |
| `lib/PhpParser/ParserAbstract.php` | Recovery, numeric/doc-string interpretation, namespace/destructuring normalization, locations and contextual checks |
| `lib/PhpParser/Lexer.php`, `lib/PhpParser/Lexer/Emulative.php`, `lib/PhpParser/Lexer/TokenEmulator/` | Token normalization, target-version behavior and emulation; ordinary scanning itself comes from PHP's tokenizer |
| `lib/PhpParser/Node/` | Node meaning, required/optional fields, order, spelling metadata and special cases |
| `lib/PhpParser/ParserFactory.php`, `lib/PhpParser/PhpVersion.php` | Best-effort version semantics and supported-version assumptions |
| `test/PhpParser/CodeTestParser.php`, `test/PhpParser/CodeTestAbstract.php`, `test/PhpParser/CodeParsingTest.php` | Exact fixture format, expansion, modes and assertions |
| `test/bootstrap.php` | Fixture canonicalization rules |
| `test/code/parser/` | Main source/AST/error corpus |
| `test/PhpParser/ParserTestAbstract.php`, `test/PhpParser/LexerTest.php`, `test/PhpParser/Lexer/EmulativeTest.php`, `test/PhpParser/TokenTest.php` | Additional source-position, literal, lexical and token behavior beyond `.test` files |
| `doc/component/Lexer.markdown`, `doc/component/Error_handling.markdown` | Position conventions and partial-AST/error contracts |
| `test/code/prettyPrinter/`, `test/code/formatPreservation/` | Later printer/rewriter corpus; useful additional parser input after version classification |
| `test_old/run.php`, `test_old/run-php-src.sh` | Ideas for wider corpus validation, with the limitations below |

These are repository-relative reference paths for implementation navigation. Record exact upstream paths and hashes in generated fixture manifests so the ignored local checkout is not the only source of provenance.

## 2. Actual corpus inventory

Counts below describe the whole pinned checkout, including features outside the first-release scope. Case counts come from grouping the `-----` sections according to the upstream fixture parser; they are not PHPUnit's expanded test count.

| Corpus | Files | Cases | Initial use |
|---|---:|---:|---|
| Parser `.test` | 180 | 290 | Exported and classified in P02 |
| Pretty printer `.test` | 72 | 83 | Inventory for E04 |
| Pretty printer `.file-test` | 4 | 12 | Inventory for E04 |
| Format preservation `.test` | 32 | 145 | Inventory for E05 |
| **Total** | **288** | **530** | Entire fixture inventory |

There are also **58 `*Test.php` files** with tests/data providers for tokens, lexing, parsing, AST helpers, traversal, resolution, printing, builders, and evaluation. Extract relevant behavioral cases deliberately; porting every PHP unit-test method is not the parser milestone.

### Parser families

| Directory under `test/code/parser` | Files | Cases |
|---|---:|---:|
| Root/general | 7 | 10 |
| `errorHandling` | 3 | 33 |
| `scalar` | 14 | 24 |
| `expr` directly | 43 | 52 |
| `expr/fetchAndCall` | 10 | 10 |
| `expr/uvs` | 8 | 8 |
| `stmt` directly | 21 | 23 |
| `stmt/class` | 29 | 69 |
| `stmt/function` | 23 | 31 |
| `stmt/generator` | 3 | 3 |
| `stmt/loop` | 4 | 4 |
| `stmt/namespace` | 15 | 23 |
| **Total** | **180** | **290** |

Expected outcomes across the parser corpus:

- 179 cases: clean AST.
- 99 cases: diagnostics plus partial AST.
- 12 cases: diagnostics without an AST.

The 111 error cases span multiple syntax directories. Testing only `errorHandling` would miss most error expectations.

There are 15 cases requesting position dumps, one requesting an attribute dump, and 21 explicitly versioned cases spanning PHP 5.6–8.2. The upstream harness nevertheless requires position metadata on every returned node. Version modes are not an exhaustive language-support matrix.

## 3. Scope classification before export

Create a machine-readable manifest with a stable case ID such as `parser/scalar/int.test#0`; use repository-relative paths and explicit section indices. Give every case a language-feature classification and phase owner.

The reference README says PHP 7.0–8.4, `PhpVersion::getNewestSupported()` returns 8.5, and the changelog includes experimental 8.6 partial function application. Consequently:

1. Explicit PHP 5.x behavior is deferred unless the same case is deliberately derived for an in-scope target.
2. PHP 8.5 pipe, void cast, clone-call changes, constant attributes, and other newer-only behavior are deferred. PHP 8.6 argument placeholders are separately deferred.
3. Fixtures mixing in-scope syntax with newer syntax may be split into derived cases. Preserve the untouched original and document exactly which source/expectation subset was derived; never edit upstream expectations silently.
4. Keep the original upstream mode, including “newest.” For project comparisons, run the reference with an explicit in-scope target such as 8.4 and record that target separately. A changed target can change expected output; do not relabel the old dump as if it were regenerated.
5. Assign targeted tests to all ten supported minor versions (7.0–7.4, 8.0–8.4), even when original fixture modes do not mention them.

Classify features independently of the mode: `expr/partialFunctionApplication.test#1` requests PHP 7.4 but expects newer argument-placeholder nodes. An old explicit target is not sufficient evidence that a fixture belongs to the 7.0–8.4 scope. Mixed cases include `stmt/const.test#0` (constant attributes), `stmt/class/property_promotion.test#0` (final promotion), and `stmt/function/clone_function.test#0` (clone calls/declarations). Ordinary clone expressions remain in scope.

Fixture scope and implementation status are separate fields. Coverage statuses are `pending` with a phase, `passing`, `intentional-difference` with an explanation, and `deferred` with an out-of-scope reason. A failure in a case already marked passing is always a regression; it cannot become a blanket ignore or expected failure without review.

P00 completed per-case classification: 277 original parser cases are core, nine are deferred, and four need derived cases. The 290-case figure remains the whole-reference inventory; the final acceptance denominator also depends on P02's derived and additional cases.

## 4. Reproducible exporter and oracle

Implement a development-only PHP exporter under `tools/reference/`. Reuse the pinned fixture parser instead of independently recreating its executable-macro semantics in Rust.

### Fixture expansion order

The upstream sequence matters:

1. Canonicalize fixture-file CRLF, trailing spaces/tabs, and final newlines as `test/bootstrap.php` does.
2. Evaluate trusted checked-in `@@{PHP expression}@@` macros.
3. Split `-----` sections and group by the correct arity.
4. Extract the expected section's leading `!!` mode line.

Parser and standard printer cases are pairs; format-preservation cases are triples: input, PHP AST mutation code, expected output. Do not apply text cleanup to the expanded source: macros intentionally create raw newlines, whitespace, NUL/control bytes, and invalid UTF-8. Fixture macro execution belongs only to this pinned development exporter, never to the Rust library or to arbitrary user-provided fixture files.

### Environment record

Record the reference commit and file hashes, exporter/schema version, host PHP version, `PHP_INT_SIZE`, tokenizer/JSON extension availability, short-open-tag setting, relevant locale/configuration and requested target version. Use a defined 64-bit oracle profile. A local PHP 8.5.6 executable was observed during planning; that fact does not constitute an executed upstream test run.

The reference requires PHP >=7.4 with tokenizer and JSON. Its development tooling includes PHPUnit 9 and php-yacc. `.references/vendor` is absent in the inspected checkout, so upstream PHPUnit is not immediately runnable. P02 established the [isolated oracle profile](../tools/reference/README.md) using PHP 8.5.6 with INI disabled, a verified source autoloader and no Composer/PHPUnit dependencies. It executes all parser fixture expectations; it does not claim a complete upstream PHPUnit run. Normal Cargo builds have no PHP/Composer requirement.

### Export records

Each exported case should contain:

- Schema version, stable ID, title, origin path/section, original file hash and reference commit.
- Original fixture modes, chosen reference target and runtime-profile ID.
- Expanded source as a byte-safe value; original expected dump retained separately.
- Scope/features and phase ownership; any derivation from the original fixture.
- Normalized reference tokens, AST, diagnostics and result status when generated.
- Separate comparison expectations for structure, positions/comments, and errors/recovery.

Use JSON with explicitly tagged byte arrays (or a documented byte encoding) and tagged non-finite floats. Never substitute invalid UTF-8, rely on native JSON `INF`/`NaN`, or round-trip large integers through a floating-point JSON consumer. Preserve raw numeric spelling and encode large integer values losslessly, for example as tagged decimal strings.

Check in generated cases and minimal provenance/license files under `tests/fixtures/upstream/`. Ordinary tests read snapshots only. Regeneration is an explicit development action with deterministic ordering and a reviewable diff; it is not a network or PHP operation in `build.rs`.

## 5. Comparison model

Use project-owned normalization adapters on both sides. Their schema is a test contract and should not force the public Rust AST to copy PHP inheritance or attribute dictionaries.

### Tokens

Compare stable token names, raw bytes, order and spans rather than PHP's numeric token IDs. Preserve whitespace, comments, PHP tags and inline HTML. Record explicit mapping for a different but equivalent token split; otherwise compare tokens exactly. A coarse normalization must not hide dropped bytes or changed lexical boundaries.

The Rust EOF token has an empty span at source length and contributes no source bytes. The PHP reference uses a sentinel with `"\0"` text; normalize it deliberately rather than appending that byte to reconstruction. Upstream `Token::getEndPos()` is already exclusive.

### AST structure and values

Compare node meaning, child order, distinct optional/empty values, modifiers, names/qualification, literal bytes/value/kind, array versus destructuring forms, and namespace grouping. Keep a mapping table for intentional Rust node-shape differences.

Compare spelling metadata separately from semantic structure so a quote-style or raw-number regression remains visible. Preserve meaningful distinctions such as `null` child versus empty list, unpack versus callable placeholder, and static/instance/dynamic access.

For every valid in-scope case, require structural/value parity. An intentional deviation needs a narrowly scoped explanation, a regression expectation, and an explicit entry in the support documentation. Passing a self-approved Rust snapshot alone is insufficient.

### Positions and comments

Upstream AST `endFilePos`/`endTokenPos` are inclusive. Convert them to half-open Rust ranges with checked arithmetic. Upstream permits inverted ranges for some zero-length no-op/error/parameter nodes; map these to an explicit empty origin. Missing metadata is not interchangeable with zero.

Normalize diagnostic ranges separately from AST and token ranges. The reference's unterminated-comment error uses the token's exclusive end position, while EOF errors locate a point at source length. Preserve raw oracle error attributes and apply explicit category-specific conversion; blindly adding one to every diagnostic `endFilePos` can produce an out-of-bounds span. Test unterminated comments, bad bytes, ordinary syntax errors, and EOF in the adapter suite.

Verify bounds for all nodes and diagnostic spans, including EOF and synthetic/error positions. Do not assume every semantic parent's span strictly contains every child/comment after upstream normalization; enumerate legitimate exceptions before imposing that property.

For comparable lines/columns, apply P00 D03's source-derived LF/CRLF/lone-CR rule to spans, retaining raw upstream attributes. Native tokenization and some reference line/column helpers differ on lone CR; this normalization must not hide differing byte spans.

Compare comment text, spans, type and attachment, including doc comments and comments at the end of files/classes. Token preservation and comment attachment are separate assertions.

### Diagnostics and recovery

Compare lexical/syntax/contextual category, relevant location, rejected construct, ordered result status and surviving valid structure. Map known upstream errors to project codes with explicit rules; do not use loose string matching to silently accept unrelated errors.

Exact wording and identical recovery choices inside a damaged region are not required. For intentional recovery differences, assert the expected Rust partial shape, diagnostics, and preserved unaffected prefix/suffix. Clean, recovered and aborted/no-root results are distinct. Unexpected acceptance without an error is a failure.

Keep error recovery and message rendering tests separate so improving wording does not regenerate the entire AST corpus.

### Protect the adapters themselves

Test fixture parsing and normalization against known upstream dumps for bytes, floats, empty nodes, comments and explicit modes. Add checks that deliberate mismatches in node kind, value, span or diagnostics fail comparison. Review exporter updates before accepting regenerated expectations.

## 6. Coverage gates by implementation phase

| Phase | Required evidence |
|---|---|
| P00–P02 | Inventory 530 cases; export/classify all 290 parser cases; validate IDs, modes, bytes, provenance and normalization; no silently dropped macros |
| P03–P04 | Hard-case prototype tests; source/span/result invariants; standalone downstream API skeleton |
| P05–P06 | Lexer/token unit behavior, reconstructed bytes, scalar interpretation and lexical-state transitions, malformed/truncated input, arbitrary-byte fuzzing |
| P07 | Assigned in-scope cases from the 94 scalar/expression-family cases; targeted precedence, references, dereferencing and interpolation tests |
| P08–P09 | Assigned in-scope cases from the 153 statement-family cases; procedural/object/type/namespace syntax plus contextual errors |
| P10 | All in-scope clean AST cases, all-node positions/comments, version transition tests and completed grammar ledger |
| P11 | All in-scope error/partial/no-AST cases, strict/recovering contracts and recovery progress |
| P12 | Stress/property/fuzz runs, regression corpus, resource limits, reproducible performance and memory measurements |
| P13–P14 | Visitor completeness, doc examples, downstream consumption, MSRV/stable checks, packaged crate and reproducible reference results |
| E04–E05 | Separately adapted 95 printer cases and 145 mutation cases if those milestones are selected |

The family counts above include out-of-scope cases until P00 classification. Use the manifest's supported subset for gates; report total, supported, passing, pending, intentionally different and deferred counts independently.

For every feature implementation, the workflow is: identify upstream productions/tests, implement behavior, compare structured output, add meaningful malformed/version-boundary cases, update the ledger, and run the affected suite plus prior compatibility regressions. Do not add tests that simply repeat implementation details.

## 7. Rust-native properties and fuzzing

Add these incrementally rather than waiting for grammar completion:

- Arbitrary source bytes never require UTF-8 or trigger unchecked indexing.
- Token spans are ordered/in-bounds and reconstruct all original source bytes, including raw compiler-halt tails.
- EOF and empty-source behavior are consistent; every parser/recovery loop progresses or terminates.
- Parsing is deterministic for identical bytes/options; parsing one file does not leak state into the next.
- Span slicing never panics; line/column conversions agree at boundaries and on CRLF/LF/control bytes.
- Truncating input at every token boundary always terminates with valid result invariants and no hang or stack overflow. Complete prefixes may parse cleanly; require diagnostics when the resulting prefix is syntactically incomplete.
- Bound recursive parsing, interpolation, type nesting, AST construction depth, traversal and teardown. Long flat operator chains can generate deep trees even without deep parser recursion.
- Generate bounded valid syntax and mutate delimiters/keywords/literals. Persist minimized failures with source/options and a regression expectation.
- Compare recovering and strict entry points: any error prevents strict success.

Use Proptest and cargo-fuzz rather than inventing generators/runners from scratch. Add small grammar-specific strategies where needed. Record fuzz engine/toolchain, seeds/corpus, elapsed budget, limits and results; “fuzzing passed” without a stated run is not release evidence.

## 8. Wider corpus and independent checks

After the core corpus is useful, add small pinned selections from real PHP projects and versioned `php-src` `.phpt` files. Record origin/license, source revision, extraction and exclusion reasons, parser target, and expected success/error classification. Keep full external corpora out of ordinary unit tests when their size is inappropriate.

Use PHP's parser/tokenizer as an additional oracle with an explicit matching runtime. `php -l` also performs compiler checks that nikic may intentionally omit, so distinguish syntax differences from compiler-level rejection. Never execute parsed application code as part of conformance testing.

The legacy reference runner is useful inspiration, but cannot be copied as the release gate:

- `test_old` contains two runners, not a bundled php-src/Symfony corpus.
- `run.php` defaults to target PHP 8.0 and filters out parse/fatal-error cases; supply explicit targets and retain independent invalid-input tests.
- Its format-preservation failure count is not part of its failure exit condition, and some exceptions are only printed. Implement explicit failing assertions in the new harness.
- `run-php-src.sh` downloads/replaces a working directory. Use a deliberate pinned corpus preparation step instead of running it incidentally.

Future printer testing should compare parse-print-parse AST equivalence, output under explicit parser/printer targets, and byte-identical unchanged formatting-preserving output. PHP mutation snippets from the upstream format-preservation fixtures must be translated into equivalent Rust AST operations, not executed against Rust nodes.

## 9. CI and release evidence

Normal CI uses Rust and checked-in fixtures. Separate jobs cover an explicitly provisioned reference oracle, wider external corpora, and longer fuzz/benchmark runs. No automated schedule is being created by this plan.

Planned checks include `cargo fmt --check`, Clippy with the project's chosen lint policy, unit/integration/doc tests, declared feature combinations, MSRV/stable builds, and `cargo package`. Use representative OS checks for byte/path portability; PHP numeric interpretation remains the documented 64-bit model on every Rust host.

Before stable release, archive a reproducible report containing the reference/exporter/schema identity, supported case totals and outcomes, reviewed deviations, version-matrix results, fuzz/stress evidence, benchmark inputs/results, dependency/MSRV settings, and downstream package verification. No known unexplained in-scope regression may be hidden by a fixture exclusion.
