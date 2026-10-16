# P07 expression grammar oracle

`expressions.json` contains 133 project-authored probes exported by the frozen PHP-Parser: 100 ambiguous-expression cases (ten binding combinations across ten target versions) and 33 file/expression/context probes. Inputs and reasons are in [`tools/parser/inputs.json`](../../../tools/parser/inputs.json). Focused slices exercise expressions from complete upstream files that also need P08/P09 syntax.

Expected values retain the P02 normalized node/field schema, tagged bytes, signed integer strings, binary64 bits and completion status. Blocks remain explicit `Stmt_Block` nodes. The exporter calls the actual reference parser without evaluating PHP. It records the reference commit, 64-bit PHP 8.5.6 and hashes of the inputs/exporter/helpers. Rust tests independently project the production AST, verify provenance and require every observation to match exactly. Invalid-grammar robustness and recovery-policy tests are separate; this oracle does not claim general malformed-input tree equivalence.

```sh
php -n -d short_open_tag=0 tools/parser/export.php --check
```

Use `--write` only for reviewed regeneration. `.references` and the pinned runtime are needed only to regenerate, not to run normal tests. P02 structure promotions and the eight exact Unicode-recovery differences remain in [`coverage.json`](../coverage.json); metadata/diagnostic parity belongs to P10/P11. See [decision 0006](../../../docs/decisions/0006-expression-parser.md).
