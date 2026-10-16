<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/../reference/bootstrap.php';
require __DIR__ . '/../reference/normalize.php';
[$root, , $lock] = bootstrap();
$inputs = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
$cases = [];
foreach ($inputs as $input) {
    $source = hex2bin($input['source']['bytes']);
    $observed = observe($source, $input['target']);
    $cases[] = $input + ['expected' => array_intersect_key($observed, array_flip(['target', 'status', 'ast']))];
}
$hashes = [];
foreach (['tools/reference/bootstrap.php', 'tools/reference/normalize.php', 'tools/statements/export.php', 'tools/statements/inputs.json'] as $path) {
    $hashes[$path] = hash_file('sha256', "$root/$path");
}
$output = json_encode(['schema_version'=>1, 'reference_commit'=>$lock['commit'], 'php_version'=>PHP_VERSION, 'php_int_size'=>PHP_INT_SIZE, 'sources_sha256'=>$hashes, 'cases'=>$cases], JSON_THROW_ON_ERROR|JSON_UNESCAPED_SLASHES|JSON_PRETTY_PRINT)."\n";
$path = "$root/tests/fixtures/statements/statements.json";
if (in_array('--check', $argv, true)) {
    requireThat(file_get_contents($path) === $output, 'P08 statement export drift');
    echo 'P08 statement oracle reproduced: '.count($cases)." cases.\n";
} elseif (in_array('--write', $argv, true)) {
    file_put_contents($path, $output);
    echo 'P08 statement oracle written: '.count($cases)." cases.\n";
} else { echo $output; }
