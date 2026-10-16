# P04 source, token, AST and result contracts

Status: accepted, 2026-10-16. Implements the foundations in [P04](../../plans/technical-implementation-plan.md), following [behavior decision 0001](0001-parser-behavior.md) and [strategy decision 0002](0002-parser-strategy.md). Production lexing/parsing entry points remain P05–P07 work. These pre-release types may evolve as the remaining syntax is implemented.

## Source ownership and positions

`SourceText` owns an `Arc<[u8]>` and a shared line-start index. `from_shared` avoids copying the source bytes; cloning shares both allocations. Borrowed byte/string conveniences copy once. The source object itself is a general byte container, not a parser resource gate: future parsing entry points must check the input budget before copying/indexing caller bytes.

`SourceText::span` validates half-open offsets and permits empty spans through EOF. Spans and token ranges carry bounds, not source identities. Keep them paired with the result that produced them; accessors recheck bounds when given ranges from elsewhere. No unchecked indexing or UTF-8 conversion is exposed. We deliberately avoid global source IDs or lifetime branding at this stage.

Lines and byte columns are one-based. LF, lone CR and CRLF each create one new line; the LF byte inside CRLF remains on the preceding line. A trailing line break creates an empty final line. Non-UTF-8 bytes and NULs each occupy one byte column. EOF is an empty span, never the reference sentinel's artificial NUL byte.

`ByteString` is a project-owned wrapper around `bstr::BString`. It exposes bytes without a lossy `Display` conversion; debugging uses the crate's byte-preserving escape representation. No dependency-specific types appear in the public signatures.

## Lossless token streams and lazy cursor

`TokenStream::new` validates contiguous, nonoverlapping, complete source coverage and a single empty final EOF. Every non-EOF token has positive width. Token text stays in the source. Token kinds are a non-exhaustive project vocabulary, extended with each lexical phase; there is no generic unknown-syntax placeholder.

Whitespace, ordinary comments, doc comments and ordinary open tags are grammar trivia. Short echo tags and close tags remain significant. Invalid tokens consume source bytes and may be followed by valid tokens. Lexer diagnostics are a separate obligation; stream construction validates coverage, not whether a kind accurately classifies its bytes.

The private `TokenCursor` requests tokens lazily, retains every trivia token and original index, and bounds significant lookahead to three tokens. EOF is sticky and never advances. Recovery must consume a token, return at an owned delimiter, or abort; it cannot retry EOF indefinitely. The producer trait is internal, with deterministic test fixtures rather than a second PHP lexer. P05 will connect Logos plus explicit byte states to this contract.

Only the parser may authorize the halt transition after validating the complete top-level `__halt_compiler();` statement. The cursor requires a consumed semicolon and no buffered suffix before switching producer state. It cannot itself prove PHP syntax or scope. If lookahead already scanned beyond the boundary, the transition fails instead of dropping those tokens. P06/P07 must test the syntax/scope checks with the real lexer.

Normal finalization scans through EOF under the token budget. Abort finalization performs **no further lexing**: it retains the scanned prefix, adds one positive-width `Unscanned` suffix if needed, then EOF. This suffix is neither invalid PHP nor a valid halt tail. `is_complete()` reports scanning completeness only; a parser can still abort after all tokens were scanned. Complete parsing must never contain an unscanned suffix. This resolves the open finalization question in P03 without forcing work beyond an exhausted budget.

## Options, limits and diagnostics

`PhpVersion` accepts exactly 7.0–7.4 and 8.0–8.4. Parsing version strings rejects whitespace, patch versions, aliases and unsupported versions; no host PHP query occurs. `ParseOptions` defaults to PHP 8.4 with short tags disabled. Fields can be changed only to already valid typed values.

`Limits::new` validates finite bounds; `Limits::check` checks proposed usage before work. Initial defaults are deliberately provisional:

| Resource | Default | Contract |
| --- | ---: | --- |
| Input bytes | 16 MiB | Check before source copy/index construction at parser ingress. Zero is allowed. |
| Grammar/AST depth | 128 | Check recursion **and constructed tree depth**, including long operator chains. |
| Retained tokens | 1,000,000 | Includes trivia, EOF and abort bookkeeping; at least two. |
| Constructed nodes | 250,000 | Includes recovery nodes; at least one. |
| Diagnostics | 100 | Includes a final limit error; at least one. |

The cursor conservatively reserves two token slots for an unscanned suffix and EOF before requesting a non-EOF token. This can stop scanning up to one ordinary token earlier than the nominal bound even if that token would end the file; bounded storage and no speculative scan take priority. The diagnostic collector reserves one entry for its terminating limit diagnostic; after exhausting that allowance, it rejects further emissions. Existing entries keep emission order. Error messages are not stable keys; code strings `PPRS-L*`, `PPRS-S*`, `PPRS-C*` and `PPRS-R*` are. Codes determine lexical/syntax/context/resource categories, and found-EOF differs from absent/not-applicable found information.

P04 tests limit validation, token and diagnostic exhaustion, and result conversion. Input ingress enforcement starts in P05; AST construction/depth enforcement starts in P07. Traversal, debugging and teardown stress checks remain P12 acceptance work. The limit fields alone do not claim that a parser or arbitrary caller-constructed AST is stack-safe. Allocation failure is outside the recoverable contract.

## Initial AST and metadata

The initial model covers names/identifiers, integer/float/string values, simple variables, constant fetches, assignment, five binary operators, error expressions, expression statements, echo, return, inline HTML and no-ops. Other variants are added with their owning grammar phase, not as empty placeholders. [The live node ledger](../implementation/nodes.jsonl) maps all 193 frozen reference node files without editing the P00 inventory or claiming parsing coverage.

`Identifier` is one nonempty component. `Name` has nonempty components plus `Normal`, `FullyQualified` or `Relative` qualification; normal names may have one or several components. Prefixes and namespace separators belong to spelling, not component values. These constructors enforce component shape, not PHP lexical/keyword validity. High bytes are preserved, and context determines permissible identifiers.

AST children use owned `Box` and `Vec`. Integer semantics use `i64` and an explicit base, floating values use binary64 `f64` (including infinities), and string values use decoded `ByteString` plus delimiter kind. Numeric overflow, sign handling and escape decoding remain P06/P07 work; P04 does not interpret literals. Raw numeric/string spelling, separators, binary prefixes and doc labels remain available via source metadata. Ordinary float equality is IEEE equality, so AST `PartialEq` is not a bit-exact oracle comparator for NaNs or signed zero; compatibility adapters must compare float bits as the P02 schema specifies.

`NodeMeta::parsed` derives byte coverage from retained-token coverage, validates comment references, and rejects unscanned node coverage. Leading/trailing comments may be outside the node's token range. Empty token ranges point to the next token's start (or EOF); missing expressions use an explicit `ExprKind::Error`. `NodeMeta::synthetic` makes no source claim and has no source comment references. Consumers editing their own ASTs must update origins explicitly; result access is immutable.

`Return(None)` means a valid absent value; `Return(Some(Error))` means a damaged/missing value. An empty program list is valid, whereas an absent root is an aborted parse. Echo's nonempty-list constraint and writable assignment targets remain parser checks. AST types support construction by tools, so constructors are not a substitute for PHP validation.

## Owned results and strict conversion

`ParseResult` owns a token stream (and therefore its source), optional program, diagnostics and status. Only the internal parser assembler can establish completion; there is no public constructor or placeholder parsing function that fabricates successful parsing. Private assembly and `into_strict` are tested with controlled inputs until P07 supplies real parsing.

- `Clean`: a root exists, completion was asserted without recovery, scanning is complete, and no error/resource diagnostic exists. Warnings alone are allowed.
- `Recovered`: a root exists and parsing completed with errors or explicit recovery.
- `Aborted`: processing stopped or a resource limit was reported; a partial root may exist. An unscanned suffix always forces abort.

`into_strict` yields `ParsedFile` only for clean results. `ParseError` retains the entire result on failure, including source and unaffected structure, and implements the standard error trait. Strict conversion does not enforce a stricter PHP language version or reparse the input. P07's assembler callers must emit diagnostics for recovery holes and maintain metadata/source consistency; P11 audits those integration invariants against malformed reference fixtures.

## Dependencies and validation

Adopted runtime dependencies: [bstr 1.13.1](https://docs.rs/bstr/1.13.1/bstr/) with default features disabled and `std` enabled, and [thiserror 2.0.21](https://docs.rs/thiserror/2.0.21/thiserror/) for conventional typed errors. Their downloaded package manifests declare MSRV 1.65 and 1.77 respectively and MIT OR Apache-2.0 licensing. Cargo resolved their dependency graph under the project's Rust 1.85 constraint; both root and consumer lockfiles are checked in. Logos stays test-only until P05.

Focused checks live in [foundation integration tests](../../tests/foundations.rs), private cursor/diagnostic/result unit tests and the [independent consumer](../../tests/consumer/src/lib.rs). They cover mixed line endings, empty files/nodes/EOF, all 256 byte values, invalid ranges/streams, owned results, explicit options, budget exhaustion, and strict failure preservation. Existing P02/P03 tests still run; no compatibility case is promoted by the existence of these data types.

Local P04 validation passed on stable Rust 1.99.0 and Rust 1.85.0: 29 unit/integration tests and one doctest, plus the independent consumer (also checked offline). Formatting, Clippy with warnings denied, and rustdoc with warnings denied pass. All 13 Python reference-tool tests, exported-corpus integrity checks and the full P00 pinned-checkout verifier pass. The unchanged CI workflow discovers the new tests through `--all-targets`; hosted CI execution still awaits a push. No oracle/exported fixture bytes changed in this phase.
