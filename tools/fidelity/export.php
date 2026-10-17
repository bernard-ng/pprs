<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/../reference/bootstrap.php';
require __DIR__ . '/../reference/normalize.php';
[$root, $reference, $lock] = bootstrap();
$inputs = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
$cases = [];
foreach ($inputs as $input) {
    $source = hex2bin($input['source']['bytes']);
    $observed = observe($source, $input['target']);
    $cases[] = $input + ['expected' => array_intersect_key(
        $observed, array_flip(['target', 'status', 'ast', 'metadata']))];
}

// The node API defines preorder independently of any fixture's expected tree.
$fields = [];
$directory = "$reference/lib/PhpParser/Node/";
foreach (new \RecursiveIteratorIterator(new \RecursiveDirectoryIterator($directory)) as $file) {
    if ($file->getExtension() !== 'php') {
        continue;
    }
    $name = 'PhpParser\\Node\\' . str_replace(
        ['/', '.php'], ['\\', ''], substr($file->getPathname(), strlen($directory)));
    $reflection = new \ReflectionClass($name);
    if ($reflection->isAbstract() || $reflection->isInterface()) {
        continue;
    }
    $node = $reflection->newInstanceWithoutConstructor();
    $fields[$node->getType()] = $node->getSubNodeNames();
}
ksort($fields);
$fieldOutput = json_encode($fields, JSON_THROW_ON_ERROR | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) . "\n";
$fieldPath = "$root/tests/fixtures/fidelity/fields.json";
$hashes = [];
foreach (['tools/reference/bootstrap.php', 'tools/reference/normalize.php',
          'tools/fidelity/export.php', 'tools/fidelity/inputs.json'] as $path) {
    $hashes[$path] = hash_file('sha256', "$root/$path");
}
$hashes['tests/fixtures/fidelity/fields.json'] = hash('sha256', $fieldOutput);
$output = json_encode([
    'schema_version' => 1, 'reference_commit' => $lock['commit'],
    'php_version' => PHP_VERSION, 'php_int_size' => PHP_INT_SIZE,
    'sources_sha256' => $hashes, 'cases' => $cases,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT) . "\n";
$path = "$root/tests/fixtures/fidelity/fidelity.json";
if (in_array('--check', $argv, true)) {
    requireThat(file_get_contents($fieldPath) === $fieldOutput, 'node field schema drift');
    requireThat(file_get_contents($path) === $output, 'P10 fidelity export drift');
    echo 'P10 fidelity oracle and node schema reproduced: ' . count($cases) . " cases.\n";
} elseif (in_array('--write', $argv, true)) {
    file_put_contents($fieldPath, $fieldOutput);
    file_put_contents($path, $output);
    echo 'P10 fidelity oracle and node schema written: ' . count($cases) . " cases.\n";
} else {
    echo $output;
}
