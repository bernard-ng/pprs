# Implementation plans

Status: planning only. No parser implementation has started.

The first stable release will be a reusable Rust library for PHP **7.0–8.4**, as confirmed on 2026-10-16. There is no product CLI.

1. [Technical implementation plan](technical-implementation-plan.md): scope, architecture, crate choices, ordered implementation steps, and release gates.
2. [Reference and testing strategy](reference-and-testing-strategy.md): pinned PHP-Parser reference, fixture inventory, oracle/export design, compatibility rules, and verification.

Start implementation at **P00**, then follow the dependency order in the technical plan. A phase is complete only when its acceptance checks pass. Update these documents when evidence changes a design decision or the agreed scope changes.
