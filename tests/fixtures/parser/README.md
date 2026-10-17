# Expression grammar oracle

`expressions.json` contains 133 project-authored probes exported by the frozen PHP-Parser: 100 ambiguous-expression cases (ten binding combinations across ten target versions) and 33 file/expression/context probes. Inputs and reasons are in [`tools/parser/inputs.json`](../../../tools/parser/inputs.json). Focused slices exercise expressions from complete upstream files that also contain statements, callables or declarations.

Expected values retain the normalized node/field schema of the reference corpus, tagged bytes, signed integer strings, binary64 bits and completion status. Blocks remain explicit `Stmt_Block` nodes. The exporter calls the actual reference parser without evaluating PHP. It records the reference commit, 64-bit PHP 8.5.6 and hashes of the inputs/exporter/helpers. Rust tests independently project the production AST, verify provenance and require every observation to match exactly. Invalid-grammar robustness and recovery-policy tests are separate; this oracle does not claim general malformed-input tree equivalence.

```sh
php -n -d short_open_tag=0 tools/parser/export.php --check
```

Use `--write` only for reviewed regeneration. `.references` and the pinned runtime are needed only to regenerate, not to run normal tests. The reference corpus structure promotions and the eight exact Unicode-recovery differences remain in [`coverage.json`](../coverage.json); the [source fidelity](../fidelity/README.md) and [recovery](../recovery/README.md) suites check metadata and diagnostics. See [decision 0006](../../../docs/decisions/0006-expression-parser.md).
