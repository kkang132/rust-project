# Architecture Decision Log

This file records key design decisions made during the development of `pr-analyzer`. Each entry captures the reasoning behind a choice so that future developers (or future us) can understand not just *what* the code does, but *why* it's shaped this way.

Entries are numbered and appended chronologically. We don't delete or rewrite old entries — if a decision is reversed, we add a new entry referencing the old one.

---

## ADR-001: Two-Agent Development Model (Claude + Codex)

**Date:** 2026-02-17
**Status:** Active

**Context:** This project is being built using AI coding agents. We needed to decide whether to use a single agent for everything or split responsibilities.

**Decision:** Split ownership across two agents with explicit boundaries:
- **Codex** owns PR fetching, diff parsing, CLI, and config (`src/pr/`, `src/config.rs`, `src/main.rs`)
- **Claude** owns analysis engine and report formatting (`src/analysis/`, `src/report/`)

**Rationale:** Clear module boundaries prevent agents from making conflicting edits to the same files. Each agent can work independently within its domain. Handoff files (`claude/handoff.md`, `codex/handoff.md`) serve as the coordination mechanism.

**Alternatives considered:**
- *Single agent:* Simpler coordination but slower (serial work) and higher risk of context loss across the full codebase.
- *File-level locking:* Too granular; module-level ownership is easier to reason about.

---

## ADR-002: Trait-Based Analyzer Architecture

**Date:** 2026-02-17
**Status:** Active

**Context:** The tool needs multiple independent analysis passes (security, complexity, style). We needed a way to structure these so they're independently testable and can run concurrently.

**Decision:** Define an `Analyzer` trait with `name()` and `async analyze()` methods. Each analyzer is a separate struct implementing this trait. A `run_all()` function executes them concurrently via `tokio::join!`.

**Rationale:** The trait-based approach gives us:
- Independent unit testing per analyzer with fixture diffs
- Concurrent execution without shared mutable state (each analyzer is `Send + Sync`)
- Easy extensibility — new analyzers just implement the trait

**Alternatives considered:**
- *Single monolithic analyze function:* Simpler initially but harder to test, extend, and parallelize.
- *Plugin system with dynamic loading:* Over-engineered for MVP scope. Could revisit if custom rule plugins become a requirement.

---

## ADR-003: Per-Module Error Types

**Date:** 2026-02-17
**Status:** Active

**Context:** Rust requires explicit error handling. We needed to decide between a single global error type or module-scoped errors.

**Decision:** Each module defines its own error enum (`PrError`, `ConfigError`, `AnalysisError`, `ReportError`) using `thiserror` for derive macros. Errors propagate via `?` in library code; `unwrap()` is only permitted in tests.

**Rationale:** Module-scoped errors keep each module's failure modes self-contained. Callers can match on the specific error variants they care about. `thiserror` eliminates boilerplate without runtime overhead.

**Alternatives considered:**
- *Single `AppError` enum:* Leads to a god-enum that grows with every module. Harder to reason about which errors a function can actually produce.
- *`anyhow` everywhere:* Good for applications, but we wanted typed errors for the library-like analysis/report modules where callers benefit from matching on variants.

---

## ADR-004: Optional Configuration with Sensible Defaults

**Date:** 2026-02-17
**Status:** Active

**Context:** The tool needs a GitHub token and may need configurable thresholds (e.g., security patterns, complexity limits). We needed to decide how configuration works.

**Decision:** Configuration loads from an optional `.pr-analyzer.toml` file in the current directory, with `GITHUB_TOKEN` falling back to an environment variable. The tool works with zero configuration — all analysis thresholds have hardcoded defaults.

**Rationale:** Zero-config startup means a developer can `cargo run -- <url>` immediately after setting `GITHUB_TOKEN`. The TOML file exists for teams that want to customize thresholds for their codebase.

**Alternatives considered:**
- *Required config file:* Higher friction for first-time use.
- *CLI flags for everything:* Gets unwieldy with many thresholds; config file is better for persistent team settings.
- *XDG config directory:* More correct on Linux but adds complexity for a tool that's typically run per-repo.

---

## ADR-005: Risk Level Aggregation via Maximum

**Date:** 2026-02-17
**Status:** Active

**Context:** Each analyzer produces its own `RiskLevel` (Low, Medium, High). The report needs a single overall risk level.

**Decision:** The overall risk is the maximum risk level across all analyzers. If any analyzer reports High, the overall is High.

**Rationale:** Conservative by design — a PR with a critical security finding shouldn't get a "Medium" overall just because complexity and style are fine. The goal is to surface risk, not average it away.

**Alternatives considered:**
- *Weighted average:* More nuanced but harder to explain. A "Medium" overall when there's a High security finding would undermine trust.
- *Per-analyzer reporting only (no overall):* Leaves interpretation to the reader, which defeats the purpose of a quick risk signal.

---

## ADR-006: Unified Diff Parsing (Custom, Not a Crate)

**Date:** 2026-02-17
**Status:** Active

**Context:** The tool needs to parse unified diffs from GitHub's API to extract per-file, per-hunk change data for the analyzers.

**Decision:** Implement a custom diff parser in `src/pr/diff.rs` rather than using an existing crate.

**Rationale:** The parser is straightforward (~200 lines) and tightly coupled to our data model (`DiffFile`, `Hunk` with raw line prefixes). Existing diff crates either parse too much (full patch application) or too little (no hunk-level detail). Keeping it in-house means we control exactly what data the analyzers receive.

**Alternatives considered:**
- *`unidiff` crate:* Python-centric design ported to Rust; API doesn't map cleanly to our needs.
- *`git2` crate:* Heavy dependency (links libgit2) for a problem that's essentially string parsing.

---

## ADR-007: Correct Analyzer Trait Signature and Naming Convention Violation

**Date:** 2026-02-17
**Status:** Active

**Context:** An architecture audit found two issues:
1. AGENT.md showed the `Analyzer` trait returning `AnalysisResult` directly, but the implementation (correctly) returns `Result<AnalysisResult, AnalysisError>`. This contradicted ADR-003's requirement for per-module error types and `?` propagation.
2. The `_colorize_risk` helper in `src/report/mod.rs` used a leading underscore, which in Rust signals an unused binding. The function is actively used, making the name misleading and inconsistent with the `snake_case` convention in `skills.md`.

**Decision:**
- Updated AGENT.md's "Core Trait" code block to show `-> Result<AnalysisResult, AnalysisError>`, matching the implementation and ADR-003.
- Renamed `_colorize_risk` to `colorize_risk` in `src/report/mod.rs`.

**Rationale:** The code was already correct — the spec document (AGENT.md) was stale. Fixing the spec prevents future agents from "correcting" the code back to the wrong signature. The naming fix removes a misleading Rust convention violation that could confuse both human readers and linters.

---

## ADR-008: Structured Tracing with `tracing` + `tracing-subscriber`

**Date:** 2026-02-17
**Status:** Active

**Context:** The project had zero observability — no logging, no span timing, no structured diagnostics. All output went through `println!()` to stdout. Debugging performance issues (especially GitHub API latency) or analyzer behavior required adding temporary print statements.

**Decision:** Add `tracing` and `tracing-subscriber` (with `env-filter`) as dependencies. Instrument the pipeline with spans and events:
- `main.rs`: Initialize subscriber writing to stderr, gated by `RUST_LOG` env var. Top-level span covers the full analysis.
- `pr/mod.rs`: `#[instrument]` on `fetch_pull_request` with `debug!` events for each API call.
- `analysis/mod.rs`: Per-analyzer spans via `.instrument()` on the `tokio::join!` futures.
- `report/mod.rs`: `#[instrument]` on `output` with debug events for terminal vs. file paths.

Tracing output goes to stderr so it never contaminates the report on stdout.

**Rationale:**
- `tracing` is the Rust ecosystem standard for structured diagnostics, recommended over `log` for async code.
- `env-filter` means zero overhead when `RUST_LOG` is unset — no impact on normal usage.
- Writing to stderr keeps the clean stdout report unaffected.
- `#[instrument]` macros keep instrumentation minimal and co-located with the code.

**Alternatives considered:**
- *`log` + `env_logger`:* Simpler but lacks span support and async-aware context propagation.
- *`println!` with `--verbose` flag:* Ad-hoc, unstructured, mixes with report output on stdout.
- *OpenTelemetry:* Over-engineered for a CLI tool; better suited for long-running services.

---

*To add a new entry: copy the template below, fill it in, and append it above this line.*

```markdown
## ADR-NNN: Title

**Date:** YYYY-MM-DD
**Status:** Active | Superseded by ADR-XXX | Deprecated

**Context:** What situation or problem prompted this decision?

**Decision:** What did we decide?

**Rationale:** Why this approach over others?

**Alternatives considered:**
- *Alternative A:* Why not this.
- *Alternative B:* Why not this.
```
