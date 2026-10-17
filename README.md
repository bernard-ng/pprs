# PPRS

A PHP parser library written in Rust. The first release is planned to support **PHP 7.0–8.4**, using [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) as its compatibility reference.

**Status:** P00–P10 are complete. `parse`, `parse_source` and `parse_strict` deliver expression, procedural, callable and declaration parsing, with owned ASTs, lossless tokens, diagnostics, recovery and read-only traversal. Namespaces, classes, traits, enums, attributes, promoted parameters, declaration types and PHP 8.4 property hooks are implemented. All valid core fixtures now match reference AST values, comments, spelling and source ranges. Broader diagnostic and malformed-input recovery parity remains P11. There is no product binary or PHP runtime dependency. See the [implementation plan](plans/README.md) and [P10 contract](docs/decisions/0009-version-source-fidelity.md).

## Development

The minimum supported Rust version is **1.85.0**, with edition 2024. [rust-toolchain.toml](rust-toolchain.toml) selects stable Rust with rustfmt and Clippy for normal development. Runtime dependencies are Logos for regular byte scanning, `bstr` for semantic byte strings/debugging and `thiserror` for typed errors; there are no feature flags. Test dependencies include `serde_json`, `sha2`, `hex` and Proptest. A separate [fuzz package](fuzz/README.md) uses libFuzzer.

Run the same Rust checks used in CI:

```sh
cargo fmt --all --check
cargo fmt --manifest-path tests/consumer/Cargo.toml --all --check
cargo fmt --manifest-path fuzz/Cargo.toml --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --manifest-path tests/consumer/Cargo.toml --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
cargo check --locked --manifest-path tests/consumer/Cargo.toml
cargo check --locked --manifest-path fuzz/Cargo.toml
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
```

The environment assignment on the last command uses POSIX shell syntax; in PowerShell, set `$env:RUSTDOCFLAGS = "-D warnings"` before running `cargo doc`. CI sets this environment variable for every platform. Generated API documentation is available locally through `cargo doc --open`.

Validate the minimum toolchain explicitly:

```sh
rustup toolchain install 1.85.0 --profile minimal
cargo +1.85.0 test --locked --all-targets
cargo +1.85.0 test --locked --doc
cargo +1.85.0 check --locked --manifest-path tests/consumer/Cargo.toml
```

[tests/consumer](tests/consumer/Cargo.toml) is a separate Cargo package with a path dependency on `pprs`. It verifies public lexing, parsing, strict failures, traversal, source ownership, options and AST construction through public APIs without private modules, PHP or the reference checkout. Its lockfile is checked in alongside the root lockfile. Foundation tests cover byte positions, lossless tokens, AST metadata, budgets and result contracts. The P02 integration suite reads 313 reference/probe cases and tests the comparison adapters; 296 token channels pass and four have exact intentional differences. P07–P09 provide 214 passing structure channels, 15 explicit recovery differences and 242 dedicated expression/statement/declaration observations. All 300 complete parser cases remain pending overall until later phases complete the remaining channels.

Parse the supported subset and inspect the tree:

```rust
use pprs::{parse_strict, visit, ParseOptions};

let parsed = parse_strict("<?php echo $value + 1; return [1, 2];", ParseOptions::default())?;
let program = parsed.result().program().unwrap();
assert_eq!(program.statements.len(), 2);
for entry in visit::walk(program) {
    println!("{}: {:?}", entry.depth, entry.node);
}
# Ok::<(), pprs::StrictError>(())
```

Use `parse` for recovery and inspect `ParseResult::status()` and diagnostics. Strict failures retain the recovering result; oversized input returns a separate preflight error. `parse_source` reuses a `SourceText` without copying. The effective parser depth is capped at 128 even when configured higher. Strict completion does not imply full PHP compiler validation or strict historical-version validity.

The lexer accepts arbitrary bytes and owns its output. Always distinguish completion from diagnostics:

```rust
use pprs::{lex, ParseOptions};

let result = lex("<?php echo $value + 1; ?>tail", ParseOptions::default()).unwrap();
assert!(result.is_complete());
assert!(result.diagnostics().is_empty());
```

Resource stops retain any unscanned suffix. Oversized input returns `InputTooLarge` before copying. Unfinished strings produce bounded diagnostics; an empty diagnostic list alone does not establish completion. Standalone lexing never activates halt tails; the private parser boundary validates syntax and scope first. See the [P06 contract](docs/decisions/0005-strings-literals-and-halt.md), [lexical oracle](tests/fixtures/lexer/README.md) and [literal oracle](tests/fixtures/literal/README.md).

The source API preserves raw bytes:

```rust
use pprs::SourceText;

let source = SourceText::from(&b"<?php\r\n\xff"[..]);
let name = source.span(7..8).unwrap();
assert_eq!(source.slice(name), Some(&b"\xff"[..]));
assert_eq!(source.location(7).unwrap().line, 2);
```

The [foundation decision](docs/decisions/0003-foundation-contracts.md) documents provisional limits, source/metadata invariants and strict-result semantics. Unsupported later-phase grammar produces diagnostics and fails strict parsing.

## Quality policy and CI

[CI](.github/workflows/ci.yml) runs on pushes, pull requests and manual dispatch. It checks:

- Formatting, Clippy and generated documentation on stable Rust.
- Unit/integration test targets, doctests and the independent consumer on Rust 1.85.0 (Linux) and stable Rust (Linux, macOS and Windows).
- Frozen reference metadata, exported corpus integrity, phase coverage and verifier tests using committed files only. Pull requests also check that previously passing coverage is not demoted.

Project-authored unsafe code is forbidden. Missing public documentation and Clippy's `all`/`pedantic` groups warn locally; CI promotes Rust/rustdoc/Clippy warnings to errors. Add focused lint allowances only with a reason. Keep default/no-default/all-feature and meaningful feature-combination coverage current when optional features are introduced; redundant feature jobs are unnecessary while the crate has no features.

Rust CI needs no PHP, Composer, `.references` checkout, or network corpus download. The separate baseline job uses the hosted runner's Python 3. The workflow pins action revisions and uses read-only repository permissions. Hosted workflow results will become available after these files are pushed to a GitHub repository.

## Reference and scope

- [Frozen reference and verification](reference/README.md).
- [PHP compatibility matrix and fixture inventories](docs/compatibility/README.md).
- [Behavior decisions](docs/decisions/0001-parser-behavior.md).
- [Reference testing strategy](plans/reference-and-testing-strategy.md).
- [Exported corpus contract and coverage](tests/fixtures/README.md).
- [Oracle setup and regeneration](tools/reference/README.md).
- [Selected parser strategy and experiment results](docs/decisions/0002-parser-strategy.md).
- [Foundation contracts and resource policy](docs/decisions/0003-foundation-contracts.md).
- [Live AST implementation ledger](docs/implementation/README.md).

The default PHP target is 8.4. Target-version interpretation follows nikic's best-effort behavior; parsing will not imply complete PHP compiler or strict version validation. PHP 8.5/8.6 features in the pinned reference remain outside the initial release.

## License and publication

The license for original PPRS code has not yet been chosen, so package publication is disabled with `publish = false`. No repository/documentation hosting URL is declared until one is configured. Upstream attribution is retained in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and the [copied BSD-3-Clause notice](reference/LICENSE.nikic-PHP-Parser).
