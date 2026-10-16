# Live implementation ledger

[nodes.jsonl](nodes.jsonl) maps all 193 source files in the frozen [P00 node inventory](../compatibility/nodes.jsonl), pinned to reference commit `8d2b672aff65a1a4dbf7936cec27d19f51989412`. This is an editable progress ledger; the frozen inventory remains untouched.

- `model_status`: `foundation` means a Rust representation exists (possibly only the subset described in `note`); `pending` means it will be added with its grammar phase; `deferred` means outside the PHP 7.0–8.4 ceiling.
- `rust_model`: a representation in `src/ast.rs`, or null while absent. Upstream base classes and legacy aliases can map to the same Rust type; one row does not imply one distinct Rust variant.
- `parser_phase`: the planned owner, not a claim of completeness. Shared callable/type nodes span P08/P09; scalar decoding starts in P06 and AST parsing in P07. Additional normalization/recovery checks continue through P10/P11.
- `parser_status`: `slice` means P07 constructs this node family, with links to executable structure evidence; `pending` means its grammar is not implemented; `deferred` is outside scope. This does not claim complete metadata or diagnostic equivalence.

P07 has **130 foundation mappings with expression/file slice construction, 60 pending models and three deferred models**. Partial families are called out in each note. PPRS-only `Program`, `NodeMeta`, `Origin`, `ByteString`, diagnostic and token types do not add rows to the upstream node inventory.

The [P02 case/channel ledger](../../tests/fixtures/coverage.json) remains the acceptance authority for observable behavior. All 300 complete parser cases are still pending; P06 now passes 296 token channels and records four exact standalone-lexer halt differences. P07 adds 69 exact structure/status channels, eight pinned Unicode recovery differences and 133 focused parser observations. No P07 structure channel remains pending. Whole-file fixtures requiring callable/control-flow or declaration syntax have explicit live ownership corrections to P08/P09; the frozen inventories remain unchanged. Data-type tests and the test-only P03 prototype are not parser implementation coverage.
