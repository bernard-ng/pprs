<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/../reference/bootstrap.php';
require __DIR__ . '/../reference/normalize.php';
$short = in_array('--short-tags', $argv, true);
[$root, , $lock] = bootstrap($short);
$inputs = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
$cases = [];
foreach ($inputs as $case) {
    if ($case['short_open_tag'] !== $short) { continue; }
    $source = hex2bin($case['source']['bytes']);
    $errors = new \PhpParser\ErrorHandler\Collecting();
    $tokens = (new \PhpParser\Lexer\Emulative(\PhpParser\PhpVersion::fromString($case['target'])))->tokenize($source, $errors);
    $normalized = []; $reconstructed = '';
    foreach ($tokens as $token) {
        $text = $token->id === 0 ? '' : $token->text;
        requireThat($token->pos === strlen($reconstructed), 'Token gap: ' . $case['id']);
        $reconstructed .= $text;
        $end = $token->pos + strlen($text);
        $kind = $token->id === 0 ? 'EOF' : ($token->id < 256 ? sprintf('CHAR_%02x', $token->id) : token_name($token->id));
        $normalized[] = ['kind' => $kind, 'text' => bytes($text), 'span' => [$token->pos, $end],
            'start' => coordinates($source, $token->pos), 'end' => coordinates($source, $end)];
    }
    requireThat($reconstructed === $source, 'Lost bytes: ' . $case['id']);
    $case['oracle'] = ['target' => $case['target'], 'tokens' => $normalized,
        'diagnostics' => array_map(fn($error) => diagnostic($error, true, $source), $errors->getErrors())];
    $cases[] = $case;
}
$hashes = [];
foreach (['tools/reference/bootstrap.php','tools/reference/normalize.php','tools/lexer/export.php','tools/lexer/inputs.json'] as $path) {
    $hashes[$path] = hash_file('sha256', "$root/$path");
}
$output = json_encode(['schema_version' => 1, 'reference_commit' => $lock['commit'], 'php_version' => PHP_VERSION,
    'php_int_size' => PHP_INT_SIZE, 'short_open_tag' => $short, 'sources_sha256' => $hashes, 'cases' => $cases],
    JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT) . "\n";
$path = "$root/tests/fixtures/lexer/" . ($short ? 'short-tags' : 'regular') . '.json';
if (in_array('--check', $argv, true)) {
    requireThat(file_get_contents($path) === $output, 'P05 export drift');
    echo "P05 lexical oracle reproduced: " . count($cases) . " cases.\n";
} elseif (in_array('--write', $argv, true)) {
    file_put_contents($path, $output);
    echo "P05 lexical oracle written: " . count($cases) . " cases.\n";
} else { echo $output; }
