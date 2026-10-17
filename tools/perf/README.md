# Development performance measurements

This separate, unpublished package measures the public library. It adds no product binary or runtime dependency. Allocation instrumentation uses [`stats_alloc` 0.1.10](https://docs.rs/stats_alloc/0.1.10/stats_alloc/), with no unsafe code in this repository. The package has its own committed lockfile and supports Rust 1.85.

From the repository root, on macOS or Linux with `/usr/bin/time`:

```sh
python3 tools/perf/measure.py --output target/performance.json --cold-target target/performance-cold
```

Both output paths must be new. Dependency downloads should be completed before a cold build measurement. Run with other CPU/memory-heavy work stopped. The script builds normal and allocation-instrumented release executables separately; timing samples always use the normal allocator. It records compiler/host information, dependency tree, source/input hashes, nine samples, configured iterations, per-result sizes/counts, and process peak RSS. It also measures a library-only build in an empty target directory and a subsequent no-op build. Cargo's registry/download caches remain warm.

The valid/invalid datasets are all 188 clean and 112 malformed core inputs from the frozen P02 corpus, using each input's target version. Synthetic dataset definitions live in `src/main.rs`: wide statements/members/arguments, a long comment, heredoc, a deep flat chain, an error storm, byte-limit whitespace/newlines, and node/token exhaustion. No reference checkout or PHP is needed.

Each timed batch parses every input in the chosen dataset and drops every result. Source ownership/indexing and lexing are included in full parsing. There is one warm-up batch and one calibration batch; nine timed samples target approximately 50 ms each. Reported rates are **offered input MiB/s**. For aborted workloads, inspect status and `scanned_bytes`: a high offered rate does not mean the full input was parsed.

Allocation mode measures one operation per input, excluding input loading, AST inspection and JSON reporting. It records allocation/reallocation counts, cumulative allocated bytes, and net live heap bytes while the result is retained. `inline_result_bytes` is the inline Rust value size, not the complete heap graph. Allocation counters are not peak-memory measurements. Process peak RSS is measured separately and includes the executable, loaded corpus, allocator retention, warm-up, metadata collection and repetitions. It must not be interpreted as per-result heap usage. `allocation_process` separately records the process peak for a single batch with the system-backed allocation counter, without warm-up or timing repetitions; fixture inputs in that batch are handled sequentially.

These are machine-specific descriptive baselines, not latency promises or a timing gate in CI. No confidence interval or cross-language speed claim is made. The [P12 baseline](../../docs/benchmarks/p12.md) retains raw evidence and limitations. CI checks formatting, linting and compilation of both allocator configurations; normal tests exercise deterministic robustness separately.
