# P00 observations

These are narrowly scoped source inspections/probes, not a PHP-Parser test-suite run.

## Lone CR line handling

Host: PHP CLI 8.5.6, 64-bit, inspected on 2026-10-16. The built-in tokenizer was called directly; no program source was executed.

```php
$source = "<?php\recho 1;\recho 2;";
foreach (PhpToken::tokenize($source) as $token) {
    echo json_encode([
        token_name($token->id), $token->text, $token->line, $token->pos
    ]), PHP_EOL;
}
```

The first `echo` token starts at byte 6, line 2; the second at byte 14, line 3. Native tokenization therefore recognizes lone CR breaks in this probe.

In contrast, pinned `lib/PhpParser/Token.php::getEndLine()` adds `substr_count(text, "\n")`, and `lib/PhpParser/Error.php::toColumn()` locates the preceding LF. Those helpers cannot define a single consistent CR-only source map. [D03](../decisions/0001-parser-behavior.md#d03--bytes-positions-and-source-preservation) chooses consistent source-derived lines/byte columns and preserves raw oracle attributes separately.

## Reference permissiveness is not language-version validation

Source inspection establishes that the PHP7.4-mode case `parser/expr/partialFunctionApplication.test#1` expects argument-placeholder nodes from experimental PHP8.6 syntax. Similarly, `parser/expr/argumentList.test#1` combines ordinary call forms with PHP8.5 clone-call forms under PHP7.4 mode. These cases are respectively deferred and mixed in the inventory.

The grammar declares ternary with `%left '?' ':'` outside the PHP7/PHP8 conditional blocks. The inspected parser helper contains no non-associative-ternary validation. P00 therefore makes no promise of enforcing PHP8 engine ternary validity through nikic-compatible parsing.
