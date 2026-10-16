<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/../reference/bootstrap.php';
require __DIR__ . '/../reference/normalize.php';
[$root, , $lock] = bootstrap();
requireThat(ini_get('zend.assertions') === '1', 'P03 requires -d zend.assertions=1');
$cases = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
foreach ($cases as &$case) {
    try { $case['oracle'] = observe(hex2bin($case['source']['bytes']), $case['target']); }
    catch (\AssertionError $error) {
        requireThat($case['id'] === 'halt-invalid', 'Unexpected oracle assertion');
        $case['oracle_failure'] = $error->getMessage();
        continue;
    }
    requireThat(($case['oracle']['status'] !== 'clean') === $case['errors'], 'Unexpected oracle status: ' . $case['id']);
}
unset($case);
$output = json_encode(['schema_version' => 1, 'reference_commit' => $lock['commit'],
    'php_version' => PHP_VERSION, 'php_int_size' => PHP_INT_SIZE, 'zend_assertions' => 1,
    'inputs_sha256' => hash_file('sha256', __DIR__ . '/inputs.json'),
    'cases' => $cases], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT) . "\n";
$path = "$root/tests/fixtures/strategy/cases.json";
if (($argv[1] ?? null) === "--check") {
    requireThat(file_get_contents($path) === $output, "P03 oracle snapshot differs");
    echo "P03 oracle snapshot reproduced.\n";
} elseif (($argv[1] ?? null) === "--write") {
    file_put_contents($path, $output);
    echo "P03 oracle snapshot written.\n";
} else { echo $output; }
