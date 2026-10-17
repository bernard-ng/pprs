# Third-party notices

## nikic/PHP-Parser

PPRS uses [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) as its parser compatibility reference. The initial frozen revision is `8d2b672aff65a1a4dbf7936cec27d19f51989412`.

Copyright (c) 2011, Nikita Popov. All rights reserved.

The upstream BSD-3-Clause license is reproduced in full in [reference/LICENSE.nikic-PHP-Parser](reference/LICENSE.nikic-PHP-Parser). Preserve that notice with copied/adapted grammar, source or tests, including future exported fixtures and distributed packages. Attribution does not imply endorsement.

The reference test corpus includes copied parser fixture files, expanded/normalized oracle expectations and derived test cases under `tests/fixtures/upstream/`, with a full license copy in that directory. `tools/reference/bootstrap.php` adapts the upstream fixture canonicalization helper. The complete upstream implementation remains in the separate ignored checkout. This notice does not choose a license for original PPRS code. That package-license decision remains open before release.

The literal decoder in `src/literal/` and its oracle exporter adapt literal interpretation behavior from upstream `Node/Scalar/String_.php`, `Int_.php`, `Float_.php`, and `ParserAbstract.php`. The copyright and BSD-3-Clause notice above apply to those adaptations. Dedicated lexer/literal inputs are project-authored; their snapshots contain observations of the pinned reference.

The expression parser in `src/parser/` adapts expression productions, precedence and AST construction behavior from upstream `grammar/php.y` and `ParserAbstract.php`. The copyright and BSD-3-Clause notice above apply. `tests/support/parser.rs` projects the Rust AST to the upstream schema; `tests/fixtures/parser/` and the expression parser snapshots in `tests/fixtures/differences/` retain reference-derived observations.

The statement and callable parsers extend those adaptations with statement, callable and type productions in `src/parser/statements.rs` and `callables.rs`, typed payloads in `src/ast/procedural.rs`, and reference-schema projections. `tests/fixtures/statements/` contains observations of the frozen reference from project-authored probes. The copyright and BSD-3-Clause notice above apply.

The declaration and namespace parsers in `src/parser/declarations.rs`, `namespaces.rs`, and the callable parser adapt namespace normalization, declaration/member productions, modifiers and parser-level checks from `grammar/php.y`, `ParserAbstract.php` and `Modifiers.php`. Typed declaration payloads and traversal are in `src/ast/declarations.rs` and `src/visit.rs`. `tests/fixtures/declarations/` contains project-authored probes with reference-derived observations; the notice and BSD license above apply.
