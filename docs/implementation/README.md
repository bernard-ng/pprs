# Live implementation ledger

[nodes.jsonl](nodes.jsonl) maps all 193 source files in the frozen [node inventory](../compatibility/nodes.jsonl), pinned to reference commit `8d2b672aff65a1a4dbf7936cec27d19f51989412`. This is an editable progress ledger; the frozen inventory remains untouched.

- `model_status`: `foundation` means a Rust representation exists (possibly only the subset described in `note`); `pending` means it will be added with its grammar phase; `deferred` means outside the PHP 7.0–8.4 ceiling.
- `rust_model`: a representation in `src/ast.rs` or its `procedural`/`declarations` modules, or null while absent. Upstream base classes and legacy aliases can map to the same Rust type; one row does not imply one distinct Rust variant.
- `parser_phase`: the historical implementation milestone assigned to the node, not a claim of completeness. Shared callable/type nodes belong to the callable and declaration parsers; scalar decoding belongs to literal support and AST construction to the expression parser. The source fidelity and recovery suites check normalization and malformed-input behavior.
- `parser_status`: `slice` means the parser constructs this node family, with links to executable structure evidence; `pending` means its grammar is not implemented; `deferred` is outside scope. The source fidelity audit checks valid-input metadata; this does not claim malformed-input or diagnostic equivalence.

The node ledger has **190 foundation mappings, no pending models and three deferred models**. The [declaration production ledger](declarations.md) links grammar families to their implementation owners and positive/negative evidence. Partial families are called out in each note. PPRS-only `Program`, `NodeMeta`, `Origin`, `ByteString`, diagnostic and token types do not add rows to the upstream node inventory.

The [case/channel ledger](../../tests/fixtures/coverage.json) remains the acceptance authority for observable behavior. All 300 core cases now exercise every comparison channel: tokens 296 exact / 4 differences; structure/status 266 / 34; metadata 265 / 35; diagnostics/status 239 / 61. All 188 clean core cases match AST values, source metadata and diagnostics. No core channel remains pending; 13 out-of-scope originals remain deferred. The [version matrix](version-behavior.md) accounts for production version branches. Frozen inventories remain unchanged; data-type tests and the test-only strategy prototype are not parser implementation coverage.

[Recovery and validation](recovery.md) records delimiter ownership, progress guarantees, diagnostic policy and parser-local validation coverage. All core comparison channels are active; malformed-input differences are reviewed and exact expectations remain enforced.

[Robustness and performance](../decisions/0011-robustness-and-performance.md) records bounded campaigns, lifecycle regressions, retained resource defaults and measured traversal allocation improvements.
