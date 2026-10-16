<?php declare(strict_types=1);

// canonicalize() adapted verbatim from the pinned test/bootstrap.php, except
// for types. Copyright (c) 2011 Nikita Popov; see reference/LICENSE.nikic-PHP-Parser.
namespace PhpParser {
    function canonicalize($str) {
        $str = str_replace("\r\n", "\n", $str);
        $str = rtrim($str, "\n");
        $lines = explode("\n", $str);
        $lines = array_map(function ($line) { return rtrim($line, " \t"); }, $lines);
        return implode("\n", $lines);
    }
}

namespace PprsReference {
    function requireThat(bool $condition, string $message): void {
        if (!$condition) { throw new \RuntimeException($message); }
    }

    function bootstrap(bool $shortOpenTag = false): array {
        $root = dirname(__DIR__, 2);
        $lock = json_decode(file_get_contents("$root/reference/lock.json"), true, 512, JSON_THROW_ON_ERROR);
        $reference = "$root/.references";
        // Verify every executable input BEFORE loading upstream PHP or evaluating fixture macros.
        $manifest = file_get_contents("$root/reference/files.sha256");
        requireThat(hash('sha256', $manifest) === $lock['file_manifest_sha256'], 'Reference manifest drift');
        foreach (explode("\n", rtrim($manifest, "\n")) as $line) {
            [$hash, $path] = explode('  ', $line, 2);
            requireThat(hash_file('sha256', "$reference/$path") === $hash, "Reference drift: $path");
        }
        requireThat(PHP_VERSION === '8.5.6' && PHP_INT_SIZE === 8, 'Oracle requires 64-bit PHP 8.5.6');
        requireThat(extension_loaded('tokenizer') && extension_loaded('json'), 'Missing tokenizer/json');
        requireThat((bool) ini_get('short_open_tag') === $shortOpenTag, 'short_open_tag profile mismatch');
        requireThat(php_ini_loaded_file() === false && !php_ini_scanned_files(), 'Run PHP with -n');
        requireThat(setlocale(LC_ALL, 'C') !== false, 'C locale unavailable');
        date_default_timezone_set('UTC');
        spl_autoload_register(function (string $class) use ($reference): void {
            $prefix = 'PhpParser\\';
            if (strncmp($class, $prefix, strlen($prefix)) === 0) {
                $relative = str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
                foreach (['lib/PhpParser', 'test/PhpParser'] as $base) {
                    $path = "$reference/$base/$relative";
                    if (is_file($path)) { require $path; return; }
                }
            }
        });
        return [$root, $reference, $lock];
    }
}
