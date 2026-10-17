# Bounded robustness campaigns

`seed.py` derives byte seeds from all 300 frozen core fixtures: original source, five sampled truncation/deletion/NUL-replacement positions, and every one-byte value. Deduplication gives 4,036 seeds including the empty input; libFuzzer reports 4,035 nonempty files. It also seeds the structured target with 256 deterministic byte vectors. No PHP or `.references` checkout is needed.

`campaign.py` creates a **new** output directory, writes fresh initial corpora, and runs each target sequentially with an explicit random seed, time bound, input-size bound, per-input timeout and RSS limit. It retains full logs and JSON evidence, including initial corpus identity, actual commands, toolchain, return status, execution count and final coverage line. Failures stop the campaign; preserve/minimize artifacts and add a normal regression before accepting a fix.

Recommended instrumented setup follows the [Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz/setup.html):

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
python3 tools/robustness/campaign.py --output target/fuzz-campaign --seconds 120
```

The target package pins `libfuzzer-sys` in its lockfile. The campaign records the actual cargo-fuzz and compiler versions. `--toolchain nightly-YYYY-MM-DD` can select an installed dated nightly. The benchmark host could not resolve `static.rust-lang.org`; its documented local run used the installed Rust 1.99 compiler's unstable instrumentation through this **explicit experimental fallback**:

```sh
python3 tools/robustness/campaign.py --output target/fuzz-campaign --seconds 120 --bootstrap-stable
```

That flag sets `RUSTC_BOOTSTRAP=1` only in child processes. It does not change the repository toolchain or disable sanitizers. The recorded campaign verified the Apple Silicon fuzz binaries linked `librustc-stable_rt.asan.dylib` and reported inline coverage counters. This fallback is not a claim of nightly execution or a portable supported stable compiler workflow.

The production library contains no fuzz-specific code or unsafe code. The byte target reuses full source/AST/diagnostic invariants; the generated target additionally requires clean parsing of its bounded grammar across target versions. It is not a differential PHP oracle. Ordinary CI runs `tests/robustness.rs`, including every target for 256 generated programs, large inputs, 2 MiB stack lifecycle checks and configured resource exhaustion. Instrumented campaigns are a separate explicit development command.
