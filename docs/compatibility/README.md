# PHP 7.0–8.4 compatibility matrix

P00 completed on 2026-10-16. **This is the frozen implementation scope, not a claim of implemented support.** All parser features are still unimplemented. P01/P02 foundations and reference harness are complete; the next implementation phase is P03.

## Reading the matrix

Targets are PHP 7.0, 7.1, 7.2, 7.3, 7.4, 8.0, 8.1, 8.2, 8.3 and 8.4. The default is fixed at 8.4. `Y` means the feature exists in that PHP language version and belongs to the planned release; `—` means it had not arrived or had been removed from the language. `baseline` means present by PHP7.0, not necessarily introduced in 7.0. Shared grammar/recovery infrastructure is marked across the range.

**These cells are not a strict rejection table.** Nikic uses best-effort targets and accepts some newer/obsolete syntax under other targets. PPRS follows that interpretation within its 8.4 ceiling. Feature dates, actual upstream target behavior and future implementation status are distinct. See [D02](../decisions/0001-parser-behavior.md#d02--explicit-versions-fixed-default-best-effort-interpretation).

The machine-readable [feature catalog](features.json) records lexical requirements, exact upstream nonterminal names, fixture-file associations, planned Rust owners, implementation phases, source references and notes. [grammar.jsonl](grammar.jsonl) maps **all 192 nonterminals**, with source line ranges; [nodes.jsonl](nodes.jsonl) accounts for **all 193 node source files**, including aliases and abstract bases. Shared productions can contain both in-scope and deferred alternatives; a family mapping does not claim every alternative is already tested or implemented.

[fixtures.jsonl](fixtures.jsonl) assigns all **530 cases in 288 fixture files**. [unit-tests.jsonl](unit-tests.jsonl) classifies all **58 unit-test source files** for parser behavior or later helper capabilities. The frozen reference, hashes, verification commands and recovery procedure are in [reference/README.md](../../reference/README.md).

## Planned core feature coverage

Language-version evidence comes from the pinned grammar/emulators/`PhpVersion.php` and PHP's migration guides. Each feature's exact source links are in the catalog. Current implementation state for every row: **not implemented**.

| Feature ID / behavior | Since | 7.0 | 7.1 | 7.2 | 7.3 | 7.4 | 8.0 | 8.1 | 8.2 | 8.3 | 8.4 | Phase |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `file` — File structure, PHP tags, inline HTML and blocks | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P05 |
| `names` — Names, variables, contextual and semi-reserved identifiers | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P05 |
| `comments` — Comments, no-ops, positions and spelling metadata | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P10 |
| `recovery` — Lexical errors, grammar recovery and contextual diagnostics | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P11 |
| `numbers` — Numeric bases, overflow and floating-point values | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P06 |
| `numeric-separators` — Numeric literal separators | 7.4 | — | — | — | — | Y | Y | Y | Y | Y | Y | P06 |
| `explicit-octal` — Explicit 0o/0O octal literals | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P06 |
| `strings` — Byte strings, interpolation and shell strings | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P06 |
| `unicode-escapes` — Unicode escape syntax in strings | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P06 |
| `interpolated-offsets` — Negative offsets in simple string interpolation | 7.1 | — | Y | Y | Y | Y | Y | Y | Y | Y | Y | P06 |
| `heredoc` — Heredoc and nowdoc delimiters/newlines | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P06 |
| `flexible-heredoc` — Flexible heredoc/nowdoc indentation and closing labels | 7.3 | — | — | — | Y | Y | Y | Y | Y | Y | Y | P06 |
| `magic-constants` — Magic constant nodes (without evaluation) | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `property-magic` — __PROPERTY__ magic constant | 8.4 | — | — | — | — | — | — | — | — | — | Y | P09 |
| `operators` — Unary, binary, assignment, ternary and coalescing operators | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `coalesce-assignment` — Null-coalescing assignment operator | 7.4 | — | — | — | — | Y | Y | Y | Y | Y | Y | P07 |
| `precedence` — PHP 8 concatenation precedence; ternary language change | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P10 |
| `casts` — Scalar/object/array casts and historical spellings | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `variables` — Variables, variable variables and static members | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `access` — Uniform variable syntax, calls and dereference chains | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `nullsafe` — Nullsafe member access | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P07 |
| `curly-offsets` — Legacy curly-brace array/string offsets | baseline | Y | Y | Y | Y | Y | — | — | — | — | — | P10 |
| `arrays` — Arrays and list-pattern infrastructure | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `short-destructuring` — Short and keyed destructuring forms | 7.1 | — | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `list-references` — References in destructuring | 7.3 | — | — | — | Y | Y | Y | Y | Y | Y | Y | P07 |
| `array-unpack` — Unpacking in array literals | 7.4 | — | — | — | — | Y | Y | Y | Y | Y | Y | P07 |
| `calls` — Call arguments, unpacking and reference forms | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `call-trailing-comma` — Trailing commas in calls | 7.3 | — | — | — | Y | Y | Y | Y | Y | Y | Y | P07 |
| `named-arguments` — Named arguments | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P07 |
| `first-class-callables` — First-class callable ellipsis | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P07 |
| `new-clone` — Instantiation and ordinary clone expressions | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `new-dereference` — Dereference new expressions without surrounding parentheses | 8.4 | — | — | — | — | — | — | — | — | — | Y | P07 |
| `constructs` — include/require/eval/print/isset/empty/exit language constructs | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P07 |
| `exit-call` — Extended exit/die call syntax and stub names | 8.4 | — | — | — | — | — | — | — | — | — | Y | P07 |
| `match` — Match expressions | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P07 |
| `throw-expression` — Throw as an expression | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P07 |
| `closures` — Closures, capture lists and callable references | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `arrow-functions` — Arrow functions | 7.4 | — | — | — | — | Y | Y | Y | Y | Y | Y | P08 |
| `generators` — Yield, yield-from and generator binding | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `statements` — Statements, declare, static/global variables, labels and unset | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `control-flow` — Conditionals, switch and alternate syntax | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `loops` — For, foreach, while and do loops | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `exceptions` — Try/catch/finally structure | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `multi-catch` — Multiple exception types in catch | 7.1 | — | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `optional-catch-variable` — Catch without a bound variable | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P08 |
| `functions` — Functions, conditional declarations and parameters | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `parameter-trailing-comma` — Trailing commas in parameters and closure captures | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P08 |
| `types` — Named/built-in types and target-sensitive classification | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `nullable-void-iterable` — Nullable types and void/iterable built-ins | 7.1 | — | Y | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `object-type` — object built-in type | 7.2 | — | — | Y | Y | Y | Y | Y | Y | Y | Y | P08 |
| `union-types` — Union types, mixed and static return types | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P08 |
| `intersection-types` — Intersection types and never return type | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P08 |
| `dnf-types` — DNF types and standalone null/false/true | 8.2 | — | — | — | — | — | — | — | Y | Y | Y | P08 |
| `namespaces` — Namespaces, imports, aliases and namespace normalization | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `group-imports` — Grouped namespace imports | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `group-trailing-comma` — Trailing commas in grouped imports | 7.2 | — | — | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `classes` — Classes/interfaces/traits, modifiers and members | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `anonymous-classes` — Anonymous classes | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `traits` — Trait use and adaptations | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `trait-constants` — Constants declared in traits | 8.2 | — | — | — | — | — | — | — | Y | Y | Y | P09 |
| `trait-final` — Final trait-method adaptation | 8.3 | — | — | — | — | — | — | — | — | Y | Y | P09 |
| `constants` — Global/class constants and constant fetches | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `class-constant-visibility` — Class constant visibility | 7.1 | — | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `final-constants` — Final class constants | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P09 |
| `typed-constants` — Typed class/interface/trait/enum constants | 8.3 | — | — | — | — | — | — | — | — | Y | Y | P09 |
| `dynamic-class-constant` — Dynamic class constant fetch | 8.3 | — | — | — | — | — | — | — | — | Y | Y | P07 |
| `properties` — Properties and property modifier validation | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P09 |
| `typed-properties` — Typed properties | 7.4 | — | — | — | — | Y | Y | Y | Y | Y | Y | P09 |
| `promotion` — Constructor property promotion | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P09 |
| `readonly-properties` — Readonly properties | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P09 |
| `readonly-classes` — Readonly classes | 8.2 | — | — | — | — | — | — | — | Y | Y | Y | P09 |
| `readonly-anonymous` — Readonly anonymous classes | 8.3 | — | — | — | — | — | — | — | — | Y | Y | P09 |
| `enums` — Unit/backed enums and enum cases | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P09 |
| `attributes` — Attribute groups on in-scope declaration targets | 8.0 | — | — | — | — | — | Y | Y | Y | Y | Y | P09 |
| `property-hooks` — Property hooks, abstract/final properties and hook bodies | 8.4 | — | — | — | — | — | — | — | — | — | Y | P09 |
| `asymmetric-visibility` — Asymmetric property visibility | 8.4 | — | — | — | — | — | — | — | — | — | Y | P09 |
| `new-initializers` — New expressions in permitted initializers | 8.1 | — | — | — | — | — | — | Y | Y | Y | Y | P09 |
| `static-initializers` — Arbitrary static variable initializers | 8.3 | — | — | — | — | — | — | — | — | Y | Y | P08 |
| `halt` — __halt_compiler syntax, scope and raw tail | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P06 |
| `ast-model` — AST common structures, bases and compatibility aliases | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P04 |

## Version transitions and interpretation obligations

| Target boundary | Required parser work and tests | Language source |
|---|---|---|
| PHP 7.0 baseline | Scalar/return type syntax, uniform variable syntax, anonymous classes, group imports, spaceship/coalesce, yield-from and Unicode escapes; preserve earlier syntax still present | Pinned grammar and [PHP 7.0 migration](https://www.php.net/manual/en/migration70.new-features.php) |
| PHP 7.1 | Nullable/void/iterable types, short/keyed destructuring, class-constant visibility, multi-catch, negative interpolated offsets; identifier DEL handling changes | [PHP 7.1 migration](https://www.php.net/manual/en/migration71.new-features.php) and pinned `PhpVersion.php` |
| PHP 7.2 | `object` classification and trailing comma in grouped imports | [PHP 7.2 migration](https://www.php.net/manual/en/migration72.new-features.php) |
| PHP 7.3 | Flexible heredoc, call trailing commas, list references; keep call and parameter comma rules separate | [PHP 7.3 migration](https://www.php.net/manual/en/migration73.new-features.php) |
| PHP 7.4 | Typed properties, arrow functions, number separators, coalesce assignment, array unpack; contextual `fn`; curly offsets still in PHP7 grammar | [PHP 7.4 migration](https://www.php.net/manual/en/migration74.new-features.php) |
| PHP 8.0 | PHP8 concat binding and removal of curly-offset productions; attributes, named arguments, union/mixed/static types, promotion, match, nullsafe, expression throw, optional catch variables, parameter/capture commas | [PHP 8.0 features](https://www.php.net/manual/en/migration80.new-features.php), [incompatibilities](https://www.php.net/manual/en/migration80.incompatible.php) |
| PHP 8.1 | Enums, readonly properties, intersection/never types, explicit octal, first-class callables, final constants, new-in-initializers; named-after-unpack form | [PHP 8.1 migration](https://www.php.net/manual/en/migration81.new-features.php) |
| PHP 8.2 | DNF/standalone types, readonly classes, trait constants; `readonly` function-name handling | [PHP 8.2 migration](https://www.php.net/manual/en/migration82.new-features.php) and keyword emulator |
| PHP 8.3 | Typed constants, dynamic class-constant fetch, readonly anonymous classes, final trait adaptation, arbitrary static initializers | [PHP 8.3 migration](https://www.php.net/manual/en/migration83.new-features.php) |
| PHP 8.4 | Property hooks, asymmetric visibility, property magic constant, new-expression dereferencing, extended exit/die forms, abstract/final property syntax | [PHP 8.4 migration](https://www.php.net/manual/en/migration84.new-features.php), pinned grammar/emulators |

This is syntax/AST scope. Runtime additions such as fibers, lazy objects, new standard functions and attribute execution semantics need no new interpreter in PPRS. They use ordinary parsed declarations/calls/attributes. Type variance, runtime constant-expression validity, property behavior and execution are not compiler promises.

Important upstream behavior to preserve or document separately:

- PHP7/PHP8 concat precedence and curly-offset branches are explicit grammar differences.
- Built-in type classification is version-sensitive, but standalone type legality is not fully enforced. `true` recognition changes at 8.2; `null`/`false` recognition starts at 8.0.
- The grammar retains left ternary precedence and legacy casts even where PHP8 compilation differs. Strict parse success only reports the parser's own errors.
- Qualified-name token normalization and `&` splitting are upstream internal conventions; preserve equivalent meaning/locations without exposing PHP token numbers.
- Version-sensitive lexer rules must be implemented without depending on PHP's host tokenizer. Generic keyword classification alone is insufficient.

## Fixture dispositions

The parser inventory is **277 core + 9 deferred + 4 mixed = 290**. Core includes positive, negative, and intentionally permissive parser cases. P02 exported all original cases and reproduced their original expected dumps. Four derived cases and 19 targeted probes bring the comparable corpus to 300 cases, all pending Rust implementation; see the [coverage report](../../tests/fixtures/coverage-report.json). These figures are not parser test-pass counts.

| Original case | Scope | Reason / next action |
|---|---|---|
| `parser/expr/argumentList.test#0` | deferred | Empty clone call is accepted by newer parser grammar; not an in-scope valid clone expression. |
| `parser/expr/argumentList.test#1` | mixed | Ordinary call forms plus PHP8.5 clone arguments; call-only derivation exported in P02. |
| `parser/expr/assignNewByRef.test#0` | deferred | Explicit PHP5.x target interpretation is outside the supported target range. |
| `parser/expr/partialFunctionApplication.test#0` | deferred | Experimental PHP8.6 placeholders; no in-scope original case. |
| `parser/expr/partialFunctionApplication.test#1` | deferred | PHP7.4 mode still expects PHP8.6 placeholders; mode is not scope. |
| `parser/expr/pipe.test#0` | deferred | PHP8.5 pipe syntax. |
| `parser/scalar/invalidOctal.test#1` | deferred | Explicit PHP5.x target interpretation is outside the supported target range. |
| `parser/scalar/unicodeEscape.test#1` | deferred | Explicit PHP5.x target interpretation is outside the supported target range. |
| `parser/stmt/class/property_promotion.test#0` | mixed | PHP8.0/8.1/8.4 promotion forms plus PHP8.5 final promotion; derivation without final parameter exported in P02. |
| `parser/stmt/const.test#0` | mixed | Plain constants plus PHP8.5 constant attributes; plain-constant derivation exported in P02. |
| `parser/stmt/function/clone_function.test#0` | mixed | Ordinary clone expressions plus PHP8.5 clone-call/stub syntax; ordinary-expression derivation exported in P02. |
| `parser/stmt/function/typeVersions.test#0` | deferred | Explicit PHP5.x target interpretation is outside the supported target range. |
| `parser/stmt/voidCast.test#0` | deferred | PHP8.5 void cast. |

All **240 printing-related cases** (95 standard-printer and 145 formatting-preservation cases) are inventoried as deferred product capabilities. Their internal PHP-version subsets are not classified for printer conformance in P00. The unit-test inventory likewise assigns builder, resolver, evaluator and printer tests to later milestones; core traversal reuse is limited to the read-only subset.

Do not exclude ordinary `expr/clone.test` or the PHP7 error case in `expr/assignNewByRef.test` merely because nearby files exercise newer/older syntax. Mixed originals remain unchanged; derive new IDs with provenance and regenerate expected results in P02. No automatic filename/version-tag filtering should replace these dispositions.

## Deferred feature families

| ID | Scope origin | Planned milestone |
|---|---|---|
| `legacy-five` — PHP 5.x target-specific interpretation | 5.x | E08 |
| `pipe` — Pipe operator | 8.5 | E07 |
| `void-cast` — Void cast | 8.5 | E07 |
| `clone-call` — Expanded clone call arguments and clone stub name | 8.5 | E07 |
| `constant-attributes` — Attributes on global constants | 8.5 | E07 |
| `final-promotion` — Final modifier on promoted parameters | 8.5 | E07 |
| `partial-application` — Experimental partial function application | 8.6 | E07 |
| `printer` — Standard printing | extension | E04 |
| `format-preservation` — Formatting-preserving transforms | extension | E05 |

## Coverage gaps to fill during implementation

Every upstream family has an owner, but the upstream tests do not provide a complete version-boundary suite. Add explicit target transitions for all ten versions, keyword contexts and 7.0/7.1 DEL identifiers. Add focused trait-constant/final-adaptation and arbitrary-static-initializer cases rather than treating broad fixture association as proof of those subfeatures. Port lexical modes, source locations and literal edge cases from the inventoried PHPUnit sources as needed.

P02 now distinguishes raw sections from expanded sources, retains modes/macros, provisions a pinned oracle profile, and generates explicit 8.4-target expectations where upstream defaults to 8.5. P04/P05 must add raw-byte and CR-only source-map checks because upstream canonicalization removes some newline distinctions. P10/P11 must close alternative-level grammar, metadata, contextual-check and recovery gaps. P00's catalog is the starting map, not a replacement for those gates.

## P00 acceptance record

- The unchanged, clean reference is pinned by commit/tree and hashes for every tracked file.
- Every grammar nonterminal, node source file, fixture case and unit-test source file is inventoried; no family is unclassified.
- PHP5 and PHP8.5/8.6 scope is separated from supported negative/reference-permissive cases; mixed cases have explicit P02 actions.
- [Behavior decisions D01–D06](../decisions/0001-parser-behavior.md) fix bytes, spans, numeric model, target/default semantics and recovery compatibility.
- Upstream license text and [third-party notice](../../THIRD_PARTY_NOTICES.md) are retained; the original-code package license remains a later maintainer decision.
- Reference/metadata verification and the verifier's negative regression tests pass. No Rust parser or upstream PHPUnit conformance result is claimed.
