# Contributing

## Check a change

The repository uses stable Rust for normal work and checks Rust 1.85 as its MSRV.
Run these commands from the repository root:

```sh
cargo fmt --all --check
cargo fmt --manifest-path tests/consumer/Cargo.toml --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --manifest-path tests/consumer/Cargo.toml --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
cargo test --locked --manifest-path tests/consumer/Cargo.toml
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
python3 tools/check_package.py --toolchain stable
```

The environment assignment uses POSIX shell syntax. In PowerShell, set
`$env:RUSTDOCFLAGS = "-D warnings"` before running `cargo doc`.
Repeat the tests with `cargo +1.85.0` for MSRV validation. CI also runs tests on
macOS and Windows. Rust checks use committed fixtures and need no PHP or
`.references` checkout. The package check uses Python 3.12 or later and an offline
Cargo cache populated by the preceding checks. It tests the Cargo archive from
a temporary directory outside the checkout.

Project code forbids unsafe Rust. CI rejects Rust, rustdoc and Clippy warnings.
Keep internal AST traversal matches exhaustive. Update the traversal coverage
and ordering tests when adding nodes. Follow the compatibility policy in the
[API guide](docs/guide.md#api-compatibility).

## Compare behavior

The [reference record](reference/README.md) pins the PHP-Parser checkout.
The [export tools](tools/reference/README.md) describe regeneration, and the
[corpus contract](tests/fixtures/README.md) defines comparisons. Preserve frozen
provenance. Give each intentional difference an explanation and an exact expected
observation. Do not disable a comparison to make a test pass.

```sh
python3 tools/reference/verify_p00.py --metadata-only
python3 tools/reference/corpus.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/reference -p 'test_*.py' -v
```

The [fuzz runner](tools/robustness/README.md) records bounded instrumented runs.
The [measurement package](tools/perf/README.md) measures release performance,
allocations and process memory. Both are separate from the library package.
Keep their formatting, lint and compile checks in sync with CI.

## Package and attribution

`Cargo.toml` lists the files included in the library archive. Tests, PHP tools,
reference source, benchmark evidence and repository planning material are not
runtime dependencies. The package includes the Rust source, embedded API guide,
README and upstream notices.

Publication remains disabled in Cargo metadata. Choosing the license for original
PPRS code and authorizing publication are separate repository-owner decisions.
The copied upstream BSD license does not license original PPRS code.
