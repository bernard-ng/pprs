# PPRS : PHP Parser written in Rust

A Rust library for parsing PHP 7.0 through 8.4. PPRS returns an owned AST, preserves
source bytes and comments, and reports errors with byte spans. It uses
[nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) as its compatibility reference.

PPRS requires Rust 1.85 or later. It has no PHP runtime dependency and no command-line
program. Add it to your project's `Cargo.toml`:

```toml
[dependencies]
pprs = "0.1"
```

## Parse PHP

```rust
use pprs::{ast::StmtKind, parse_strict, visit::{self, Node}, ParseOptions};

let file = parse_strict("<?php function greet() { return 'Hello'; }", ParseOptions::default())
    .expect("valid PHP");
for entry in visit::walk(file.program()) {
    if let Node::Stmt(stmt) = entry.node {
        if let StmtKind::Function(function) = &stmt.kind {
            println!("{:?}", function.name.bytes());
        }
    }
}
```

Pass bytes from `std::fs::read` to parse a file. Parsing starts outside PHP mode,
so untagged input becomes inline HTML. The caller handles file access; PPRS never
reads files or executes PHP.

Use `parse` to retain a partial tree after errors. Check the result's status and
diagnostics before treating the tree as complete. `parse_strict` rejects recovery
and resource aborts, and keeps the result in its parse error. Oversized input
returns a separate `InputTooLarge` error.

## Work with the result

The [API guide](https://docs.rs/pprs) has examples for file input, declarations,
comments, source spans, error recovery and shared source storage. Run
`cargo doc --open` for local API documentation. The guide's Rust examples run
as documentation tests.

- Source and decoded strings preserve arbitrary bytes, including invalid UTF-8.
- Spans use half-open byte offsets. Line and byte-column numbers start at one.
- `visit::walk` traverses the AST without recursive visitor calls and supports early exit.
- Results own their data and support shared access across threads.
- Configurable limits bound input size, depth, tokens, nodes and diagnostics.

The default PHP target is 8.4. Other targets change syntax interpretation and AST
construction according to the reference. Strict success does not imply complete
PHP compiler validation or strict historical-version validity. PPRS does not
resolve names, evaluate code, print PHP or define a public serialization format.
Malformed-input trees and diagnostic wording can differ from the reference.

## Compatibility and testing

The [compatibility records](https://github.com/bernard-ng/pprs/blob/main/docs/compatibility/README.md) describe the supported
scope. The [test corpus](https://github.com/bernard-ng/pprs/blob/main/tests/fixtures/README.md) records exact comparisons and
reviewed differences. [Performance measurements](https://github.com/bernard-ng/pprs/blob/main/docs/benchmarks/robustness-performance.md) include
allocation and memory behavior. Resource limits are not a process-memory quota.

See [CONTRIBUTING.md](https://github.com/bernard-ng/pprs/blob/main/CONTRIBUTING.md) for checks and reference regeneration.

## License

Original PPRS code is licensed under the [MIT license](LICENSE).
Upstream attribution and its BSD license are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
and [reference/LICENSE.nikic-PHP-Parser](reference/LICENSE.nikic-PHP-Parser).
