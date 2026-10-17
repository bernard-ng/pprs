# Reference corpus and compatibility contract (schema 1)

P02 exports **313 cases**: 290 unchanged upstream parser cases, four supported derivations, and 19 project probes. The comparator has **300 eligible cases**, all still pending complete parser implementation (296 token channels pass and four have exact intentional differences). Nine out-of-scope originals and four mixed originals are deferred; the four mixed originals each have a supported derivation. The remaining **240 printer cases** are inventoried separately, not executed or counted as parser coverage.

The [manifest](upstream/manifest.json) records the frozen upstream commit, SHA-256 hashes, exporter inputs and exact PHP runtime profile. [coverage.json](coverage.json) is an independently maintained state ledger; [coverage-report.json](coverage-report.json) reports case totals and channel obligations by phase. A successful harness test is not a successful PHP parser test.

## Files and byte fidelity

- `upstream/raw/parser/**`: all 180 original fixture files, byte for byte. Git attributes disable newline conversion for these files.
- `upstream/cases.jsonl`: one deterministic, ID-sorted JSON record per case.
- `upstream/deferred-printer.jsonl`: the 240 unmodified P00 inventory rows.
- `upstream/LICENSE`: upstream BSD-3-Clause notice; retain it when distributing derived material.
- `coverage.json`: states for every case and each comparison channel. Regeneration never replaces it.

Original IDs are the P00 IDs, e.g. `parser/scalar/int.test#0`. Origins retain the reference commit, path, section index, full-file hash and original raw-section hashes. `mode` retains the exact mode line or null; `modes` is its parsed object. `original_target` resolves absent upstream versions to the pinned newest version, **8.5**. `original_expected` contains the macro-expanded original dump, which the exporter verifies before accepting a case.

`comparison_target` is an explicit supported target: an in-scope fixture's explicit version, otherwise **8.4**. Its `oracle` observation is freshly generated. Deferred/mixed originals have null comparison targets and retain observations under their original targets for inspection only. No original dump is relabeled as an 8.4 expectation.

Derived IDs live under `derived/`. Their `derivation` contains the parent ID, expanded parent-source hash and ordered half-open byte segments. Concatenating those slices gives the new source. The argument-list derivation retains target 7.4; the others use 8.4. Expected results are generated anew. Original mixed fixtures remain untouched.

Project inputs are defined in [inputs.json](../../tools/reference/inputs.json). Ten version probes cover PHP 7.0–7.4 and 8.0–8.4 with concatenation precedence and type interpretation. Nine adapter probes cover empty source, EOF errors, unterminated comments, bad bytes, mixed line endings, halt tails, numbers, short tags and BOM bytes. These probes are a starting set, not a complete language-version suite.

## Encodings and observation schema

Every source/semantic string/dump uses `{"bytes":"lowercase hexadecimal"}`. Never decode PHP source or values lossily as UTF-8. All semantic integers, including modifiers, use `{"integer":"decimal"}`; offsets and counts remain JSON integers. All semantic floats use `{"float64":"16 lowercase hex digits"}` containing IEEE-754 binary64 bits in big-endian order. This retains infinities, NaNs and negative zero without invalid JSON or decimal rounding.

Each `oracle` contains `target`, `status`, `tokens`, `ast`, `metadata`, `diagnostics` and a byte-tagged `dump`:

- `status`: `clean`, `recovered`, or `aborted`. Null AST and empty AST are distinct.
- `tokens`: stable `T_*` names, `CHAR_xx` punctuation kinds, or `EOF`; byte text; half-open byte span; source-derived start/end coordinates. Whitespace, comments, tags and halt tails remain. EOF contributes no bytes. Numeric host token IDs are excluded.
- `ast`: nodes with `kind` and named `fields`; fields contain child nodes, ordered arrays, tagged values, booleans or null. Field names are schema strings. No node metadata is folded into structure.
- `metadata`: one entry per node, keyed by its path into `ast` (e.g. `/0/fields/expr`). Each has an `origin` with half-open byte/token spans and an explicit `empty` flag, ordered attached `comments`, `spelling` attributes, and raw upstream attributes. Comment entries preserve kind, byte text, origin and raw attributes. Raw numeric attributes use the semantic integer tags on nodes; diagnostics/comment raw positions remain JSON integers.
- `diagnostics`: ordered category, adapter code, byte-tagged raw message, optional span/coordinates, and raw attributes. Lexer diagnostics are identified through a separate lexer run and an exact match to the leading collected parser diagnostics. Known lexical codes are `lex.unterminated-comment`, `lex.null-byte`, `lex.bad-character`; unknown lexical errors fail export. The anchored upstream `Syntax error,` family maps to `reference.syntax`; remaining parser-local errors map to `reference.contextual`. These are oracle adapter codes, not a proposed public diagnostic enum.

Node/comment inclusive ends become exclusive; inverted empty-node ranges map to `[start,start)`. Missing node origins fail export. Token ends are already exclusive. Unterminated-comment diagnostic ends stay exclusive, EOF diagnostics stay empty, and other diagnostic ends become exclusive. Every range must remain in bounds.

Coordinates use one-based lines and byte columns with LF, CRLF and lone CR handling. The offset between CR and LF remains on the preceding line. Raw upstream line/position attributes remain for auditing. Parent containment is not assumed after upstream AST normalization.

## Comparison channels and implementation states

The test-only adapter in [tests/support/mod.rs](../support/mod.rs) compares four channels:

| Channel | Compared fields | Default phase obligation |
|---|---|---|
| `tokens` | Full token stream, bytes, kinds, spans and coordinates | P05 (with lexical-state completion in P06) |
| `structure` | Result status and complete semantic AST | Case owner, no earlier than P07 |
| `positions` | Node origins, spelling, comment attachment/text/origins | P10 |
| `diagnostics` | Result status and ordered diagnostics | P11 |

Only `raw_attributes` are removed from comparison. Dump text is retained for audit and is not a structured comparison input. All other normalized distinctions remain exact. Diagnostic wording is currently compared exactly as a conservative harness default; later public-code mappings or recovery differences require explicit adapter rules and regression tests, not a global message filter.

States are `pending`, `passing`, `intentional-difference`, and `deferred`. Every channel has a phase owner. Core cases cannot be deferred. Differences need a reason and an `expectation` file under `tests/fixtures/differences/` containing the exact alternative observation; the expected channel must actually differ from the reference. Passing and difference cases require output from a real Rust adapter. An absent adapter or mismatch fails the test.

P05/P06 connect the production lexer through `tests/support/lexer.rs` in `tests/compatibility.rs`: 296 full token channels pass, including bytes/kinds/spans/coordinates; four halt-tail cases have exact intentional differences because standalone lexing does not infer parser scope. Production-cursor tests separately verify parser-authorized raw tails. P07–P10 connect the production AST/source adapter in `tests/support/parser.rs` and `metadata.rs`: 249 structure/status channels and 262 metadata channels pass; 17 structure and 38 metadata recovery differences have exact snapshots. All 188 valid core cases match both channels exactly. The full parser-diagnostic adapter remains absent; promoting diagnostics still fails. Pending cases are enumerated, not hidden via `#[ignore]`. The [dedicated lexical oracle](lexer/README.md) checks 109 focused cases; the [literal oracle](literal/README.md) checks 139 values/errors. The [P07 expression oracle](parser/README.md) adds 133 focused structure/status observations; the [P08 statement oracle](statements/README.md) adds 49; the [P09 declaration oracle](declarations/README.md) adds 60. The [P10 fidelity oracle](fidelity/README.md) adds 222 target/source/ceiling observations. Complete files requiring later-phase syntax have explicit live ownership corrections documented in [decisions 0006](../../docs/decisions/0006-expression-parser.md) and [0007](../../docs/decisions/0007-procedural-parser.md); frozen observations are unchanged. PPRS's additional unfinished-string diagnostic and omission of native zero-byte transition tokens are explicitly documented in [decision 0005](../../docs/decisions/0005-strings-literals-and-halt.md).

The CI history check rejects demotion/deletion of previously passing/difference channels relative to the pull request base. An intentional policy change to that rule requires a reviewed code change. Regeneration cannot silently reset states. The per-phase report counts channel obligations separately from case totals.

## Offline verification

```sh
cargo test --locked --test compatibility
PYTHONDONTWRITEBYTECODE=1 python3 tools/reference/corpus.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/reference -p 'test_*.py' -v
```

Cargo tests need only the committed snapshots and Rust dependencies; no PHP, Python, `.references`, network corpus or build script is involved. The separate Python CI job verifies raw sections, the P00 inventories, exact generated-file set, supplemental definitions, exporter hashes and the coverage report. Update the report after changing states with `python3 tools/reference/corpus.py --write-report`.

See the [exporter guide](../../tools/reference/README.md) for explicit regeneration and oracle checks.
