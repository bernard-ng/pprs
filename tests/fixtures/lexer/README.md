# P05 lexical oracle

`regular.json` contains 70 observations and `short-tags.json` contains five. Inputs come from [tools/lexer/inputs.json](../../../tools/lexer/inputs.json), shaped by the frozen `LexerTest.php`, `Lexer/EmulativeTest.php`, `Lexer.php`, keyword/version emulators and P02 edge cases. They are project-authored probes, not copied upstream test definitions.

The exporter calls the **pinned Emulative lexer only**, never PHP evaluation or the parser. It records token names, bytes, half-open spans, PPRS coordinates and lexical errors. Provenance includes the reference commit, PHP 8.5.6 / 64-bit profile, explicit short-tag setting and hashes of exporter inputs/helpers. Ordinary Rust tests verify those hashes and source definitions without PHP.

Reproduce (requires the verified `.references` checkout and pinned PHP):

```sh
php -n -d short_open_tag=0 tools/lexer/export.php --check
php -n -d short_open_tag=1 tools/lexer/export.php --short-tags --check
```

Use `--write` in place of `--check` only for reviewed fixture changes. Updating these snapshots does not modify compatibility coverage. The public Rust-to-oracle mapping is test-only in `tests/support/lexer.rs` and `token_names.rs`; a single public ampersand kind projects to the reference's contextual split.

The P02 token gate separately enforces 218 passing channels. For a read-only list of currently matching complete token streams:

```sh
PPRS_REPORT_TOKENS=1 cargo test --locked --test lexer complete_regular_corpus_tokens_match_before_coverage_promotion -- --nocapture
```

Every reported stream must pass the actual comparison. Review the result before editing `tests/fixtures/coverage.json`, then refresh `tools/reference/corpus.py --write-report`. AST/source-position/parser-diagnostic channels are independent and must not be promoted merely because lexing succeeds.
