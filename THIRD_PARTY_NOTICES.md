# Third-party notices

## nikic/PHP-Parser

PPRS uses [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) as its parser compatibility reference. The initial frozen revision is `8d2b672aff65a1a4dbf7936cec27d19f51989412`.

Copyright (c) 2011, Nikita Popov. All rights reserved.

The upstream BSD-3-Clause license is reproduced in full in [reference/LICENSE.nikic-PHP-Parser](reference/LICENSE.nikic-PHP-Parser). Preserve that notice with copied/adapted grammar, source or tests, including future exported fixtures and distributed packages. Attribution does not imply endorsement.

P02 includes copied parser fixture files, expanded/normalized oracle expectations and derived test cases under `tests/fixtures/upstream/`, with a full license copy in that directory. `tools/reference/bootstrap.php` adapts the upstream fixture canonicalization helper. The complete upstream implementation remains in the separate ignored checkout. This notice does not choose a license for original PPRS code. That package-license decision remains open before release.

P06 adapts literal interpretation behavior from upstream `Node/Scalar/String_.php`, `Int_.php`, `Float_.php`, and `ParserAbstract.php` in `src/literal/` and its oracle exporter. The copyright and BSD-3-Clause notice above apply to those adaptations. Dedicated lexer/literal inputs are project-authored; their snapshots contain observations of the pinned reference.

P07 adapts expression productions, precedence and AST construction behavior from upstream `grammar/php.y` and `ParserAbstract.php` in `src/parser/`. The copyright and BSD-3-Clause notice above apply. `tests/support/parser.rs` projects the Rust AST to the upstream schema; `tests/fixtures/parser/` and the P07 difference snapshots retain reference-derived observations.

P08 extends those adaptations with statement, callable and type productions in `src/parser/statements.rs` and `callables.rs`, typed payloads in `src/ast/procedural.rs`, and reference-schema projections. `tests/fixtures/statements/` contains observations of the frozen reference from project-authored probes. The copyright and BSD-3-Clause notice above apply.
