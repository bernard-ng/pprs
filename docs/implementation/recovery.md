# Recovery and validation ledger

This review compares recovery behavior with `.references/lib/PhpParser/ParserAbstract.php`, `Modifiers.php`, the generated PHP 7/8 parsers and all 112 malformed core observations. See [decision 0010](../decisions/0010-diagnostics-and-recovery.md) for the public contract and [case reviews](../../tests/fixtures/recovery/core-review.json) for exact retained behavior.

## Recovery ownership and progress

| Boundary | Owner / implementation | Progress or exit |
| --- | --- | --- |
| Statement / top-level file | `parser/mod.rs`: `run`, `recover_statement`, `terminate` | Insert a missing terminator at a recognized next statement; otherwise scan balanced delimiters to a terminator, declaration/control anchor, owned brace, clause boundary or EOF. The outer loop consumes one token if no progress was made. |
| Nested body / alternatives / switch / try | `parser/statements.rs`: `statement_list`, scoped body and clause parsers | The caller owns its closing token or clause. Unexpected clauses are diagnosed and consumed. Failed statements synchronize; an unchanged cursor is advanced. A completed body returns at its owned delimiter. |
| Expressions / arguments / arrays | `parser/expression.rs`: Pratt, `arguments`, `array` | Operators and separators consume tokens. Missing operands at expression boundaries become explicit error expressions. Unambiguous statement starts return failure without consumption. Missing list delimiters propagate to the statement/member owner, which discards damaged syntax through synchronization. The implementation does not reconstruct every reference list item. |
| Parameters / return types | `parser/callables.rs`: `parameters`, `recover_signature` | Synchronize with bounded nesting at comma, right parenthesis, body brace, terminator or EOF. Parameter loop consumes a separator, advances on no progress, exits at its delimiter, or propagates failure at a body boundary. |
| Class members / trait adaptations | `parser/declarations.rs`: `class_members`, `recover_member`, `trait_use` | Preserve the next member anchor or enclosing brace; scan through nested braces or a terminator. Member/adaptation loops guard unchanged cursors. Missing methods/types may discard that damaged member, not an unrelated following method. |
| Property hooks | `hook_list`, `starts_member_in_hook` | Three-token lookahead identifies a subsequent member; report a missing closing brace and leave the member for its owner. Otherwise recover within the hook list and advance on no progress. |
| Namespaces / imports | `parser/namespaces.rs` plus statement recovery | Parse/group complete namespace declarations; use statement/declaration anchors for damaged imports. Namespace grouping checks placement/style/nesting after parsing. Malformed grouping can retain different ranges from the reference. |
| Strings / interpolation | `parser/strings.rs`, lexer string states | Each segment consumes a token or propagates failure; delimiters are owned by string/interpolation productions. Unterminated states report lexical failure at EOF. A failed expression bubbles to statement recovery; the lexer retains mode-specific tokens and all bytes. |
| Halt | `halt.rs` | Peek before consuming each required token. A mismatch leaves that token for statement recovery. Nested halt aborts. Only a fully consumed outermost boundary enables raw-tail state. |
| EOF | All loops | EOF is sticky. Loops stop/return failure there; no recovery loop relies on consuming EOF for progress. |
| Resource stop | Parser, cursor, scanner | Depth (including native-stack ceiling), tokens, nodes and diagnostics are bounded. Stop on failure; do not request additional tokens after abort. The result retains an explicit unscanned suffix and EOF sentinel. |

Every nesting/recovery loop either consumes input, returns to its delimiter owner, or reaches a configured limit. Dedicated tests cover all focused byte prefixes, damaged statement/member survival, tiny limits, repeated parsing, and strict conversion. Original parser/declaration arbitrary-token and arbitrary-byte regressions remain active. The [robustness report](../benchmarks/robustness-performance.md) records the bounded instrumented fuzz campaigns.

## Parser-local validation ownership

| Reference rule | Rust owner | Regression evidence |
| --- | --- | --- |
| Duplicate/access/final-abstract modifier conflicts | `declarations::modifiers` | Upstream modifier/asymmetric-visibility fixtures; related-location probes |
| Reserved class/interface/enum names and inheritance names | Declaration checks | `class/name` and alias fixtures; trait name clean control |
| Static constructor/destructor/clone; readonly method | `declarations::method` | All static-method fixtures plus compound static/readonly constructor |
| Static/abstract/readonly class constants | `class_member` | Constant-modifier fixtures plus three simultaneous errors |
| Reserved use aliases | `namespaces::use_item` | Namespace invalid-name fixtures |
| Hooks on multiple properties, empty lists, get parameters, unknown names, allowed modifiers | Property/hook parsers | Every invalid upstream hook case and focused probes |
| Variadic defaults / void parameters | `callables::parameter` | Ten-target checks; PHP 7.0 treats `void` as a name, matching the reference |
| Try without catch/finally | `statements::try_statement` | Upstream and ten-target checks |
| Namespace position, mixed styles, nesting, outside code | Namespace parser/grouping | All invalid namespace fixtures and focused probes |
| Empty array items vs assignment patterns | Delayed `Parser::run` validation | Upstream empty-element fixture and destructuring clean control |
| Invalid lvalues / references / foreach targets / grammar contexts | Expression and statement parsers | Expression, statement and declaration context tests and malformed foreach fixtures |
| Outer-scope halt | Statement/halt boundary | Upstream nested halt and focused mismatch/survival checks |
| Literal validity, indentation and escapes | Literal decoding / string parser | literal oracle, invalid octal/Unicode/doc core cases |
| Features beyond PHP 8.4 | Explicit unsupported syntax checks and grammar | version-ceiling suite remains active |

Symbol resolution, duplicate declarations/parameters, undefined labels, break/continue scope, type inhabitation and other compiler/runtime checks are intentionally absent. The reference accepts the corresponding clean controls; no later semantic-validation phase is implied by the parser API.
