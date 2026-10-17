# Diagnostics and recovery observations

`recovery.json` preserves 87 focused observations, with source bytes, targets, reasons, frozen-reference identity, PHP 8.5.6/64-bit host metadata and exporter/input hashes. There are 85 successful oracle observations and two explicit reference assertion failures. Ordinary Rust tests need neither PHP nor `.references`.

`probe-review.json` records 57 exact diagnostic matches, 28 intentional differences and the two reference failures. Each row checks native codes, severity, primary spans, wording, expected/found tokens and secondary labels. Exact alternative observations preserve order, ranges and coordinates. Reference assertion failures are not counted as parity; independent tests require a non-clean Rust result and preservation of the following echo statement.

`core-review.json` covers all 112 invalid core fixtures, including those outside `errorHandling`. Each row states the reviewed behavior and freezes the native diagnostic contract. The main coverage ledger compares all 300 core diagnostic channels: 239 exact matches and 61 explicit differences. AST/status and metadata comparisons remain independently active. There are no pending core channels.

The normalization adapter maps only the documented native code families to oracle codes. It does not discard messages, reorder errors, strip spans or force statuses to match. Only `raw_attributes`, the pre-existing provenance field, is excluded from comparison. See [decision 0010](../../../docs/decisions/0010-diagnostics-and-recovery.md) and the [recovery ledger](../../../docs/implementation/recovery.md).

Reproduce using the frozen checkout and assertions enabled:

```sh
php -n -d short_open_tag=0 -d zend.assertions=1 tools/recovery/export.php --check
cargo test --locked --test recovery
```

Use `--write` for a deliberate oracle regeneration. The exporter admits only the two named halt failures and their exact assertion text; any new reference failure stops export. Reviewed Rust expectations are separate and are never written by the oracle exporter.

An explicit development audit writes actual observations without modifying expectations:

```sh
PPRS_RECOVERY_REPORT=/tmp/pprs-recovery.json cargo test --locked --test recovery export_recovery_audit -- --ignored
```

The suite checks strict/recovering equivalence, repeated-call determinism, every focused byte prefix, small budgets, error storms, diagnostic comparison mutations, related locations and survival of following statements/declarations/members. Existing all-core/source/AST invariants and prior randomized regressions continue to run. Instrumented fuzz campaigns are recorded separately in the [robustness report](../../../docs/benchmarks/robustness-performance.md).
