# Frozen PHP-Parser reference

P00 baseline, frozen on 2026-10-16. This directory records provenance and integrity; `.references/` remains an ignored, separate checkout.

| Property | Frozen value |
|---|---|
| Upstream | [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) |
| Commit | `8d2b672aff65a1a4dbf7936cec27d19f51989412` |
| Description | `v5.9.0-7-g8d2b672a` |
| Checkout at freeze | Clean; original branch/checkout was not changed |
| Contents | 668 tracked files, SHA-256 recorded in [files.sha256](files.sha256) |
| License | [BSD-3-Clause notice copied verbatim](LICENSE.nikic-PHP-Parser) |
| Project targets | PHP 7.0–7.4 and 8.0–8.4; default 8.4 |
| Reference scope warning | README says 8.4; newest target is 8.5; grammar also has experimental 8.6 partial application |

[lock.json](lock.json) records the Git commit/tree, observed remote, file-manifest hash, counts and P00 inventory hashes. The public upstream URL is canonicalized; the original remote's capitalization is retained separately.

## Verification

From the project root, using Python 3.9 or later and Git:

```sh
python3 tools/reference/verify_p00.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/reference -p 'test_*.py' -v
```

The verifier checks the pin, tree, clean status, tracked-file set, all file hashes, license copy, inventory checksums, feature/production mappings, node/unit-test/fixture completeness, case IDs, raw fixture-section hashes, modes and counts. It does not execute PHP, expand fixture macros, run the upstream suite, fetch anything, change Git state, or rewrite metadata.

For a checkout without `.references`, validate committed metadata alone:

```sh
python3 tools/reference/verify_p00.py --metadata-only
```

This cannot certify the absent reference bytes. Ordinary Rust tests use the [P02 exported fixtures](../tests/fixtures/README.md) and do not depend on this checkout or Python.

## Recreating the ignored checkout

When `.references` does not exist, these commands create a separate checkout at the recorded revision:

```sh
git clone --no-checkout https://github.com/nikic/PHP-Parser.git .references
git -C .references checkout --detach 8d2b672aff65a1a4dbf7936cec27d19f51989412
python3 tools/reference/verify_p00.py
```

Do not reset an existing modified checkout to reproduce the baseline. Inspect it or create another directory and pass `--reference /path/to/checkout` to the verifier. Provisioned ignored dependencies are not part of the 668 tracked-file hash set; non-ignored extra files make the clean-check fail.

## Baseline changes

P00 inventories are a frozen scope snapshot, not a live test-pass dashboard. Every implementation status remains pending/not implemented at this point. P02 maintains [conformance states](../tests/fixtures/coverage.json) separately; all parser cases remain pending. The lock's `oracle_execution` field is historical P00 state; the P02 corpus manifest records the executed oracle profile.

An intentional reference update must review upstream grammar, AST, lexer and fixture changes, update the scope classifications, recompute the file manifest and corresponding lock hashes, regenerate affected inventories, and run verification. Inventory-only corrections require a reviewable metadata/hash change but must not silently change the upstream commit. Do not “fix” a verification failure by accepting new hashes without investigating the difference.

The fixture inventory records **raw section bytes before canonicalization and macro evaluation**. These hashes identify source material; they are not expanded PHP source or oracle outputs. See the [compatibility matrix](../docs/compatibility/README.md) and [test strategy](../plans/reference-and-testing-strategy.md).
