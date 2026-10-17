# P10 version behavior and source fidelity

Status: accepted, 2026-10-17. Reference pin and PHP 7.0–8.4 scope are unchanged.

## Observable contract

Every one of the **188 clean core fixtures** now matches the pinned reference in AST structure, decoded values, comment ownership, byte/token ranges, coordinates and spelling attributes. Across all 300 core cases, 262 position channels match exactly and 38 malformed-input cases have individual reviewed expectations. P10 closes its 37 structure obligations with 35 exact matches and two recovery differences. The live totals are 249 exact structure channels, 17 structure differences and 34 P11 structure obligations. All 300 diagnostic channels remain P11 work; complete-case compatibility is not claimed.

`NodeMeta::origin()` describes the semantic node. Grouping parentheses no longer overwrite its location. The new `enclosing_origin()` includes the outermost grouping parentheses when present, otherwise returning the ordinary origin. This preserves both reference-compatible node ranges and the source needed by later printers. Grouping still participates in assignment/dereference grammar checks. The optional enclosing origin is boxed to keep node and recursive parser frames small; it contains no recursive AST. Existing depth, node, token and diagnostic budgets remain in force.

Each attached comment belongs to one node, selected in outermost-first source traversal. A private iterative metadata walk removes duplicate descendant attachments after namespace normalization and recovery. It covers all typed node variants; the public traversal API is unchanged. Unattached comments remain in the lossless token stream. Trailing-comment Nops are zero-width at the last comment's end, before any following whitespace. Explicit empty array elements retain the offending comma's range; absent destructuring elements remain absent.

Other corrected ranges include `${...}` interpolation, callable names synthesized from `exit`/`clone` tokens, typed group-use items, and `__halt_compiler()` statements. A halt node covers the statement, while its raw payload remains a separate token and semantic byte string.

## Spelling and comparison boundary

The library retains exact source bytes and token ranges rather than introducing a mutable PHP-style attribute dictionary. Typed array/string/namespace/base variants and source slices preserve numeric separators and casing, quote/binary prefixes, heredoc labels and indentation, cast aliases, exit/die spelling, and explicit namespace form. Semantic and enclosing origins are independently checked against retained token spans.

The independent test adapter now maps every represented node and auxiliary node to a reference metadata path. It projects spelling from the retained source, including stripped-but-undecoded heredoc text segments, hook property names and close-tag newline ownership. It uses a pinned node-field schema to follow reference traversal order; it does not use expected fixture values to construct observations. Only the pre-existing `raw_attributes` provenance exclusion applies to comparisons. Production parsing contains no JSON or oracle dependencies.

## Version behavior and ceiling

The [implementation matrix](../implementation/version-behavior.md) records every version-sensitive branch and distinguishes parser behavior from compiler introduction dates and printer-only helpers. The 222 additional probes cover all ten targets, source metadata and 45 ceiling checks. All 132 clean, in-scope reference probes match structure and metadata exactly. The suite has 143 complete exact observations and 79 explicitly recorded differences: 39 historical malformed-input recovery observations and 40 ceiling observations. Five ceiling probes already match the reference's rejection.

P07 accidentally accepted expanded clone calls from the newer grammar. P10 corrects that: zero/multiple arguments, unpacking, named arguments, placeholders and trailing-comma clone forms produce `UnsupportedSyntax`, as do global `function clone` declarations. Ordinary unary `clone $x` and `clone($x)` remain supported. This deliberately supersedes P07's complex-clone acceptance statement. The original P07 oracle is unchanged; its `clone-forms` probe has one explicit status difference. PHP 8.4 exit/die call behavior remains supported. Global constant attributes and final promotion remain diagnosed; pipe, void casts and partial-application syntax fail clean parsing under both old and new supported targets.

All nine future/mixed original fixtures were retargeted to 7.0 and 8.4 and rejected. The four PHP 5.6 originals remain outside the configuration domain. No frozen inventory, original source fixture or reference file was changed.

## Malformed-input decisions and evidence

The [per-case recovery review](../../tests/fixtures/fidelity/recovery-review.json) names all 38 metadata differences and their exact expectation files. They describe retained error expressions, zero-width missing names, statement/member synchronization, partial trees after missing delimiters, previously documented Unicode/class/import recovery, or complete ranges on invalid mixed namespaces. These are explicit current observations, not a claim that P11 recovery equivalence is finished. The two new core structure differences are PHP 8 curly-offset recovery and an incomplete function signature at EOF. No valid core fixture uses an intentional-difference exemption.

The [fidelity fixtures](../../tests/fixtures/fidelity/README.md) preserve exporter/input hashes, the reference commit, PHP host version and word size. Offline Rust tests run full comparisons, every new probe's byte prefixes, small resource budgets, metadata mutation checks and public API assertions. Shared invariants now check unique comment ownership, semantic/enclosing token-to-byte ranges and bounds even on damaged input and resource aborts. Native-stack and arbitrary-input regressions from prior phases remain active.

Validation: 75 Rust tests and five doctests pass on stable and Rust 1.85.0; one explicit development audit exporter is ignored during ordinary test runs. Formatting, Clippy, rustdoc, consumer/fuzz compilation, 13 Python tooling tests, frozen-reference verification, coverage non-demotion and all oracle reproduction checks pass locally. Fuzz targets were compiled; no instrumented fuzz campaign or hosted CI execution is claimed.
