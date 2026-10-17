# Parser strategy test cases

Seventeen project-authored inputs exercise the selected lexer/parser strategy. Definitions and frozen upstream production links are in `tools/strategy/inputs.json`; `cases.json` retains the byte-encoded inputs and reference-schema oracle observations. The reference commit, runtime version and input-file hash are recorded. The one known malformed-halt assertion is explicit, not an omitted case.

These are architecture probes. The 313-case reference corpus, comparison states and coverage report remain unchanged. No passing prototype case counts as implemented PHP support.

Run `cargo test --locked --test strategy` without PHP or `.references`. The suite checks clean projected reference shapes, explicit malformed-input recovery, complete byte ranges, interpolation/heredoc boundaries, all 495 input prefixes, arbitrary byte values, reference-marker trivia and depth-limit termination. The projection intentionally treats literals/templates/names as atoms; the production parser and lexer suites check full AST values and tokenization.

Regenerate only with the pinned oracle profile:

```sh
php -n -d short_open_tag=0 -d zend.assertions=1 tools/strategy/export.php --write
php -n -d short_open_tag=0 -d zend.assertions=1 tools/strategy/export.php --check
```

See [decision 0002](../../../docs/decisions/0002-parser-strategy.md) for the comparison, measured costs, limitations and production interfaces. Oracle-derived material is covered by [the upstream notice](../../../reference/LICENSE.nikic-PHP-Parser).
