# Literal oracle

`values.json` records 139 project-authored observations from [inputs.json](../../../tools/literal/inputs.json). The exporter uses the frozen PHP-Parser `String_`, `Int_` and `Float_` utilities; complete doc-string cases use its parser. It never evaluates PHP source. Simple numeric-offset observations implement the exact `ParserAbstract::parseNumString` predicate, retaining overflowing/noncanonical strings.

Values use the reference corpus byte/integer/binary64 encodings. Errors record a stable test classification, not a requirement to duplicate message wording. Fixtures pin the reference commit, PHP 8.5.6 64-bit profile and helper/input hashes. Rust tests check provenance, every value/error, input-relative error ranges, invalid utility arguments and arbitrary-byte bounds without requiring PHP.

Reproduce with the verified reference and pinned runtime:

```sh
php -n -d short_open_tag=0 tools/literal/export.php --check
```

Use `--write` only for reviewed updates. These are literal-value checks, not scalar AST/parser coverage; the expression oracle checks AST parity.
