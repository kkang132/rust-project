# Development Metrics

Runtime and operational metrics for `pr-analyzer`. This file tracks observable behavior of the tool to inform optimization and debugging.

## Runtime Performance Baselines

Measured with `RUST_LOG=info` against a medium-sized PR (~300 lines, 5 files).

| Phase | Metric | Baseline | Notes |
|-------|--------|----------|-------|
| URL Parsing | Latency | <1ms | In-process string parsing |
| Config Loading | Latency | <1ms | Single TOML file read |
| GitHub API — Metadata | Latency | 200-800ms | Depends on network + GitHub load |
| GitHub API — Diff | Latency | 200-800ms | Sequential after metadata fetch |
| Diff Parsing | Latency | <5ms | Custom parser, ~200 lines of code |
| Security Analysis | Latency | <5ms | Pattern matching, no I/O |
| Complexity Analysis | Latency | <1ms | Counting metrics |
| Style Analysis | Latency | <2ms | Pattern + layer checks |
| Report Generation | Latency | <1ms | String formatting |
| **Total (typical)** | **End-to-end** | **500ms-2s** | **Dominated by GitHub API calls** |

## Observability Stack

| Component | Tool | Activation |
|-----------|------|------------|
| Structured logging | `tracing` + `tracing-subscriber` | `RUST_LOG=info` (or `debug`, `trace`) |
| Span timing | `tracing` spans with `#[instrument]` | Visible at `debug` level |
| Error context | `thiserror` per-module error enums | Always active |

### Log Levels

| Level | What it shows |
|-------|---------------|
| `error` | Unrecoverable failures (API errors, missing token) |
| `warn` | Degraded operation (config file missing, fallback behavior) |
| `info` | Key pipeline stages (fetching PR, running analysis, writing report) |
| `debug` | Per-analyzer timing, individual findings, diff parse details |
| `trace` | Raw API responses, full diff content, pattern match details |

### Usage

```bash
# Default (no tracing output — clean report only)
cargo run -- https://github.com/org/repo/pull/42

# Info-level: see pipeline stages
RUST_LOG=info cargo run -- https://github.com/org/repo/pull/42

# Debug: see per-analyzer timing
RUST_LOG=debug cargo run -- https://github.com/org/repo/pull/42

# Module-specific tracing
RUST_LOG=pr_analyzer::analysis=debug cargo run -- https://github.com/org/repo/pull/42
```

## Key Bottleneck

The GitHub API calls account for ~90% of wall-clock time. The two HTTP requests (metadata + diff) are sequential because the diff endpoint shares the same URL with a different `Accept` header. Parallelizing them is possible but would require separate URL construction.

## Metrics to Watch

| Metric | Threshold | Action if exceeded |
|--------|-----------|-------------------|
| Total end-to-end | >5s | Check network, GitHub rate limits |
| Any single analyzer | >100ms | Profile the analyzer; likely a regex issue |
| Diff parsing | >50ms | Check for unusually large diffs |
| Findings count | >50 per analyzer | Consider noise reduction / deduplication |

---

*Update baselines after significant changes to analysis logic or API interaction.*
