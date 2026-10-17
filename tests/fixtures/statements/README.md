# Statement and callable oracle

`statements.json` stores 49 exact semantic AST/status observations from the frozen reference and PHP 8.5.6 (64-bit). It includes provenance hashes for the exporter, input definitions and shared bootstrap/normalizer. All ten PHP targets are covered for builtin classification, composite signatures and callable forms; pre-7.4 callable probes omit arrow syntax because the reference demotes `fn` there.

Inputs are project-authored in `tools/statements/inputs.json`. Tests compare the production Rust parser via the shared AST projector, with no oracle-guided parser behavior. They also check truncations, resource budgets, recovery, invalid syntax, deep nesting and traversal. The [source fidelity](../fidelity/README.md) and [recovery](../recovery/README.md) suites check metadata and diagnostics.

Explicitly regenerate or verify using the pinned PHP runtime and verified checkout:

```sh
php -n -d short_open_tag=0 tools/statements/export.php --write
php -n -d short_open_tag=0 tools/statements/export.php --check
cargo test --locked --test statements
```

Reference-derived observations retain the notice in [THIRD_PARTY_NOTICES.md](../../../THIRD_PARTY_NOTICES.md) and [BSD license](../../../reference/LICENSE.nikic-PHP-Parser). Upstream complete-file cases and the two statement-parser recovery differences remain tracked separately in `coverage.json`.
