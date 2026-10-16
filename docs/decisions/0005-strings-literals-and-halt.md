# P06 strings, literal interpretation and halt boundaries

Accepted 2026-10-16. P06 extends the production lexer with string modes and adds byte-oriented literal utilities. P07 remains responsible for parsing embedded expressions and building scalar AST nodes.

## Lexical states and source preservation

`lex` / `lex_source` now scan single/double-quoted strings, binary prefixes, backticks, simple variable/property/array-offset interpolation, `${...}` / `{$...}` expressions, nested quoted expressions, and dynamically labelled heredoc/nowdoc. Public tokens distinguish raw string parts, delimiters, interpolation openers, variable names and numeric offsets. Nowdoc never interprets interpolation or escapes. Doc labels may be indented and end before punctuation; identifier continuations prevent a false closing-label match. Raw indentation, labels and newlines remain token bytes.

A private stack drives these modes without input-driven Rust recursion. The configured depth budget counts mode frames and nested expression braces, including braces in enclosing interpolation frames. Input, token and diagnostic limits still apply. Resource stops retain any remaining source as `Unscanned` plus EOF. The temporary P05 `DeferredSyntax` / `LexStop::Deferred` API is removed; string syntax no longer causes a capability stop.

Unfinished quotes, doc strings and interpolation emit `PPRS-L003` with an input span. The reference lexer leaves these failures for its parser, so the lexical comparison helper excludes only this additional code from reference-error comparison; dedicated tests require it. `LexResult::is_complete()` means scanning **and EOF checks** finished without a resource stop, not that PHP is valid. An EOF diagnostic can exhaust its budget after all bytes have been scanned: in that case the token stream covers all bytes, while `LexResult::stop()` still reports the limit. Consumers must inspect both completion and diagnostics.

For malformed simple array offsets, native PHP sometimes emits a zero-length `T_ENCAPSED_AND_WHITESPACE` transition token. PPRS retains its positive-width token invariant and omits that empty transition. The dedicated oracle retains the native token; its test projection explicitly removes only zero-length string-part tokens. No source byte, nonempty token or diagnostic is dropped. This is separate from the P02 exact comparison, whose current cases contain no such empty transitions.

## Literal values

The public `literal` module operates on byte slices and returns owned `ByteString` values or typed errors with input-relative byte ranges. It does not evaluate PHP. Source/token spans remain the authority for raw spelling.

- `decode_quoted` validates complete single/double-quoted delimiters and binary prefixes before decoding a constant string token.
- `decode_escapes` accepts an explicit single-quote, double-quote, backtick, heredoc or nowdoc mode. Unknown escapes retain their backslash. Octal escapes wrap to one byte, hex escapes consume up to two digits, and Unicode escapes reject malformed sequences and values above U+10FFFF. The frozen reference encodes surrogate code points as bytes rather than rejecting them; PPRS matches this behavior without coercing the result to Rust `String`.
- The reference first replaces escaped quote/backtick delimiters and then processes other escapes. PPRS follows that order and keeps error ranges mapped to the original segment.
- `decode_doc_segment` strips closing-label indentation before escape interpretation. Its first/last flags support text segments around interpolation, including an empty initial segment when an expression precedes required indentation. It handles blank lines, mixed tabs/spaces, insufficient indentation and removal of the final LF/CRLF/CR. As in the reference utility, interior indentation anchors after LF, including the LF in CRLF; lone CR is preserved without inventing another indentation rule. Errors identify the first failure; parser recovery/value substitution belongs to P07/P11.
- `decode_number` validates one complete unsigned numeric spelling, uses signed 64-bit integers and binary64 floats, and promotes overflow independently of pointer width. Decimal conversion uses Rust's standard float parser; radix conversion follows the reference's overflow accumulation. Separators, bases, invalid octal digits, underflow and infinity are covered. A malformed octal remains an error while an integer; overflow reached before an invalid digit produces a float from the valid prefix, as the reference does. Unary signs belong to expression parsing.
- `decode_numeric_offset` produces an integer only for canonical decimal spellings in the signed 64-bit range. Leading zeroes, prefixes, separators, negative zero and overflow retain their byte-string value.

All supported target versions share these literal-utility rules. Earlier versions continue using the pinned reference's best-effort tokenization rather than imposing strict historical compiler validity. No new dependency was needed: Logos, standard conversions and the existing byte/error types cover the generic work.

## Halt authorization

The production scanner emits `HaltCompiler` as an ordinary keyword. **Standalone `lex` never authorizes a raw halt tail.** Native PHP's tokenizer instead skips into raw bytes after three significant tokens following the keyword, even when they are not `();` or the scope is invalid. Reproducing that behavior would hide malformed input before a parser can validate it.

The private [halt boundary](../../src/halt.rs) consumes the exact keyword, empty parentheses and semicolon or closing-tag terminator. The statement parser must supply the resolved outermost scope. Only after syntax/scope validation does it switch the lazy cursor into terminal `HaltTail` mode, before requesting any suffix token. The cursor rejects already-buffered suffixes. The close-tag token retains its owned newline; subsequent bytes become the tail unchanged, including NULs, non-UTF-8 bytes and apparent PHP delimiters.

Tests exercise this boundary with the **production scanner**, including empty tails, comments between grammar tokens, closing tags, malformed syntax and nested scope. P07 will call this existing boundary from its statement parser; P06 does not claim a public parsing API or independent scope inference.

The P02 standalone-lexer gate records **296 passing token channels and four exact intentional differences** for halt tails. Each difference has a checked-in alternate token observation and rationale; it is not a skip or pending implementation. The full parser comparison will use the parser-authorized tail when available. All 300 complete parser cases remain pending.

## Evidence and reproduction

[Lexical oracles](../../tests/fixtures/lexer/README.md) now contain **109 observations** (104 short tags disabled, five enabled), extending P05's 75 with 34 string-state probes. [Literal oracles](../../tests/fixtures/literal/README.md) contain **139 observations**, checked against frozen PHP-Parser methods and, for doc strings, its parser. No PHP code is evaluated. Existing P02 observations and frozen inventories remain unchanged.

Rust tests verify exact token bytes/kinds/spans/coordinates, reference lexical diagnostics, additional unfinished-string diagnostics, scalar bytes/integer values/float bits, error classifications, nesting budgets and provenance. Robustness checks exercise all 256 bytes in ten lexical entry states, every truncation of every lexical probe, and Proptest inputs for file/PHP modes, string states and literal utilities. The separate libFuzzer target now seeds all these lexical modes and checks literal bounds; it is compiled, not claimed as an instrumented fuzz campaign. P12 retains campaign responsibility.

Local validation passed on stable and Rust 1.85: 44 Rust tests and two doctests on each, plus the downstream consumer check. Formatting, Clippy/rustdoc with warnings denied, fuzz-target compilation, 13 Python tooling tests, full frozen-reference verification and P02/P03/P05/P06 export reproducibility also passed. The exact-depth regression passed on both toolchains after the final review correction. No hosted CI run is claimed.
