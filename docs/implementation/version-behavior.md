# Implemented PHP version behavior

Version and source fidelity audit, 2026-10-17. The fixed default is **8.4**; accepted targets are exactly 7.0–7.4 and 8.0–8.4. This matrix describes the frozen PHP 8.5.6-hosted nikic reference's parser behavior, not PHP compiler validation. Evidence is in the [222-probe oracle](../../tests/fixtures/fidelity/fidelity.json), [core channel ledger](../../tests/fixtures/coverage.json), and earlier lexical/literal/declaration oracles.

| Transition | Implemented target behavior | Implementation owner / probe |
| --- | --- | --- |
| 7.0 baseline | `bool`, `int`, `float`, `string`, `array`, `callable` are builtin types; reference assignment-to-new and invalid octal errors apply throughout the range; Unicode escapes and unparenthesized yield apply throughout | `parser/callables.rs::atomic_type`, `parser/expression.rs`, `literal/`; `builtin-types`, `unicode-numbers`, existing numeric/yield probes |
| 7.1 | `iterable` and `void` become builtin types | `parser/callables.rs`; `builtin-types/7.0`, `/7.1` |
| 7.2 | `object` becomes builtin | `parser/callables.rs`; `builtin-types/7.1`, `/7.2` |
| 7.3 | No additional target-dependent parser branch; flexible doc strings and call commas follow the permissive reference even at earlier targets | `parser/strings.rs`, `parser/expression.rs`; `flexible-doc` and `commas` at all targets |
| 7.4 | `fn` becomes a keyword; arrow functions become recognizable | `lexer/keywords.rs::keyword`; `arrow-keyword` at all targets |
| 8.0 | Concatenation binds below addition/shifts; curly offsets leave the grammar; `match` becomes a keyword; `mixed`, `null`, `false` become builtins; property-hook grammar uses the PHP 8 family | `parser/operators.rs::binary`, `parser/expression.rs::{expression_inner,class_target_inner}`, `lexer/keywords.rs`, `parser/callables.rs`, `parser/declarations.rs::property_hooks`; `precedence`, `curly-offset`, `match-keyword`, `builtin-types`, `property-keyword` |
| 8.1 | `enum` and `readonly` token emulation changes; `never` becomes builtin | `lexer/keywords.rs`, `parser/callables.rs`; `enum-keyword`, `readonly-keyword`, `builtin-types` |
| 8.2 | `true` becomes builtin | `parser/callables.rs`; `builtin-types/8.1`, `/8.2` |
| 8.3 | No additional parser target branch; typed class constants follow reference grammar permissiveness | declaration and type probes and core typed-constant fixtures |
| 8.4 | `__PROPERTY__` keyword and combined asymmetric-visibility tokens become available | `lexer/keywords.rs`, `lexer/scanner.rs`; `property-keyword` and regular-lexer oracle |
| All targets | Left-associative full ternary and short ternary, coalesce associativity, canonical builtin type spelling, attributes/nullsafe tokens, numeric separators/explicit octal, DNF types and new dereferences follow the reference | `precedence`, `attributes`, `nullsafe`, `unicode-numbers`, `union-intersection`, `new-deref` |

The table accounts for every explicit version comparison in production parser/lexer code. Other uses merely pass the selected version into these policies. No new parser-version branch was required by these version probes. Future changes to version comparisons must update this table and add a boundary probe.

Several upstream `PhpVersion` helpers are used by printers, not parser acceptance. `supportsShortArrayDestructuring`, `supportsFlexibleHeredoc`, `supportsTrailingCommaInParamList`, `supportsAttributes`, `supportsNewDereferenceWithoutParentheses` and `allowsDelInIdentifiers` must not become invented parser rejection gates. In particular, the pinned host rejects DEL lexically at every target, including 7.0; ten probes record that behavior and its recovery differences. Arbitrary high bytes continue to be preserved without UTF-8 conversion.

## Fixed feature ceiling

| Deferred family | Enforced behavior |
| --- | --- |
| PHP 8.5 pipe / void cast | Lex as the supported older token sequence; malformed constructs are diagnosed |
| PHP 8.5 expanded clone / global clone declaration | Explicit unsupported-syntax diagnostic, retaining checked recovery syntax where available |
| PHP 8.5 global constant attributes / final promotion | Explicit unsupported-syntax diagnostic |
| PHP 8.6 partial-application placeholders | Unsupported argument grammar fails clean parsing |
| PHP 5.x / PHP 8.5+ target configuration | Reject unsupported version values |

`ceiling/*` probes cover 7.0, 8.0 and 8.4 plus all nine future/mixed original fixtures at 7.0 and 8.4. A supported older interpretation, such as ordinary unary cloning, remains valid. `parse_strict` requires no errors; it is not a strict historical PHP compiler validator.

## Source fidelity support

All 188 valid core parser fixtures and all 132 valid in-scope new probes have exact structure/value/comment/span/spelling matches. No valid-input difference is exempted. The source projection covers every AST family represented by expression, statement and declaration parsing, including auxiliary types, arguments, items, branches, attributes and hooks. Original source bytes retain spelling even when semantic values are canonicalized or decoded. `origin()` excludes grouping parentheses from the semantic child; `enclosing_origin()` retains the outer grouped form.

The [diagnostic and recovery decision](../decisions/0010-diagnostics-and-recovery.md) records exact, individually explained expectations for malformed input. The [robustness report](../benchmarks/robustness-performance.md) records instrumented campaigns and performance baselines. The frozen reference baseline feature inventory remains historical; this document and the live coverage report describe implementation status.
