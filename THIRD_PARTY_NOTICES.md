# Third-party notices

## nikic/PHP-Parser

PPRS uses [nikic/PHP-Parser](https://github.com/nikic/PHP-Parser) as its parser compatibility reference. The initial frozen revision is `8d2b672aff65a1a4dbf7936cec27d19f51989412`.

Copyright (c) 2011, Nikita Popov. All rights reserved.

The upstream BSD-3-Clause license is reproduced in full in [reference/LICENSE.nikic-PHP-Parser](reference/LICENSE.nikic-PHP-Parser). Preserve that notice with copied/adapted grammar, source or tests, including future exported fixtures and distributed packages. Attribution does not imply endorsement.

P02 includes copied parser fixture files, expanded/normalized oracle expectations and derived test cases under `tests/fixtures/upstream/`, with a full license copy in that directory. `tools/reference/bootstrap.php` adapts the upstream fixture canonicalization helper. The complete upstream implementation remains in the separate ignored checkout. This notice does not choose a license for original PPRS code. That package-license decision remains open before release.
