# PPRS

A PHP parser library written in Rust. The first release is planned to support **PHP 7.0–8.4**, using [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) as its compatibility reference.

**Status:** P00–P03 are complete: library foundations, reference harness and parser-strategy selection. The project builds as a library; parsing APIs are not implemented yet. There is no product binary or PHP runtime dependency. See the [implementation plan](plans/README.md) for subsequent phases.

## Development

The minimum supported Rust version is **1.85.0**, with edition 2024. [rust-toolchain.toml](rust-toolchain.toml) selects stable Rust with rustfmt and Clippy for normal development. The library currently has no runtime dependencies or feature flags. Test-only dependencies are `serde_json`, `sha2` and `hex` for corpus checks, plus Logos for the test-only strategy prototype.

Run the same Rust checks used in CI:

```sh
cargo fmt --all --check
cargo fmt --manifest-path tests/consumer/Cargo.toml --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --manifest-path tests/consumer/Cargo.toml --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
cargo check --locked --manifest-path tests/consumer/Cargo.toml
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

[tests/consumer](tests/consumer/Cargo.toml) is a separate Cargo package with a path dependency on `pprs`. It verifies that another project can import the library without private modules, PHP or the reference checkout. Its lockfile is checked in alongside the root lockfile. The import doctest and consumer build check the scaffold. The P02 integration suite reads 313 reference/probe cases and tests the comparison adapters; all 300 eligible parser cases remain pending implementation.

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

The default PHP target will be 8.4. Target-version interpretation follows nikic's best-effort behavior; parsing will not imply complete PHP compiler or strict version validation. PHP 8.5/8.6 features in the pinned reference remain outside the initial release.

## License and publication

The license for original PPRS code has not yet been chosen, so package publication is disabled with `publish = false`. No repository/documentation hosting URL is declared until one is configured. Upstream attribution is retained in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and the [copied BSD-3-Clause notice](reference/LICENSE.nikic-PHP-Parser).
