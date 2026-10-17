# PPRS

PPRS parses PHP 7.0 through 8.4 into an owned Rust AST. It retains source bytes,
comments and tokens, and reports errors with byte spans. It requires Rust 1.85
or later and has no PHP runtime dependency.

## Parse a file

Pass the file's bytes to `parse_strict`. The caller reads the file. PPRS does no
file or network I/O and never executes PHP.

```no_run
use pprs::{parse_strict, ParseOptions, ParsedFile};

fn read_php(path: &std::path::Path) -> Result<ParsedFile, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    Ok(parse_strict(bytes, ParseOptions::default())?)
}

let file = read_php(std::path::Path::new("src/index.php"))?;
let program = file.program();
println!("{} top-level statements", program.statements.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Parsing starts outside PHP mode. Use an opening tag for PHP code. Untagged input
is inline HTML, even if its text looks like an expression. There is no separate
snippet or expression entry point.

```rust
use pprs::{parse_strict, ast::StmtKind, ParseOptions};

let html = parse_strict("echo 1;", ParseOptions::default())?;
assert!(matches!(html.program().statements[0].kind, StmtKind::InlineHtml(_)));
let php = parse_strict("<?php echo 1;", ParseOptions::default())?;
assert!(matches!(php.program().statements[0].kind, StmtKind::Echo(_)));
# Ok::<(), pprs::StrictError>(())
```

The fixed default target is PHP 8.4. Set `ParseOptions::version` to choose another
supported target. Short `<?` tags require `short_open_tag: true`; `<?=` always
opens PHP mode. Version selection follows nikic/PHP-Parser's best-effort
interpretation. It affects token classification, precedence and AST construction.
It does not reject every newer construct on an older target. Strict success means
no error diagnostics, recovery or resource abort. It does not prove that the PHP
compiler accepts the program. Name resolution, type checking and execution are
outside this library. PHP syntax introduced after 8.4 is outside its scope.

```rust
use pprs::{parse_strict, ParseOptions, PhpVersion};
let options = ParseOptions {
    version: "7.4".parse()?,
    short_open_tag: true,
    ..Default::default()
};
assert_eq!(options.version, PhpVersion::Php74);
assert!("8.5".parse::<PhpVersion>().is_err());
let file = parse_strict("<? echo 1;", options)?;
assert_eq!(file.program().statements.len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Collect declarations

`visit::walk` yields a node before its children. Named functions appear as
`StmtKind::Function`, methods as `StmtKind::Method`, and anonymous functions as
expression variants. Names remain unresolved, so a function's name here excludes
its enclosing namespace.

```rust
use pprs::{ast::StmtKind, parse_strict, visit::{self, Node}, ParseOptions};
let file = parse_strict(
    "<?php namespace App; function greet() {} class User { function save() {} }",
    ParseOptions::default(),
)?;
let functions: Vec<_> = visit::walk(file.program())
    .filter_map(|entry| match entry.node {
        Node::Stmt(stmt) => match &stmt.kind {
            StmtKind::Function(function) => Some(function.name.bytes().as_bytes()),
            _ => None,
        },
        _ => None,
    })
    .collect();
assert_eq!(functions, vec![b"greet".as_slice()]);
# Ok::<(), pprs::StrictError>(())
```

The program has depth one. Children retain their order in the syntax.
Traversal does not follow execution order or sort nodes by their byte offsets.
For example, a `for` loop's steps precede its body. Signatures and declaration
payloads do not add wrapper nodes. Simple types appear as names or identifiers;
composite types appear as `Node::Type`. Interpolated expressions appear as
`Node::Expr`, and text segments as `Node::StringPart`. Array holes and absent
optional children produce no entries. Comments and tokens are separate from
AST traversal.

The iterator uses an explicit stack. `find`, `any`, `try_for_each` or dropping it
can stop traversal early. If you retain the iterator, its next entry follows the
one that stopped the search, which may be a child of that node. `filter` does not
skip subtrees. Repeated calls after exhaustion return `None`.

## Inspect spelling and comments

`Node::meta()` returns metadata for every visited node except the program root.
`origin()` describes the semantic node; `enclosing_origin()` also includes
surrounding grouping parentheses. A synthetic origin makes no claim about source
text. Comments use indices into the result's token stream.

```rust
use pprs::{ast::{Origin, StmtKind}, parse_strict, visit::{self, Node}, ParseOptions};
let file = parse_strict("<?php /** Public entry. */ function greet() {}", ParseOptions::default())?;
let node = visit::walk(file.program()).find(|entry| {
    matches!(entry.node, Node::Stmt(stmt) if matches!(stmt.kind, StmtKind::Function(_)))
}).unwrap().node;
let meta = node.meta().unwrap();
let Origin::Parsed { span, .. } = meta.origin() else { panic!("expected parsed origin") };
let result = file.result();
assert_eq!(result.source().slice(span), Some(b"function greet() {}".as_slice()));
let comments: Vec<_> = meta.comments().iter()
    .map(|&index| result.tokens().text(index).unwrap())
    .collect();
assert_eq!(comments, vec![b"/** Public entry. */".as_slice()]);
# Ok::<(), pprs::StrictError>(())
```

Spans are half-open byte ranges with no source identity. Keep them with the
source that produced them. Slicing checks bounds, not identity. Locations use
one-based lines and byte columns, not Unicode display columns. LF, CRLF and
lone CR each count as one line break. EOF is a valid empty span.

Source text, identifiers and decoded strings can contain invalid UTF-8.
`ByteString::as_bytes()` preserves those bytes. Use `std::str::from_utf8` when
you need checked text, or choose a lossy conversion in your own display code.
Raw spelling stays in the source even when literal decoding changes the value.
Integers use signed 64-bit values on every host. Overflow promotes numeric
literals to binary64, which can reach infinity. Unary signs remain operators.
The library does not evaluate constant expressions. Standalone `literal`
functions have no parser budget; callers must bound their inputs.

## Handle errors and partial trees

`parse` returns a result for malformed input. Inspect its status and diagnostics
before treating the AST as complete. `Recovered` means parsing finished with
errors. `Aborted` means parsing stopped, and any root may be partial. An absent
root differs from an empty program. Both prevent strict success.

```rust
use pprs::{parse, ParseOptions, ParseStatus, DiagnosticCode, ast::StmtKind};
let result = parse("<?php $x = ; echo 2;", ParseOptions::default())?;
assert_eq!(result.status(), ParseStatus::Recovered);
assert!(result.diagnostics().iter().any(|d| d.code == DiagnosticCode::UnexpectedToken));
assert!(result.program().unwrap().statements.iter()
    .any(|s| matches!(s.kind, StmtKind::Echo(_))));
for diagnostic in result.diagnostics() {
    let location = result.source().location(diagnostic.primary.start()).unwrap();
    println!("{} at {}:{}: {}", diagnostic.code.as_str(), location.line,
        location.column, diagnostic.message);
}
let failure = result.into_strict().unwrap_err();
let recovered = failure.into_result();
assert_eq!(recovered.status(), ParseStatus::Recovered);
# Ok::<(), pprs::InputTooLarge>(())
```

`parse_strict` returns `StrictError::Input` for an oversized input, with its length
and limit but no parse result. `StrictError::Parse` retains the recovering result
through `result()` or `into_result()`. `parse_source(source, options)?.into_strict()`
provides strict conversion when you already own a `SourceText`.

Diagnostic codes and categories support programmatic decisions. English messages
and exact recovery trees can change as bugs are fixed. `expected` lists useful
token expectations when available; an empty list does not mean every token is
valid. `found: Some(TokenKind::Eof)` differs from `None`. Related locations are
in `secondary`. Diagnostics follow deterministic discovery order, which need not
be source order. PPRS suppresses duplicate syntax cascades at the same location.
It does not promise identical wording or malformed trees to PHP-Parser.

## Ownership, sharing and limits

`parse` and `lex` copy input bytes after checking the byte budget. Results outlive
the caller's buffer. `SourceText::clone()` shares immutable bytes and the line
index. `parse_source` and `lex_source` consume that shared source without copying
it again. Constructing `SourceText` itself builds the index without parser limits;
the source entry points check a source that the caller has already allocated.
Use raw-input entry points when preflight rejection must happen before indexing.

```rust
use pprs::{parse_source, SourceText, ParseOptions};
let source = SourceText::from("<?php echo 1;");
let saved = source.clone();
let result = parse_source(source, ParseOptions::default())?;
assert_eq!(result.source().as_bytes().as_ptr(), saved.as_bytes().as_ptr());
# Ok::<(), pprs::InputTooLarge>(())
```

Results, sources, options and ASTs are `Send + Sync`. Each call owns its parser
state. You can share a result through `Arc` and inspect it on several threads.
Results expose immutable trees. Clone a tree if you need to edit it, and use
synthetic metadata for nodes that no longer represent their original source.
There is no mutation visitor, printer or public serialization format.

Default limits are 16 MiB of input, depth 128, 1,000,000 tokens, 250,000 nodes
and 100 diagnostics. `Limits::new` validates custom budgets. Token accounting
includes trivia and EOF, and diagnostic accounting reserves the terminal limit
error. The parser caps effective depth at 128 even if configured higher. Resource
exhaustion produces an abort and a diagnostic; an unscanned suffix remains an
explicit token when bytes remain.

These limits bound work and structure, not process RSS. Line indexes, ASTs and
allocator retention can use much more memory than the input size. Lower limits
for constrained environments. Allocation failure is not a recoverable parser
error. Manually constructed ASTs have no parser depth bound.

`lex` and `lex_source` return lossless tokens without a tree. Check both
`LexResult::is_complete()` and its diagnostics. Completion alone does not mean
valid PHP. Standalone lexing scans past `__halt_compiler()` as ordinary PHP;
only parsing validates the statement and retains the remaining bytes as raw data.

## API compatibility

The crate uses semantic versioning. In the 0.1 series, breaking Rust API changes
require 0.2. After 1.0, they require a new major version. Bug fixes may change
incorrect parsing or diagnostics within a compatible release. Diagnostic code
identifiers keep their meaning; exact messages, `Debug` output, layout and
allocation counts are not serialization or ABI contracts. MSRV increases require
a new minor release and a release note. The current MSRV is Rust 1.85.

`ExprKind`, `StmtKind`, `BinaryOp`, `TokenKind`, `DiagnosticCode`,
`LiteralErrorKind` and `visit::Node` are non-exhaustive. Consumers must allow
unknown variants. Other enums and public-field structs are closed for the
current compatible release line. Adding a required field or a closed-enum
variant is a breaking change. Internal traversal matches remain exhaustive so
new syntax forces a traversal review.
