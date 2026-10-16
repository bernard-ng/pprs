# P08 procedural statements, callables and signature types

Accepted 2026-10-17. Public parsing now constructs procedural statements, named functions, closures, arrow functions and shared signature types. Namespace/class/member declarations, attributes, promoted parameters and anonymous classes remain P09. Source and tokens stay owned and byte preserving; no runtime dependency was added.

## Grammar and AST

The parser follows the frozen `grammar/php.y` and parser checks. It handles if/elseif/else and dangling else, switch/case/default, while/do/for/foreach, colon/end forms, break/continue, labels/goto, global/static/unset, declare, try/catch/finally and throw. PHP closing tags terminate eligible statements and inline HTML remains in the enclosing body. Body braces normalize to statement lists; standalone blocks keep their own node. Trailing comment no-ops follow the reference's body/case distinction. Metadata equivalence remains P10.

Named functions, closures/captures and arrow expressions share `Signature` and `Parameter`. Defaults, by-reference returns/parameters/captures, variadics, trailing commas and generators are represented explicitly. Arrow functions have implicit captures. Function declarations require a statement list or braced body. `Function`, `Closure`, and statement payloads are boxed where necessary to bound enum/frame sizes. Public auxiliary nodes carry origins and participate in iterative source-order traversal; signature grouping itself is not an extra AST node.

`Type` covers named, builtin, nullable, union and intersection types, including parenthesized intersections inside DNF unions. Unparenthesized intersections inside unions and standalone/nested type parentheses are diagnosed. Builtin names are classified by target and canonicalized only when unqualified; `static` return types remain canonical `Name` nodes. P09 will reuse this grammar and add declaration contexts. Parameter attributes, flags and hooks are not claimed as implemented; reference projection currently emits their empty values.

Target selection follows the reference lexer/parser policy, not a historical PHP compiler. For example, modern type combinations may parse on older targets, while `fn` is an ordinary identifier before PHP 7.4. Compiler checks such as duplicate parameters, required-after-optional parameters, loop-only break, resolved goto labels and generator legality are not added. Direct builtin `void` parameters, variadic defaults and try without catch/finally produce parser errors as the reference does. Yield without an operand can participate in binary operations (`yield * -1`), preserving reference precedence.

## Recovery and limits

Statement lists and recovery recognize enclosing clause keywords. Parameter recovery synchronizes on commas/closing parentheses or a body/statement boundary. A damaged parameter is discarded or represented by an explicit missing-variable expression; later parameters remain available. A missing callable body retains an empty body and leaves the following statement visible. Damaged catch headers discard that clause/body while preserving subsequent catch/finally clauses. Delimiter and progress guards, cumulative node/token/diagnostic limits, and AST-depth checks apply to new constructs. Grammar depth retains P07's conservative cap of 128. A regression caught missing switch-body depth accounting; nested switches now stop with a resource diagnostic instead of overflowing the native stack.

Two exact P08 structure differences are intentional:

- Invalid `global $$foo->bar;` is discarded through its terminator. The reference retains the partial global and synthesizes a separate `bar` constant expression. PPRS preserves valid statements after the terminator; it does not expose that suffix as valid code.
- Nested `__halt_compiler` aborts parsing without activating a raw payload. PPRS retains completed outer statements, including an empty partial program when none completed; the reference returns no AST. Resource aborts already use this partial-program contract. No statement after this fatal stop is parsed; the unscanned suffix remains in lossless tokens.

These differ from broad malformed-file parity, which remains P11. Neither difference can pass strict conversion. Each has a pinned alternate observation in the live corpus ledger; no global comparison normalization masks it.

## Evidence and accounting

P08 adds 49 exact upstream structure/status matches and two explicit differences. Six formerly P08-assigned complete files contain global constants or class/member declarations: const, staticType, disjointNormalFormTypes, intersectionTypes, parameters_trailing_comma case 1 and unionTypes. Their live structure owner is now P09; standalone signature probes independently cover those type forms. Frozen P00/P02 inventories and observations are unchanged.

There are 49 dedicated reference observations: 30 versioned type/callable cases across all ten targets and 19 focused statement, recovery, HTML and precedence cases. Offline tests verify provenance hashes, exact AST/status, every probe truncation, small node/depth/token budgets, invalid grammar, retained later statements/clauses, visitor ordering, thousand-level nesting and 256 arbitrary procedural token sequences. The original 133 expression observations remain unchanged. The shared invariant suite still checks all 300 core files. The fuzz target adds procedural/signature entry points; only compilation is claimed, not an instrumented campaign.

Combined structure coverage is 118 passing channels and ten explicit differences. No P07/P08 structure channel remains pending. All 300 complete cases remain pending overall because metadata and full diagnostic channels are not promoted. The live node ledger has 165 implemented mappings, 25 pending and three deferred; partial families are documented per row.

Reproduce with `php -n -d short_open_tag=0 tools/statements/export.php --check`. Normal Cargo tests use committed snapshots without PHP or `.references`. See [probe contract](../../tests/fixtures/statements/README.md) and [tests](../../tests/statements.rs).

Local validation passed: 62 unit/integration tests and four doctests on stable Rust and Rust 1.85.0; formatting; Clippy with warnings denied; rustdoc; independent consumer checks; fuzz-target compilation; 13 Python tooling tests; full reference verification; P02/P03/P05/P06/P07/P08 oracle reproduction; and coverage non-demotion. Hosted CI and instrumented fuzzing are not claimed.
