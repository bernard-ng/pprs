<?php declare(strict_types=1);
namespace PprsReference;
require __DIR__ . '/../reference/bootstrap.php';
require __DIR__ . '/../reference/normalize.php';
[$root, , $lock] = bootstrap();
$inputs = json_decode(file_get_contents(__DIR__ . '/inputs.json'), true, 512, JSON_THROW_ON_ERROR);
foreach ($inputs as &$case) {
    $raw = hex2bin($case['raw']['bytes']);
    try {
        switch ($case['kind']) {
            case 'quoted': $value = \PhpParser\Node\Scalar\String_::parse($raw); break;
            case 'escapes':
                $mode = $case['mode'];
                $value = $mode === 'nowdoc' ? $raw : ($mode === 'single'
                    ? \PhpParser\Node\Scalar\String_::parse("'" . $raw . "'")
                    : \PhpParser\Node\Scalar\String_::parseEscapeSequences($raw, ['double'=>'"','backtick'=>'`','heredoc'=>null][$mode]));
                break;
            case 'number':
                $tokens=(new \PhpParser\Lexer\Emulative(\PhpParser\PhpVersion::fromString('8.4')))->tokenize('<?php ' . $raw);
                requireThat(count($tokens)===3, 'Expected one numeric token');
                $value=$tokens[1]->id === T_LNUMBER ? \PhpParser\Node\Scalar\Int_::fromString($raw)->value : \PhpParser\Node\Scalar\Float_::parse($raw);
                break;
            case 'offset':
                $value = preg_match('/^(?:0|-?[1-9][0-9]*)$/D', $raw) ? +$raw : $raw;
                if (!is_int($value)) { $value=$raw; }
                break;
            case 'doc':
                $indent=hex2bin($case['indent']['bytes']);
                $source='<?php <<<' . ($case['nowdoc'] ? "'LABEL'" : 'LABEL') . "\n" . $raw . $indent . 'LABEL;';
                $handler=new \PhpParser\ErrorHandler\Collecting();
                $ast=(new \PhpParser\ParserFactory())->createForVersion(\PhpParser\PhpVersion::fromString('8.4'))->parse($source,$handler);
                if ($handler->hasErrors()) { throw $handler->getErrors()[0]; }
                $value=$ast[0]->expr->value;
                break;
            default: throw new \RuntimeException('Unknown literal input');
        }
        $case['expected'] = is_string($value) ? bytes($value) : (is_int($value) ? ['integer'=>(string)$value] : ['float64'=>bin2hex(pack('E',$value))]);
    } catch (\PhpParser\Error $error) {
        $message=$error->getRawMessage();
        $kind=str_contains($message,'codepoint') ? 'unicode' : (str_contains($message,'tabs and spaces') ? 'mixed-indent' : (str_contains($message,'indentation level') ? 'short-indent' : (str_contains($message,'numeric literal') ? 'octal' : null)));
        requireThat($kind !== null, 'Unmapped literal error: ' . $message);
        $case['expected']=['error'=>$kind];
    }
}
unset($case);
$hashes=[];
foreach (['tools/reference/bootstrap.php','tools/reference/normalize.php','tools/literal/export.php','tools/literal/inputs.json'] as $path) { $hashes[$path]=hash_file('sha256',"$root/$path"); }
$output=json_encode(['schema_version'=>1,'reference_commit'=>$lock['commit'],'php_version'=>PHP_VERSION,'php_int_size'=>PHP_INT_SIZE,'sources_sha256'=>$hashes,'cases'=>$inputs],JSON_THROW_ON_ERROR|JSON_UNESCAPED_SLASHES|JSON_PRETTY_PRINT)."\n";
$path="$root/tests/fixtures/literal/values.json";
if (in_array('--check',$argv,true)) {requireThat(file_get_contents($path)===$output,'P06 literal export drift');echo 'P06 literal oracle reproduced: '.count($inputs)." cases.\n";}
elseif(in_array('--write',$argv,true)) {file_put_contents($path,$output);echo 'P06 literal oracle written: '.count($inputs)." cases.\n";}
else {echo $output;}
