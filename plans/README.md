# Implementation plans

Status: **P00–P03 complete** (2026-10-16). A test-only parser prototype exists; production implementation has not started.

The first stable release will be a reusable Rust library for PHP **7.0–8.4**, as confirmed on 2026-10-16. There is no product CLI.

1. [Technical implementation plan](technical-implementation-plan.md): scope, architecture, crate choices, ordered implementation steps, and release gates.
2. [Reference and testing strategy](reference-and-testing-strategy.md): pinned PHP-Parser reference, fixture inventory, oracle/export design, compatibility rules, and verification.

P00 delivered the [frozen reference](../reference/README.md), [compatibility matrix and inventories](../docs/compatibility/README.md), [behavior decisions](../docs/decisions/0001-parser-behavior.md), and [third-party notice](../THIRD_PARTY_NOTICES.md). The reference and inventory verifier passes, including its negative regression tests.

P01 delivered the [library scaffold](../src/lib.rs), [Cargo metadata and lint policy](../Cargo.toml), [independent consumer](../tests/consumer/Cargo.toml), [CI workflow](../.github/workflows/ci.yml), and [development guide](../README.md). Local checks pass on stable Rust and Rust 1.85.0; the clean consumer builds offline without the reference checkout. Hosted CI execution awaits a push to GitHub.

P02 delivered the [reference exporter](../tools/reference/README.md), [313-case corpus and comparison contract](../tests/fixtures/README.md), and [coverage report](../tests/fixtures/coverage-report.json). All 290 original dumps reproduce; the four mixed cases have supported derivations. All 300 eligible parser cases remain pending.

P03 selected Logos regular scanning with explicit byte states and recursive descent/Pratt parsing. The [strategy decision](../docs/decisions/0002-parser-strategy.md) records the comparison and the [hard cases](../tests/fixtures/strategy/README.md) remain executable tests. Production parser APIs are still pending.

Next: **P04**, implement source, token, diagnostic and AST foundations. Follow the dependency order in the technical plan. A phase is complete only when its acceptance checks pass. Update these documents when evidence changes a design decision or the agreed scope changes.
