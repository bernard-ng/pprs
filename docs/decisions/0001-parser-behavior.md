# P00 behavior decisions

Status: accepted implementation baseline, 2026-10-16. These decisions define future behavior; no parser is implemented yet. Changes require an explicit decision update and corresponding compatibility tests.

## D01 — Library boundary and reference contract

Deliver one Rust library for PHP 7.0–7.4 and 8.0–8.4. Runtime parsing has no PHP, filesystem, subprocess, network, global configuration, or product-binary dependency. Callers supply source and options. A development reference verifier/exporter is separate tooling.

The pinned nikic grammar, actions, AST definitions and parser-local checks are the main compatibility authority. Official PHP documentation supplies language-version context. PHP compiler/runtime validity is a distinct question; no interpreter, semantic/type checker, standard printer or namespace resolver is implied by parsing.

Follow reference meaning rather than PHP implementation details: use typed Rust structures, retain raw origin metadata, and keep framework-specific types private. Complete AST topology can differ where a documented normalization preserves meaning. Parser-framework selection remains P03 work; this decision does not preselect a crate.

Evidence: upstream `grammar/php.y`, `ParserAbstract.php`, `Node/`, `ParserFactory.php`; [frozen reference](../../reference/README.md).

## D02 — Explicit versions, fixed default, best-effort interpretation

Accept exactly the ten major/minor targets listed above. Default to **8.4**, independent of the development machine or any installed PHP. Reject unsupported configuration values rather than silently rounding to a version or using a future grammar.

Version selection follows nikic's best-effort interpretation: keyword/token handling, PHP7/PHP8 grammar branches, precedence and built-in type classification may change with the target. It does not guarantee rejection of all syntax introduced after that target. Feature introduction dates in the matrix describe PHP language history, not an implemented strict-validation table.

Examples of required distinctions:

- Concatenation precedence differs between PHP7 and PHP8; implement both tables.
- Curly-brace offsets have PHP7-only productions; preserve that branch boundary.
- `fn` and `readonly` have contextual/reverse-emulation behavior, not a global reserved-word replacement rule.
- The reference recognizes `null`/`false` as built-in types from 8.0, although standalone use arrived in PHP 8.2. Do not rewrite reference AST classification to manufacture strict version validation.
- The reference grammar uses left-associative ternary precedence and retains some obsolete casts. Do not promise full PHP8 compiler checks that are absent from the pinned parser.

The product's feature ceiling is **8.4**. Do not implement 8.5 pipe/void/expanded-clone/constant-attribute/final-promotion productions or 8.6 placeholders merely because this newer reference accepts them under older targets. A token sequence that has an older valid interpretation still uses that interpretation. Newer feature productions remain deferred, and newer constructs must not become cleanly accepted through those productions.

`parse_strict` means “no error diagnostics/recovery/abort,” **not** “strict PHP-version validation.” Both entry points use the same grammar and target rules. A strict version-validation mode is deferred.

Evidence: `ParserFactory.php::createForVersion`, `PhpVersion.php`, `grammar/php.y` PHP7/PHP8 branches, `Lexer/Emulative.php`, `test/code/parser/expr/partialFunctionApplication.test` and `CHANGELOG.md`.

## D03 — Bytes, positions and source preservation

Accept bytes as the fundamental source representation; offer UTF-8 string conveniences without requiring UTF-8. Semantic names and decoded string values can contain arbitrary bytes. Never use lossy UTF-8 conversion during parsing or fixture normalization. Preserve original source and raw literal spelling. A parse result owns/shares source storage so it can outlive the caller's input buffer.

Use validated half-open byte and token ranges `[start, end)`. EOF is `[source_length, source_length)`. Empty/error origins are explicit empty spans; an absent origin is not offset zero. Check additions/conversions before indexing. The source can be sliced from ranges without decoding it.

Convert reference positions at the compatibility boundary:

- AST file/token ends are inclusive: convert carefully, including inverted zero-length cases.
- `Token::getEndPos()` is already exclusive; the reference's sentinel text `"\0"` contributes no source byte.
- Error ends need category-specific conversion. Unterminated-comment errors use an exclusive token end; EOF is a point. Keep raw attributes so conversion is auditable.

PPRS line/column accessors use one-based lines and **byte** columns. Recognize LF, CRLF as one break, and lone CR as one break; do not change source bytes. An offset inside a multi-byte line ending remains a byte offset in the preceding line; the next line begins after the complete ending. Unicode/editor-display coordinates belong in adapters.

This deliberately provides a consistent line index where upstream helpers disagree: native tokenization counts lone CR as a new line, while `Token::getEndLine()` counts only LF and `Error` column calculations search LF. The structured oracle should recompute comparable coordinates from source spans under the PPRS rule, preserving raw upstream attributes separately. Add CR-only, CRLF, mixed-line-ending and EOF cases in P04/P02; do not hide byte-span differences as line-normalization differences.

Default `short_open_tag=false`; expose an explicit option. `<?=` remains recognized. Do not strip BOM/hashbang/leading HTML bytes. Preserve whitespace/comments and raw `__halt_compiler()` tails; invalid halt syntax/scope cannot silently hide the rest of the input. Parser-aware halt handling remains a P03/P06 implementation decision.

Evidence: `Token.php`, `Lexer.php`, `Error.php`, `ParserAbstract.php`, byte-string fixtures; the [P00 probe](../compatibility/observations.md).

## D04 — Numeric interpretation

Use a **64-bit PHP integer model** regardless of the Rust host's pointer width. Represent supported integer values without loss, use binary64 floating-point semantics for the relevant PHP conversions, and retain the original literal spelling/base. Overflow can change integer-token interpretation to a floating value; do not clamp to `i64::MAX` or infer semantics from `usize`.

Treat a sign as syntax around a numeric literal where the grammar does so; the spelling of a minimum-value expression is not sufficient reason to fold its unary operation into a literal. Follow upstream special handling for numeric offsets inside interpolation. Malformed octal behavior differs for PHP5 versus PHP7; only PHP7+ target behavior is in scope.

Fixture exchange uses byte-safe values and tagged non-finite floats/large integers. Host PHP word size and version belong in the P02 export profile. A 32-bit PHP interpretation mode, arbitrary-precision arithmetic and constant-expression execution are deferred.

Evidence: `Node/Scalar/Int_.php`, `Float_.php`, `ParserAbstract.php::parseLNumber`/`parseNumString`, `scalar/int.test`, `float.test` and `encapsedNegVarOffset.test`.

## D05 — Recovery, diagnostics and success

Recovering parsing returns source, tokens, diagnostics and a complete or partial AST when possible. Distinguish clean completion, completion with errors, and abort/no recoverable root. Empty valid input is a clean empty program, not failure. An aborted parse may expose a useful prefix but must never present it as a complete program.

Strict parsing succeeds only on clean completion. It preserves diagnostic information on failure; it must not conceal lexical errors just because the remaining tokens form a parseable tree. Syntax errors are normal input outcomes, not panics.

Use structured lexical, syntax, parser-context and resource-limit diagnostics with byte spans, stable codes and ordered messages. Match the detected problem, useful position and surviving unaffected structure; exact English wording and LR recovery choices inside damaged regions are not public compatibility promises. Every intentional difference receives a focused expectation and explanation.

Every recovery loop must advance, return at a delimiter owned by its caller, or stop under a budget. Introduce finite interim limits for input size, nesting/tree depth, nodes/tokens and diagnostics from P04; select measured defaults in P12. Guard traversal and teardown as well as parser recursion. System-wide allocation failure is not a promised recoverable condition.

Apply parser-local checks supported by the reference (modifiers, namespace placement, parameters, hooks, destructuring and similar constraints). Do not infer complete class inheritance, constant-expression legality, symbol resolution, type or runtime checking.

Evidence: `ParserAbstract.php`, `ErrorHandler/Collecting.php`, `Expr/Error.php`, `CodeParsingTest.php`, and the 111 upstream parser error cases.

## D06 — Provenance and test scope

Freeze the Git commit/tree, all tracked bytes and the upstream license. Preserve source path/section IDs before porting any fixture. Treat the 530 cases as a whole-checkout inventory, not the supported parser count.

P00 assigns all 290 parser cases: **277 core, nine deferred, four mixed**. Mixed cases require explicit derived inputs and regenerated expectations in P02; their original expected dump is not an acceptance oracle for a shortened input. All 240 printing cases are separately deferred as product capabilities. The 277 core cases include malformed inputs and deliberately permissive parser behavior, not just valid PHP programs.

Raw-section hashes in the P00 inventory are not canonicalized/macro-expanded source hashes. P02 must use the upstream canonicalize → macro expansion → section parsing order and preserve original modes separately from the explicit in-scope oracle target. Normal Cargo tests consume checked-in exports with no PHP requirement.

Ordinary clone expressions remain core; expanded clone calls do not. `expr/argumentList.test#1` is mixed even though its mode is PHP7.4. `partialFunctionApplication.test#1` is deferred despite the same old mode. No filename-only or version-tag-only filter can replace classification.

Evidence: [case inventory](../compatibility/fixtures.jsonl), [matrix](../compatibility/README.md), [corpus contract](../../tests/fixtures/README.md).
