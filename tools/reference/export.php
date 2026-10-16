<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/bootstrap.php';
require __DIR__ . '/normalize.php';

[$root, $reference, $lock] = bootstrap();
$fixtureParser = new \PhpParser\CodeTestParser();
$inventory = array_map(fn($line) => json_decode($line, true, 512, JSON_THROW_ON_ERROR),
    file("$root/docs/compatibility/fixtures.jsonl", FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES));
$cases = []; $parsedFiles = [];
foreach ($inventory as $entry) {
    if (!str_starts_with($entry['id'], 'parser/')) { continue; }
    $path = $entry['path'];
    if (!isset($parsedFiles[$path])) {
        $parsedFiles[$path] = $fixtureParser->parseTest(file_get_contents("$reference/$path"), 2);
    }
    [$title, $parts] = $parsedFiles[$path];
    [$mode, [$source, $expected]] = $parts[$entry['case_index']];
    requireThat($mode === $entry['mode'], "Mode differs: {$entry['id']}");
    $modes = [];
    foreach ($mode === null ? [] : explode(',', $mode) as $item) {
        $pair = explode('=', $item, 2); $modes[$pair[0]] = $pair[1] ?? true;
    }
    $originalTarget = $modes['version'] ?? $lock['reference_newest_target'];
    $original = observe($source, $originalTarget, $modes);
    requireThat($original['dump'] === bytes($expected), "Original dump differs: {$entry['id']}");
    $target = $entry['scope'] === 'core' ? ($modes['version'] ?? $lock['default_target']) : null;
    $cases[$entry['id']] = [
        'schema_version' => 1, 'id' => $entry['id'], 'title' => $title,
        'origin' => ['path' => $path, 'case_index' => $entry['case_index'],
            'file_sha256' => hash_file('sha256', "$reference/$path"),
            'raw_section_sha256' => $entry['raw_section_sha256'], 'commit' => $lock['commit']],
        'mode' => $mode, 'modes' => (object) $modes, 'source' => bytes($source),
        'source_sha256' => hash('sha256', $source), 'original_expected' => bytes($expected),
        'original_target' => $originalTarget, 'original_dump_verified' => true,
        'scope' => $entry['scope'], 'features' => $entry['features'], 'phase' => $entry['phase'],
        'reason' => $entry['reason'], 'derivation' => null,
        'oracle' => $target === null ? $original : observe($source, $target, $modes),
        'comparison_target' => $target,
    ];
}
requireThat(count($cases) === 290, 'Parser case count drift');
foreach ($parsedFiles as $path => [$title, $parts]) {
    $entries = array_filter($cases, fn($case) => $case['origin']['path'] === $path);
    requireThat(count($parts) === count($entries), "Expanded section count differs: $path");
}
$extra = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
foreach ($extra as $input) {
    $derivation = $input['derivation'] ?? null;
    if ($derivation !== null) {
        $parent = $cases[$derivation['parent']];
        $parentSource = hex2bin($parent['source']['bytes']);
        requireThat(hash('sha256', $parentSource) === $derivation['source_sha256'], 'Derivation parent drift');
        $source = '';
        foreach ($derivation['segments'] as [$start, $end]) {
            span($start, $end, strlen($parentSource));
            $source .= substr($parentSource, $start, $end - $start);
        }
    } else { $source = hex2bin($input['source']['bytes']); }
    requireThat(!isset($cases[$input['id']]), 'Duplicate input ID');
    $cases[$input['id']] = [
        'schema_version' => 1, 'id' => $input['id'], 'title' => $input['title'],
        'origin' => null, 'mode' => null, 'modes' => (object) [], 'source' => bytes($source),
        'source_sha256' => hash('sha256', $source), 'original_expected' => null,
        'original_target' => null, 'original_dump_verified' => null,
        'scope' => 'core', 'features' => $input['features'], 'phase' => $input['phase'],
        'reason' => $input['reason'], 'derivation' => $derivation,
        'oracle' => observe($source, $input['target']), 'comparison_target' => $input['target'],
    ];
}
ksort($cases);
$profile = ['php_version' => PHP_VERSION, 'php_int_size' => PHP_INT_SIZE,
    'tokenizer' => extension_loaded('tokenizer'), 'json' => extension_loaded('json'),
    'short_open_tag' => (bool) ini_get('short_open_tag'), 'locale' => setlocale(LC_ALL, '0'),
    'timezone' => date_default_timezone_get(), 'ini' => 'disabled (-n)',
    'precision' => ini_get('precision'), 'serialize_precision' => ini_get('serialize_precision')];
echo json_encode(['schema_version' => 1, 'profile' => $profile, 'cases' => array_values($cases)],
    JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES), "\n";
