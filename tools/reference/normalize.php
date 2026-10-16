<?php declare(strict_types=1);
namespace PprsReference;

use PhpParser\Comment;
use PhpParser\Error;
use PhpParser\ErrorHandler\Collecting;
use PhpParser\Lexer\Emulative;
use PhpParser\Node;
use PhpParser\NodeDumper;
use PhpParser\ParserFactory;
use PhpParser\PhpVersion;

function bytes(string $value): array { return ['bytes' => bin2hex($value)]; }

// Semantic scalar values never depend on JSON integer precision or UTF-8 decoding.
function scalar($value) {
    if (is_string($value)) { return bytes($value); }
    if (is_int($value)) { return ['integer' => (string) $value]; }
    if (is_float($value)) { return ['float64' => bin2hex(pack('E', $value))]; }
    if (is_array($value)) { return array_map(__NAMESPACE__ . '\\scalar', $value); }
    requireThat($value === null || is_bool($value), 'Unexpected scalar type');
    return $value;
}

function coordinates(string $source, int $position): array {
    requireThat($position >= 0 && $position <= strlen($source), 'Coordinate out of bounds');
    $line = 1; $start = 0;
    for ($i = 0; $i < $position; $i++) {
        if ($source[$i] === "\r" && ($source[$i + 1] ?? '') === "\n") {
            if ($i + 1 >= $position) { break; }
            $i++;
            $line++; $start = $i + 1;
        } elseif ($source[$i] === "\r" || $source[$i] === "\n") {
            $line++; $start = $i + 1;
        }
    }
    return [$line, $position - $start + 1];
}

function span(int $start, int $end, int $length): array {
    requireThat($start >= 0 && $start <= $end && $end <= $length, "Invalid span $start..$end / $length");
    return [$start, $end];
}

function origin(array $attributes, string $source, int $tokenCount): array {
    foreach (['startFilePos', 'endFilePos', 'startTokenPos', 'endTokenPos'] as $key) {
        requireThat(isset($attributes[$key]), "Missing origin $key");
    }
    $start = $attributes['startFilePos'];
    $end = max($start, $attributes['endFilePos'] + 1);
    $tokenStart = $attributes['startTokenPos'];
    $tokenEnd = max($tokenStart, $attributes['endTokenPos'] + 1);
    return [
        'span' => span($start, $end, strlen($source)),
        'token_span' => span($tokenStart, $tokenEnd, $tokenCount),
        'empty' => $start === $end,
        'start' => coordinates($source, $start), 'end' => coordinates($source, $end),
    ];
}

function comment(Comment $comment, string $source, int $tokenCount): array {
    $raw = [
        'startLine' => $comment->getStartLine(), 'endLine' => $comment->getEndLine(),
        'startFilePos' => $comment->getStartFilePos(), 'endFilePos' => $comment->getEndFilePos(),
        'startTokenPos' => $comment->getStartTokenPos(), 'endTokenPos' => $comment->getEndTokenPos(),
    ];
    return ['kind' => $comment instanceof Comment\Doc ? 'doc' : 'comment',
        'text' => bytes($comment->getText()), 'origin' => origin($raw, $source, $tokenCount),
        'raw_attributes' => $raw];
}

function ast($value, string $path, string $source, int $tokenCount, array &$metadata) {
    if ($value instanceof Node) {
        $attributes = $value->getAttributes();
        $comments = array_map(fn(Comment $c) => comment($c, $source, $tokenCount), $value->getComments());
        $other = $attributes;
        foreach (['startLine', 'endLine', 'startFilePos', 'endFilePos', 'startTokenPos', 'endTokenPos', 'comments'] as $key) {
            unset($other[$key]);
        }
        $raw = $attributes;
        unset($raw['comments']);
        $metadata[] = ['path' => $path, 'origin' => origin($attributes, $source, $tokenCount),
            'comments' => $comments, 'spelling' => (object) scalar($other), 'raw_attributes' => (object) scalar($raw)];
        $fields = [];
        foreach ($value->getSubNodeNames() as $field) {
            $fields[$field] = ast($value->$field, "$path/fields/$field", $source, $tokenCount, $metadata);
        }
        return ['kind' => $value->getType(), 'fields' => (object) $fields];
    }
    if (is_array($value)) {
        $result = [];
        foreach ($value as $key => $child) { $result[$key] = ast($child, "$path/$key", $source, $tokenCount, $metadata); }
        return $result;
    }
    return scalar($value);
}

function diagnostic(Error $error, bool $lexical, string $source): array {
    $raw = $error->getAttributes();
    $message = $error->getRawMessage();
    $category = $lexical ? 'lexical' : (str_starts_with($message, 'Syntax error,') ? 'syntax' : 'contextual');
    $code = "reference.$category";
    if ($lexical) {
        if ($message === 'Unterminated comment') { $code = 'lex.unterminated-comment'; }
        elseif ($message === 'Unexpected null byte') { $code = 'lex.null-byte'; }
        elseif (preg_match('/^Unexpected character "." \(ASCII [0-9]+\)$/sD', $message)) { $code = 'lex.bad-character'; }
        else { throw new \RuntimeException("Unmapped lexical error: $message"); }
    }
    $range = null;
    if (isset($raw['startFilePos'], $raw['endFilePos'])) {
        $start = $raw['startFilePos'];
        $end = $message === 'Unterminated comment' ? $raw['endFilePos'] : $raw['endFilePos'] + 1;
        if ($start === strlen($source)) { $end = $start; }
        $range = span($start, max($start, $end), strlen($source));
    }
    return ['category' => $category, 'code' => $code, 'message' => bytes($message),
        'span' => $range, 'start' => $range === null ? null : coordinates($source, $range[0]),
        'end' => $range === null ? null : coordinates($source, $range[1]), 'raw_attributes' => $raw];
}

function observe(string $source, string $target, array $modes = []): array {
    $version = PhpVersion::fromString($target);
    $lexerErrors = new Collecting();
    (new Emulative($version))->tokenize($source, $lexerErrors);
    $parser = (new ParserFactory())->createForVersion($version);
    $errors = new Collecting();
    $nodes = $parser->parse($source, $errors);
    $rawTokens = $parser->getTokens();
    $tokens = [];
    $reconstructed = '';
    // Include compatibility token constants that token_name() may not recognize.
    $names = [];
    foreach (get_defined_constants() as $name => $id) {
        if (str_starts_with($name, 'T_') && is_int($id)) { $names[$id] = $name; }
    }
    foreach ($rawTokens as $token) {
        $eof = $token->id === 0;
        $text = $eof ? '' : $token->text;
        $start = $token->pos;
        $end = $eof ? $start : $token->getEndPos();
        requireThat($start === strlen($reconstructed), 'Non-contiguous token stream');
        $reconstructed .= $text;
        $kind = $eof ? 'EOF' : ($names[$token->id] ?? sprintf('CHAR_%02x', $token->id));
        $tokens[] = ['kind' => $kind, 'text' => bytes($text), 'span' => span($start, $end, strlen($source)),
            'start' => coordinates($source, $start), 'end' => coordinates($source, $end)];
    }
    requireThat($reconstructed === $source && end($tokens)['kind'] === 'EOF', 'Tokens lost source bytes');
    $diagnostics = []; $dump = '';
    $lexErrors = $lexerErrors->getErrors();
    foreach ($errors->getErrors() as $i => $error) {
        $lexical = $i < count($lexErrors);
        if ($lexical) {
            requireThat($lexErrors[$i]->getRawMessage() === $error->getRawMessage() &&
                $lexErrors[$i]->getAttributes() === $error->getAttributes(), 'Lexer/parser diagnostic order differs');
        }
        $diagnostics[] = diagnostic($error, $lexical, $source);
        $dump .= ($error->hasColumnInfo() ? $error->getMessageWithColumnInfo($source) : $error->getMessage()) . "\n";
    }
    requireThat(count($diagnostics) >= count($lexErrors), 'Dropped lexer diagnostics');
    if ($nodes !== null) {
        $dumper = new NodeDumper(['dumpComments' => true, 'dumpPositions' => isset($modes['positions']),
            'dumpOtherAttributes' => isset($modes['attributes'])]);
        $dump .= $dumper->dump($nodes, $source);
    }
    $metadata = [];
    $structure = ast($nodes, '', $source, count($tokens), $metadata);
    return ['target' => $target, 'status' => $nodes === null ? 'aborted' : ($diagnostics ? 'recovered' : 'clean'),
        'tokens' => $tokens, 'ast' => $structure, 'metadata' => $metadata,
        'diagnostics' => $diagnostics, 'dump' => bytes(\PhpParser\canonicalize($dump))];
}
