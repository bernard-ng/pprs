# Implementation plans

Status: **P00–P10 complete** (2026-10-17). The public expression/procedural/callable/declaration parser, production lexing, literal utilities and read-only traversal are available.

The first stable release will be a reusable Rust library for PHP **7.0–8.4**, as confirmed on 2026-10-16. There is no product CLI.

1. [Technical implementation plan](technical-implementation-plan.md): scope, architecture, crate choices, ordered implementation steps, and release gates.
2. [Reference and testing strategy](reference-and-testing-strategy.md): pinned PHP-Parser reference, fixture inventory, oracle/export design, compatibility rules, and verification.

P00 delivered the [frozen reference](../reference/README.md), [compatibility matrix and inventories](../docs/compatibility/README.md), [behavior decisions](../docs/decisions/0001-parser-behavior.md), and [third-party notice](../THIRD_PARTY_NOTICES.md). The reference and inventory verifier passes, including its negative regression tests.

P01 delivered the [library scaffold](../src/lib.rs), [Cargo metadata and lint policy](../Cargo.toml), [independent consumer](../tests/consumer/Cargo.toml), [CI workflow](../.github/workflows/ci.yml), and [development guide](../README.md). Local checks pass on stable Rust and Rust 1.85.0; the clean consumer builds offline without the reference checkout. Hosted CI execution awaits a push to GitHub.

P02 delivered the [reference exporter](../tools/reference/README.md), [313-case corpus and comparison contract](../tests/fixtures/README.md), and [coverage report](../tests/fixtures/coverage-report.json). All 290 original dumps reproduce; the four mixed cases have supported derivations. All 300 eligible parser cases remain pending.

P03 selected Logos regular scanning with explicit byte states and recursive descent/Pratt parsing. The [strategy decision](../docs/decisions/0002-parser-strategy.md) records the comparison and the [hard cases](../tests/fixtures/strategy/README.md) remain executable tests. At P03, production parser APIs remained pending.

P04 delivered [source/token/AST/options/diagnostic/result foundations](../src/lib.rs), the private lazy cursor and abort-finalization contract, [decision 0003](../docs/decisions/0003-foundation-contracts.md), and the [live node ledger](../docs/implementation/README.md). Boundary, ownership, limit and strict-conversion tests exercise the public skeleton without claiming PHP parsing support.

P05 delivered [production regular lexing](../src/lexer/mod.rs), [75 lexical oracle cases](../tests/fixtures/lexer/README.md), property/truncation checks, a [fuzz target](../fuzz/README.md), and [decision 0004](../docs/decisions/0004-regular-lexer.md). At P05, the compatibility gate ran the real lexer with 218 passing token channels and 82 assigned to P06. AST/parser coverage remains pending.

P06 delivered [string/interpolation/doc-string modes](../src/lexer/strings.rs), [literal utilities](../src/literal/mod.rs), and the tested [parser-authorized halt boundary](../src/halt.rs). [Decision 0005](../docs/decisions/0005-strings-literals-and-halt.md) records behavior and limits. All 300 token channels are exercised: 296 pass exactly and four record intentional standalone-lexer halt differences. Dedicated oracles cover 109 lexical streams and 139 literal values/errors.

P07 delivered the [public parser](../src/parser/mod.rs), expression grammar, interpolation, recovery, resource limits and [iterative traversal](../src/visit.rs). [Decision 0006](../docs/decisions/0006-expression-parser.md) records 133 dedicated reference observations, 69 exact upstream structure matches, eight explicit Unicode recovery differences and reassignment of 32 complete files needing later-phase syntax. No P07 structure channel remains pending; metadata and broad diagnostics stay P10/P11.

P08 delivered [procedural statements](../src/parser/statements.rs), [callables and signature types](../src/parser/callables.rs), recovery and traversal extensions. [Decision 0007](../docs/decisions/0007-procedural-parser.md) records 49 dedicated probes, 49 additional exact upstream structure matches, two explicit recovery differences and six complete-file ownership corrections to P09. Deep nesting, every probe truncation, small budgets and malformed token sequences are tested.

P09 delivered [declarations and class members](../src/parser/declarations.rs), [namespace/import parsing and grouping](../src/parser/namespaces.rs), attributes, anonymous classes, enums, promotion, hooks, asymmetric visibility and shared declaration types. [Decision 0008](../docs/decisions/0008-declaration-parser.md) records 60 dedicated probes, 96 additional exact upstream structure matches and five pinned malformed-input recovery differences. All 190 in-scope node families have implementation mappings; metadata and diagnostic completeness are still separate gates.

P10 delivered [version/source fidelity](../docs/decisions/0009-version-source-fidelity.md), semantic and enclosing origins, unique comment ownership, precise trailing-comment Nops and corrected interpolation/import/halt ranges. All 188 valid core fixtures match full AST/source observations. The 222 additional probes audit every target and enforce the 8.4 ceiling. All P10 channels are closed, with explicit malformed-input differences retained for P11 review.

Next: **P11**, complete diagnostics, recovery and parser-local validation. Follow the dependency order in the technical plan. A phase is complete only when its acceptance checks pass. Update these documents when evidence changes a design decision or the agreed scope changes.
