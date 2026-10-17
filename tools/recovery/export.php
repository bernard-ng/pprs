<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/../reference/bootstrap.php';
require __DIR__ . '/../reference/normalize.php';
[$root, $reference, $lock] = bootstrap();
requireThat(ini_get('zend.assertions') === '1', 'P11 export requires zend.assertions=1');
$inputs = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
$cases = [];
foreach ($inputs as $input) {
    try {
        $observed = observe(hex2bin($input['source']['bytes']), $input['target']);
    } catch (\AssertionError $error) {
        requireThat(in_array($input['id'], ['halt-missing-close', 'halt-missing-semicolon'], true)
            && $error->getMessage() === 'assert($prevToken->id == \T_CLOSE_TAG)',
            'unexpected reference assertion: ' . $input['id'] . ': ' . $error->getMessage());
        $cases[] = $input + ['oracle_failure' => [
            'class' => get_class($error), 'message' => $error->getMessage()]];
        continue;
    }
    $cases[] = $input + ['expected' => array_intersect_key(
        $observed, array_flip(['target', 'status', 'diagnostics']))];
}
$hashes = [];
foreach (['tools/reference/bootstrap.php', 'tools/reference/normalize.php',
          'tools/recovery/export.php', 'tools/recovery/inputs.json'] as $path) {
    $hashes[$path] = hash_file('sha256', "$root/$path");
}
$output = json_encode([
    'schema_version' => 1, 'reference_commit' => $lock['commit'],
    'php_version' => PHP_VERSION, 'php_int_size' => PHP_INT_SIZE,
    'sources_sha256' => $hashes, 'cases' => $cases,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT) . "\n";
$path = "$root/tests/fixtures/recovery/recovery.json";
if (in_array('--check', $argv, true)) {
    requireThat(file_get_contents($path) === $output, 'P11 recovery export drift');
    echo 'P11 recovery oracle reproduced: ' . count($cases) . " cases.\n";
} elseif (in_array('--write', $argv, true)) {
    file_put_contents($path, $output);
    echo 'P11 recovery oracle written: ' . count($cases) . " cases.\n";
} else {
    echo $output;
}
