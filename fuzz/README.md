# Lexer fuzz target

`lex_bytes` feeds arbitrary bytes through file, PHP, quote, backtick, heredoc/nowdoc and interpolation entry states, varying target versions, short tags and nesting limits. It asserts contiguous exact byte coverage, EOF placement, diagnostic bounds and token/diagnostic limits, including partial results. It also exercises escape and numeric decoding. Input is bounded at 64 KiB and retained tokens at 4096. It does not claim differential PHP compatibility.

The separate development package uses `libfuzzer-sys` 0.4.13; it adds no product binary or runtime dependency. Its lockfile is committed. Stable CI compiles it with:

```sh
cargo check --locked --manifest-path fuzz/Cargo.toml
```

For instrumented fuzzing, install nightly and [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz/setup.html), then run from the repository root:

```sh
cargo +nightly fuzz run lex_bytes -- -max_total_time=60 -max_len=65536
```

The target is compiled locally at P05/P06; no instrumented campaign is claimed (nightly/cargo-fuzz are not installed in the development environment). Normal CI already runs arbitrary-byte Proptest and exhaustive byte/truncation checks. P12 will record bounded fuzz campaigns. Corpus, artifacts, coverage and target output are ignored; minimize discovered failures into checked-in regression tests before accepting a fix.
