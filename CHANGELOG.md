# Changelog

## 0.1.0 (2026-10-17)

Initial library release for PHP 7.0–8.4, with Rust 1.85 as the minimum supported
compiler.

- Parse expressions, statements, functions, namespaces, declarations, class
  members and types into an owned AST.
- Preserve source bytes, comments, token ranges and literal spelling, including
  non-UTF-8 PHP input.
- Recover partial trees from malformed input, or require clean parsing through
  `parse_strict`.
- Inspect structured diagnostics and traverse trees with an iterative visitor.
- Configure limits for input size, nesting, tokens, nodes and diagnostics.

Compatibility follows the pinned nikic/PHP-Parser reference. Target selection
does not enforce every historical PHP compiler restriction. Malformed-input
recovery and diagnostic wording have documented differences from the reference.
The library does not resolve names, execute PHP, print PHP or expose a stable
serialization format.

The 0.1 API follows semantic versioning: breaking API changes require 0.2.
Resource limits bound parser work and structure, not process memory usage.
