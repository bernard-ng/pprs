# Regular lexer and file boundaries

Accepted 2026-10-16. Public `lex(input, options)` and `lex_source(source, options)` now return owned `LexResult` values. Logos 0.16.1 is a production dependency, retaining byte mode and `forbid_unsafe`. Its generated token kinds remain private; the public vocabulary belongs to PPRS.

## Supported behavior

The explicit file state handles HTML, PHP open tags, short echo tags, closing tags and their owned newline. Ordinary `<?php` is case-insensitive and requires space, tab, CR, LF or EOF after `php`. It consumes one whitespace unit, treating CRLF as one unit. Short `<?` tags obey the option; `<?=` always opens PHP. BOMs, hashbangs, NULs and non-UTF-8 HTML remain original bytes. Close tags stay significant so the future parser can implement statement termination.

The regular Logos scanner recognizes whitespace, byte identifiers, qualified/fully-qualified/relative names, variables, punctuation, operators, casts and numeric spellings. Explicit comment scanning retains ordinary/doc comments, stops line comments at a closing tag, and reports unterminated block comments. Keyword comparison folds ASCII only. Property-name context persists across whitespace/comments after `->` and `?->`, accepts keywords as identifiers, and splits `$x->Foo\Bar` into property `Foo` and fully-qualified `\Bar`, matching the frozen reference.

Version rules follow the pinned **PHP 8.5.6-hosted reference**, not a strict historical tokenizer. `fn` reverses before 7.4; `match` before 8.0; `enum`/`readonly` before 8.1; `__PROPERTY__` and asymmetric visibility before 8.4. Explicit octal, attributes and nullsafe tokens remain available at older targets as the reference permits. PHP 8.5 pipe and void-cast tokens are split into their earlier regular tokens, preserving the 8.4 feature ceiling. Syntax validity remains parser work.

Host-tokenizer behavior is frozen by exported observations, never queried at runtime. Notable details discovered by the regular lexer probes:

- `enum` needs a following name after whitespace/comments. It is an ordinary identifier before `(`, `;` or `$`.
- `yield from` can include intervening comments in the pinned host; all those bytes belong to its combined token.
- Property-name mode treats `#[` as a line-comment start while awaiting the property identifier.
- The native ampersand split skips whitespace **and comments** and recognizes a following dollar sign even without a valid variable name. This differs from the old-runtime fallback in `Lexer::postprocessTokens`, which skips whitespace only. PPRS retains one public `Ampersand` kind; the test adapter reconstructs the frozen split without altering bytes or spans.

Numeric token classification uses checked `i64` accumulation to match the reference's 64-bit overflow boundary, independently of Rust pointer width. It does not yet produce scalar values. Invalid octal scanning stops numeric conversion at the first invalid digit; overflow reached before that digit still produces a float token. Malformed spellings such as `0x`, `1e+`, or `1__2` split exactly as the tokenizer does. They do not get invented lexical diagnostics: invalid-octal/value validation belongs to literal decoding and parser diagnostics.

## Incomplete modes and resource limits

The initial regular lexer stopped before quotes, heredoc/nowdoc and halt keywords with `LexStop::Deferred`. It retained the remaining bytes as `Unscanned` plus EOF. The [string and halt implementation](0005-strings-literals-and-halt.md) later replaced this temporary capability boundary. A consumer must check `is_complete()`/`stop()`; an empty diagnostic list alone does not establish completion.

`is_complete()` means all bytes were scanned, not that the file is error-free or syntactically valid. Invalid bytes produce diagnostics and positive-width `Invalid` tokens, allowing later code to remain visible. Unterminated comments retain their full spelling as comment tokens. Diagnostic codes and spans are stable; invalid-byte messages use escaped hexadecimal instead of embedding control bytes.

Input-size rejection returns typed `InputTooLarge` before copying bytes or building the line index. `lex_source` uses existing shared storage but checks the same bound. This preflight failure has no owned lexical result. Token/diagnostic exhaustion after accepting input returns a `LexResult`, a resource diagnostic, and an explicit unscanned suffix. The reserved-slot rules in the foundation contract bound both collections. Depth/node budgets have no work to count in this nonrecursive lexer; they connect to strings/AST construction later.

The lazy cursor now accepts fallible token production and drains bounded lexical diagnostics. Its parser lookahead and halt-transition contracts remain private. No public function claims full parsing or authorizes a halt based on lexical resemblance alone.

## Evidence and coverage

[The dedicated corpus](../../tests/fixtures/lexer/README.md) contains 75 full-stream observations: 70 with short tags disabled and five enabled, including all ten PHP target versions. The [production lexer tests](../../tests/lexer.rs) compare kinds, exact bytes, byte spans, coordinates, and lexical diagnostic codes/spans. Original fixture lexing also compares lexical diagnostics separately from parser diagnostics.

The live reference corpus ledger promotes **218 token channels** to passing, each enforced through the actual production adapter in `tests/compatibility.rs`. At that point, **82 token channels** awaited string, heredoc and halt handling. All 300 complete parser cases were still pending because AST construction, positions and parser diagnostics were not yet implemented. No frozen reference baseline inventory or original oracle case changed.

The reference bootstrap gained an explicit short-tag profile parameter with its old default unchanged. Re-exporting the reference corpus reproduced all original expected dumps and case bytes; only the bootstrap digest in its manifest changed. The parser strategy study remains reproducible.

Robustness checks include all 256 byte values in PHP context, every truncation of every dedicated input, 256 Proptest cases combining arbitrary bytes, versions, modes and small budgets, and focused abort/recovery tests. Proptest **1.11.0** (`std`, no default features, `~1.11.0`) declares MSRV 1.85 and MIT OR Apache-2.0 licensing; 1.12 requires Rust 1.88 and was not selected. A separate [libFuzzer target](../../fuzz/README.md) is compiled in CI. The later [robustness and performance report](../benchmarks/robustness-performance.md) records instrumented fuzz campaigns and production measurements.

Local validation passed on stable and Rust 1.85: 36 Rust tests and two doctests on each toolchain, plus the downstream consumer check. Formatting, Clippy with warnings denied, rustdoc with warnings denied, fuzz-target compilation, 13 Python tooling tests, frozen-reference verification, and reference, strategy and lexer export reproducibility also passed. The fuzz target was compiled only; no instrumented campaign or hosted CI run is claimed.
