# PHP compatibility

PPRS targets PHP 7.0 through 8.4, with 8.4 as the fixed default. Version selection
follows the pinned PHP-Parser reference. It changes interpretation without enforcing
every PHP compiler restriction or rejecting every newer construct on older targets.
See the [API guide](../guide.md) for the consumer contract.

The [comparison ledger](../../tests/fixtures/coverage.json) checks all 300 core
cases across tokens, AST structure, source metadata and diagnostics. Of these,
235 match every channel and 65 have reviewed differences. All 188 clean core cases
match AST values, metadata and diagnostics. Thirteen out-of-scope original cases
remain deferred. The [corpus contract](../../tests/fixtures/README.md) explains the
comparisons and exact alternative expectations.

## Language feature matrix

`Y` means the feature exists in that PHP language version. `no` means it had not
arrived or had been removed. `baseline` means present by PHP 7.0. These cells
record language history, not a strict parser rejection table. The reference can
accept newer or obsolete syntax under other targets within the supported ceiling.
The pinned grammar, token emulators and PHP migration guides supply the feature
dates. Exact source references appear in [features.json](features.json).

| Feature ID / behavior | Since | 7.0 | 7.1 | 7.2 | 7.3 | 7.4 | 8.0 | 8.1 | 8.2 | 8.3 | 8.4 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `file`, File structure, PHP tags, inline HTML and blocks | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `names`, Names, variables, contextual and semi-reserved identifiers | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `comments`, Comments, no-ops, positions and spelling metadata | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `recovery`, Lexical errors, grammar recovery and contextual diagnostics | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `numbers`, Numeric bases, overflow and floating-point values | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `numeric-separators`, Numeric literal separators | 7.4 | no | no | no | no | Y | Y | Y | Y | Y | Y |
| `explicit-octal`, Explicit 0o/0O octal literals | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `strings`, Byte strings, interpolation and shell strings | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `unicode-escapes`, Unicode escape syntax in strings | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `interpolated-offsets`, Negative offsets in simple string interpolation | 7.1 | no | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `heredoc`, Heredoc and nowdoc delimiters/newlines | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `flexible-heredoc`, Flexible heredoc/nowdoc indentation and closing labels | 7.3 | no | no | no | Y | Y | Y | Y | Y | Y | Y |
| `magic-constants`, Magic constant nodes without evaluation | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `property-magic`, __PROPERTY__ magic constant | 8.4 | no | no | no | no | no | no | no | no | no | Y |
| `operators`, Unary, binary, assignment, ternary and coalescing operators | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `coalesce-assignment`, Null-coalescing assignment operator | 7.4 | no | no | no | no | Y | Y | Y | Y | Y | Y |
| `precedence`, PHP 8 concatenation precedence; ternary language change | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `casts`, Scalar/object/array casts and historical spellings | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `variables`, Variables, variable variables and static members | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `access`, Uniform variable syntax, calls and dereference chains | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `nullsafe`, Nullsafe member access | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `curly-offsets`, Legacy curly-brace array/string offsets | baseline | Y | Y | Y | Y | Y | no | no | no | no | no |
| `arrays`, Arrays and list-pattern infrastructure | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `short-destructuring`, Short and keyed destructuring forms | 7.1 | no | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `list-references`, References in destructuring | 7.3 | no | no | no | Y | Y | Y | Y | Y | Y | Y |
| `array-unpack`, Unpacking in array literals | 7.4 | no | no | no | no | Y | Y | Y | Y | Y | Y |
| `calls`, Call arguments, unpacking and reference forms | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `call-trailing-comma`, Trailing commas in calls | 7.3 | no | no | no | Y | Y | Y | Y | Y | Y | Y |
| `named-arguments`, Named arguments | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `first-class-callables`, First-class callable ellipsis | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `new-clone`, Instantiation and ordinary clone expressions | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `new-dereference`, Dereference new expressions without surrounding parentheses | 8.4 | no | no | no | no | no | no | no | no | no | Y |
| `constructs`, include/require/eval/print/isset/empty/exit language constructs | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `exit-call`, Extended exit/die call syntax and stub names | 8.4 | no | no | no | no | no | no | no | no | no | Y |
| `match`, Match expressions | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `throw-expression`, Throw as an expression | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `closures`, Closures, capture lists and callable references | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `arrow-functions`, Arrow functions | 7.4 | no | no | no | no | Y | Y | Y | Y | Y | Y |
| `generators`, Yield, yield-from and generator binding | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `statements`, Statements, declare, static/global variables, labels and unset | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `control-flow`, Conditionals, switch and alternate syntax | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `loops`, For, foreach, while and do loops | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `exceptions`, Try/catch/finally structure | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `multi-catch`, Multiple exception types in catch | 7.1 | no | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `optional-catch-variable`, Catch without a bound variable | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `functions`, Functions, conditional declarations and parameters | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `parameter-trailing-comma`, Trailing commas in parameters and closure captures | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `types`, Named/built-in types and target-sensitive classification | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `nullable-void-iterable`, Nullable types and void/iterable built-ins | 7.1 | no | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `object-type`, object built-in type | 7.2 | no | no | Y | Y | Y | Y | Y | Y | Y | Y |
| `union-types`, Union types, mixed and static return types | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `intersection-types`, Intersection types and never return type | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `dnf-types`, DNF types and standalone null/false/true | 8.2 | no | no | no | no | no | no | no | Y | Y | Y |
| `namespaces`, Namespaces, imports, aliases and namespace normalization | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `group-imports`, Grouped namespace imports | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `group-trailing-comma`, Trailing commas in grouped imports | 7.2 | no | no | Y | Y | Y | Y | Y | Y | Y | Y |
| `classes`, Classes/interfaces/traits, modifiers and members | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `anonymous-classes`, Anonymous classes | 7.0 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `traits`, Trait use and adaptations | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `trait-constants`, Constants declared in traits | 8.2 | no | no | no | no | no | no | no | Y | Y | Y |
| `trait-final`, Final trait-method adaptation | 8.3 | no | no | no | no | no | no | no | no | Y | Y |
| `constants`, Global/class constants and constant fetches | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `class-constant-visibility`, Class constant visibility | 7.1 | no | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `final-constants`, Final class constants | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `typed-constants`, Typed class/interface/trait/enum constants | 8.3 | no | no | no | no | no | no | no | no | Y | Y |
| `dynamic-class-constant`, Dynamic class constant fetch | 8.3 | no | no | no | no | no | no | no | no | Y | Y |
| `properties`, Properties and property modifier validation | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `typed-properties`, Typed properties | 7.4 | no | no | no | no | Y | Y | Y | Y | Y | Y |
| `promotion`, Constructor property promotion | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `readonly-properties`, Readonly properties | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `readonly-classes`, Readonly classes | 8.2 | no | no | no | no | no | no | no | Y | Y | Y |
| `readonly-anonymous`, Readonly anonymous classes | 8.3 | no | no | no | no | no | no | no | no | Y | Y |
| `enums`, Unit/backed enums and enum cases | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `attributes`, Attribute groups on in-scope declaration targets | 8.0 | no | no | no | no | no | Y | Y | Y | Y | Y |
| `property-hooks`, Property hooks, abstract/final properties and hook bodies | 8.4 | no | no | no | no | no | no | no | no | no | Y |
| `asymmetric-visibility`, Asymmetric property visibility | 8.4 | no | no | no | no | no | no | no | no | no | Y |
| `new-initializers`, New expressions in permitted initializers | 8.1 | no | no | no | no | no | no | Y | Y | Y | Y |
| `static-initializers`, Arbitrary static variable initializers | 8.3 | no | no | no | no | no | no | no | no | Y | Y |
| `halt`, __halt_compiler syntax, scope and raw tail | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `ast-model`, AST common structures, bases and compatibility aliases | baseline | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |

## Version transitions

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


## Outside the supported scope

PHP 5 target behavior and syntax introduced after PHP 8.4 are outside scope.
This includes the pipe operator, void casts, expanded clone calls, attributes on
global constants, final promoted parameters and partial function application.
Printing, formatting-preserving transforms, name resolution, constant evaluation
and a builder API are also outside the library contract.

## Reference inventories

The frozen catalogs record 192 grammar nonterminals, 193 node source files,
530 fixture cases in 288 files and 58 unit-test source files. They preserve the
original scope inventory. Their implementation-phase and status fields are
historical; use the comparison ledger for current results.

- [Grammar productions](grammar.jsonl).
- [Node files](nodes.jsonl).
- [Fixture cases](fixtures.jsonl).
- [Unit-test sources](unit-tests.jsonl).
- [Reference revision, hashes and verification](../../reference/README.md).

Mixed upstream fixtures retain their original bytes. Derived in-scope cases have
separate IDs and provenance. Printer fixtures remain inventoried without claiming
printer support. The inventories and source hashes must not change merely to
accept a compatibility regression.
