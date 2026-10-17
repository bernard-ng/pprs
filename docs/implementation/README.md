# Live implementation ledger

[nodes.jsonl](nodes.jsonl) maps all 193 source files in the frozen [P00 node inventory](../compatibility/nodes.jsonl), pinned to reference commit `8d2b672aff65a1a4dbf7936cec27d19f51989412`. This is an editable progress ledger; the frozen inventory remains untouched.

- `model_status`: `foundation` means a Rust representation exists (possibly only the subset described in `note`); `pending` means it will be added with its grammar phase; `deferred` means outside the PHP 7.0–8.4 ceiling.
- `rust_model`: a representation in `src/ast.rs` or its `procedural`/`declarations` modules, or null while absent. Upstream base classes and legacy aliases can map to the same Rust type; one row does not imply one distinct Rust variant.
- `parser_phase`: the planned owner, not a claim of completeness. Shared callable/type nodes span P08/P09; scalar decoding starts in P06 and AST parsing in P07. Additional normalization/recovery checks continue through P10/P11.
- `parser_status`: `slice` means P07–P09 construct this node family, with links to executable structure evidence; `pending` means its grammar is not implemented; `deferred` is outside scope. P10 closes valid-input metadata fidelity; this does not claim malformed-input or diagnostic equivalence.

P09 has **190 foundation mappings, no pending models and three deferred models**. The [declaration production ledger](declarations.md) links grammar families to their implementation owners and positive/negative evidence. Partial families are called out in each note. PPRS-only `Program`, `NodeMeta`, `Origin`, `ByteString`, diagnostic and token types do not add rows to the upstream node inventory.

The [P02 case/channel ledger](../../tests/fixtures/coverage.json) remains the acceptance authority for observable behavior. All 300 core cases now exercise every comparison channel: tokens 296 exact / 4 differences; structure/status 266 / 34; metadata 265 / 35; diagnostics/status 239 / 61. All 188 clean core cases match AST values, source metadata and diagnostics. No core channel remains pending; 13 out-of-scope originals remain deferred. The [version matrix](version-behavior.md) accounts for production version branches. Frozen inventories remain unchanged; data-type tests and the test-only P03 prototype are not parser implementation coverage.

[Recovery and validation](recovery.md) records P11 delimiter ownership, progress guarantees, diagnostic policy and parser-local validation coverage. All core comparison channels are active; malformed-input differences are reviewed and exact expectations remain enforced.

[P12 robustness and performance](../decisions/0011-robustness-and-performance.md) records bounded campaigns, lifecycle regressions, retained resource defaults and measured traversal allocation improvements.
