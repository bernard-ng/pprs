# P09 declaration oracle

`declarations.json` contains 60 exact semantic AST/status observations from frozen PHP-Parser commit `8d2b672aff65a1a4dbf7936cec27d19f51989412`, PHP 8.5.6 and 64-bit integers. The exporter records hashes of its inputs, source, bootstrap and normalizer. Thirty observations cover all ten targets for declaration types/modifiers, namespaces/imports and attributes; thirty cover class/member/trait/enum/hook grammar, parser checks and recovery. Target selection follows reference parser interpretation, not strict historical compiler validation.

The inputs are project-authored. `tests/declarations.rs` verifies exact observations and provenance offline, then checks every byte truncation, small resource budgets, arbitrary declaration tokens, deep nesting, malformed-member recovery, visitor ordering and namespace ranges. Existing P07/P08 oracles remain unchanged. The separate upstream ledger promotes 96 P09 structure channels and pins five recovery differences; no P09 structure obligation remains pending.

```sh
php -n -d short_open_tag=0 tools/declarations/export.php --write
php -n -d short_open_tag=0 tools/declarations/export.php --check
cargo test --locked --test declarations
```

Regeneration requires the pinned runtime and verified reference checkout. Rust CI requires neither PHP nor `.references`. Metadata/diagnostic channel parity is not claimed here. PHP 8.5 constant attributes and final promotion remain unsupported and have negative Rust tests. See [production ownership](../../../docs/implementation/declarations.md), [behavior decision](../../../docs/decisions/0008-declaration-parser.md), [notice](../../../THIRD_PARTY_NOTICES.md), and [BSD license](../../../reference/LICENSE.nikic-PHP-Parser).
