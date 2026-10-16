<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/bootstrap.php';
require __DIR__ . '/normalize.php';
bootstrap();

function same($expected, $actual): void {
    requireThat($expected === $actual, 'Expected ' . var_export($expected, true) . ', got ' . var_export($actual, true));
}
$parser = new \PhpParser\CodeTestParser();
// Static trusted macro proves cleanup occurs BEFORE expansion, and modes and
// multiple chunks survive. Expanded NUL/non-UTF8/trailing whitespace must remain.
$fixture = "Title  \r\n-----\r\n<?php @@{chr(255) . chr(0) . chr(13) . chr(10) . '  '}@@\r\n-----\r\n!!version=7.4,positions\r\nexpected  \r\n-----\r\nsecond\r\n-----\r\n!!attributes\r\nresult\r\n\r\n";
[$title, $cases] = $parser->parseTest($fixture, 2);
same('Title', $title);
same(2, count($cases));
same('version=7.4,positions', $cases[0][0]);
same("<?php \xff\0\r\n  ", $cases[0][1][0]);
same('expected', $cases[0][1][1]);
same(['attributes', ['second', 'result']], $cases[1]);
same(['bytes' => 'ff00'], scalar("\xff\0"));
same(['integer' => '9223372036854775807'], scalar(PHP_INT_MAX));
same(['float64' => '7ff0000000000000'], scalar(INF));
same(['float64' => 'fff0000000000000'], scalar(-INF));
same(['float64' => '8000000000000000'], scalar(-0.0));
requireThat(is_nan(unpack('E', hex2bin(scalar(NAN)['float64']))[1]), 'NaN lost');
same([1, 3], coordinates("a\r\nb\rc\nd", 2));
same([2, 1], coordinates("a\r\nb\rc\nd", 3));
same([3, 1], coordinates("a\r\nb\rc\nd", 5));
$empty = origin(['startFilePos' => 2, 'endFilePos' => 1, 'startTokenPos' => 1, 'endTokenPos' => 0], 'abc', 2);
same([2, 2], $empty['span']); same(true, $empty['empty']);
try { origin([], '', 1); throw new \LogicException('Missing origin accepted'); }
catch (\RuntimeException $error) { same('Missing origin startFilePos', $error->getMessage()); }
$comment = observe("<?php /* raw\xff", '8.4');
same([6, 13], $comment['diagnostics'][0]['span']);
same('lex.unterminated-comment', $comment['diagnostics'][0]['code']);
same([13, 13], $comment['tokens'][count($comment['tokens']) - 1]['span']);
$bad = observe("<?php \0\1;", '8.4');
same('lex.null-byte', $bad['diagnostics'][0]['code']);
same([6, 7], $bad['diagnostics'][0]['span']);
same('lex.bad-character', $bad['diagnostics'][1]['code']);
$eof = observe('<?php function f(', '8.4');
same([17, 17], $eof['diagnostics'][0]['span']);
$literal = observe('<?php /** doc */ "x";', '8.4');
same('Stmt_Expression', $literal['ast'][0]['kind']);
same('Scalar_String', $literal['ast'][0]['fields']->expr['kind']);
same('doc', $literal['metadata'][0]['comments'][0]['kind']);
same(['bytes' => '78'], $literal['ast'][0]['fields']->expr['fields']->value);
fwrite(STDOUT, "Importer and normalization assertions passed.\n");
