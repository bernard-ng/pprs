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
README, changelog, MIT license and upstream notices.

Original PPRS code is MIT licensed. Reference-derived material retains the
BSD-3-Clause conditions documented in `THIRD_PARTY_NOTICES.md`. Include both
license notices in the archive. Cargo's `MIT AND BSD-3-Clause` expression records
these combined obligations; it does not offer a choice between licenses.

## Release to crates.io

Follow [Cargo's publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html).
Before publishing:

1. Update the package version and changelog. Replace the release's `unreleased`
   marker with its release date. Keep the API compatibility policy and MSRV
   current, and update README dependency examples when the compatible version
   changes.
2. Run the checks above, including Rust 1.85 tests and the isolated package
   consumer check on both stable and Rust 1.85. Commit the release changes and
   require green CI on that exact commit, including Linux, macOS and Windows.
3. Inspect `cargo package --list` and run `cargo publish --locked --dry-run` from
   the clean release checkout. The dry run builds the archive without uploading.
   Check that licenses and the embedded guide are present and that test corpora,
   local reference source and build artifacts are absent.
   Cargo may report that repository integration tests are omitted from the
   archive; those tests run separately in CI and in the checks above.
4. Confirm the crate name/version is available and the publisher's crates.io
   account has a verified email address and permission to publish. Authenticate
   locally with `cargo login`; never put a registry token in a commit or chat.
5. After the repository owner authorizes the upload, run `cargo publish --locked`.
   Confirm the version on crates.io, then tag that same commit as `v0.1.0` for
   the initial release and push the tag. Use the corresponding package version
   for later tags. Check the docs.rs build and installation from a fresh consumer.

Pushing a `v*` tag starts `.github/workflows/release.yml`. It checks the tag against
`Cargo.toml` and the dated changelog, tests and builds the library, and uploads the
portable `.crate` package and `SHA256SUMS` as an Actions artifact. A separate job
verifies the checksum and creates or updates the GitHub release with those files.
This workflow does not upload to crates.io; follow the registry steps above first.

Published versions cannot be overwritten. Correct a broken release with a new
version; use `cargo yank` when an existing version should no longer be selected
for new dependency resolutions. An upload is separate from preparing or testing
the repository.
