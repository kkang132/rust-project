# pr-analyzer

<!-- Uncomment and update once hosted on GitHub:
[![Build Status](https://img.shields.io/github/actions/workflow/status/OWNER/pr-analyzer/ci.yml?branch=main)](https://github.com/OWNER/pr-analyzer/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/pr-analyzer.svg)](https://crates.io/crates/pr-analyzer)
-->

A CLI tool that takes a GitHub Pull Request URL and returns a structured risk assessment across three dimensions: **security**, **complexity**, and **style/architecture conformance**.

```
$ pr-analyzer https://github.com/org/repo/pull/42

PR #42: "Add OAuth2 login flow"
Author: alice | Files changed: 7 | +320 -45

═══ Security Risk Assessment ═══
Risk Level: HIGH
• Raw SQL interpolation detected in migrations/003_add_tokens.sql
• Secrets handling in auth/config.rs does not use SecureString

═══ Complexity Assessment ═══
Risk Level: MEDIUM
• 2 new dependencies added (oauth2-lite, base64-url)
• Cyclomatic complexity increase: +12 across 3 files

═══ Style & Architecture Assessment ═══
Risk Level: LOW
• All files follow existing module conventions

═══ Overall Risk: HIGH ═══
```

## Features

- Three concurrent risk analyzers: security patterns, complexity metrics, style conformance
- Parses GitHub PRs via REST API (metadata + unified diff)
- Zero-config startup — just set `GITHUB_TOKEN` and go
- Optional `.pr-analyzer.toml` for team-specific thresholds
- Terminal and Markdown output formats
- Fast native binary suitable for CI integration

## Why Rust

This project exercises Rust's three core advantages:

1. **Memory safety without garbage collection** — PR content parsing and diffstat analysis operate on potentially large diffs with zero risk of buffer overflows, use-after-free, or data races. The custom diff parser in `src/pr/diff.rs` handles untrusted input from GitHub's API without a GC pause or a `try/catch` — invalid input produces typed errors (`PrError`), not crashes.

2. **Fearless concurrency** — The three risk analyzers run as concurrent async tasks via `tokio::join!`. Rust's ownership model guarantees at compile time that no analyzer can mutate shared state. This is a structural guarantee, not a convention. Adding a fourth analyzer requires zero synchronization code.

3. **Zero-cost abstractions** — The `Analyzer` trait lets each risk assessment implement a shared interface without paying for it at runtime. Each analyzer is a concrete type, so the compiler resolves calls directly rather than doing a lookup at runtime. (The `async_trait` macro does add one small allocation per call to box the returned future — this is a known Rust async limitation, not a design choice, and the cost is negligible for a CLI tool.) The result is a tool that starts instantly and analyzes diffs in milliseconds, making it viable as a CI gate without slowing down developer feedback loops.

## Installation

### From source

```bash
cargo install --path .
```

### Build manually

```bash
git clone <repo-url>
cd pr-analyzer
cargo build --release
# Binary is at ./target/release/pr-analyzer
```

**MSRV:** Rust stable (edition 2021). No nightly features required.

## Usage

Set your GitHub token, then point the tool at a PR:

```bash
export GITHUB_TOKEN="ghp_..."

# Terminal output (default)
pr-analyzer https://github.com/org/repo/pull/42

# Save report to file
pr-analyzer https://github.com/org/repo/pull/42 --output report.md
```

Optional: place a `.pr-analyzer.toml` in the repo root to customize security patterns, style layers, etc. See [AGENT.md](AGENT.md) § Configuration for the schema.

## Project Layout

```
src/
├── main.rs              # CLI entry point (clap)
├── config.rs            # Configuration loading (.pr-analyzer.toml + env)
├── pr/
│   ├── mod.rs           # PR data fetching (GitHub REST API)
│   ├── diff.rs          # Unified diff parser
│   └── types.rs         # PullRequest, DiffFile, Hunk structs
├── analysis/
│   ├── mod.rs           # Analyzer trait + concurrent runner
│   ├── security.rs      # Security risk analyzer
│   ├── complexity.rs    # Complexity risk analyzer
│   └── style.rs         # Style/architecture risk analyzer
└── report/
    ├── mod.rs           # Report formatting and output
    └── types.rs         # RiskLevel, Finding, Report structs
```

## Documentation

| File | Purpose |
|------|---------|
| [AGENT.md](AGENT.md) | Design spec — product requirements, architecture, data flow, analysis details |
| [skills.md](skills.md) | Coding conventions — error handling, naming, async patterns, linting |
| [docs/decisions.md](docs/decisions.md) | Architecture Decision Records (ADRs) |
| [claude/](claude/) | Claude agent instructions and handoff notes |
| [codex/](codex/) | Codex agent instructions and handoff notes |

`AGENT.md` is the authoritative source for what this tool does and how it's structured. Start there.

## Development Environment

This project is developed in [JetBrains Air](https://www.jetbrains.com/air/) using isolated git worktrees. Each agent operates in its own worktree, so Claude and Codex can edit files concurrently without producing merge conflicts or stepping on each other's in-progress changes. Air orchestrates the sessions; the worktrees provide the isolation.

### How the Rust Toolchain Boundaries Agent Edits

AI agents don't have taste, institutional memory, or a sense of "this feels wrong." They'll confidently produce code that compiles but violates project invariants. In this project, two mechanisms act as hard boundaries:

- **`cargo clippy -- -D warnings`** — Every Clippy warning is promoted to a build-breaking error. This catches categories of mistakes agents make routinely: unused variables left behind after a refactor, redundant clones, fallible operations that should use `?` instead of `.unwrap()`, and style violations defined in `skills.md`. Because Clippy runs as a gate (not a suggestion), an agent cannot land code that drifts from the project's conventions even if the agent's own training data would prefer a different style.

- **`cargo test`** — Tests in an agentic project aren't just regression checks. They're the **only durable record of intent**. When an agent refactors a function, the conversation context that explained *why* the function works a certain way is gone. The tests remain. Fixture-based analyzer tests (`tests/fixtures/sample_diff.patch`) encode specific detection expectations — if an agent rewrites the security analyzer and breaks SQL injection detection, the test fails, not a human reviewer's memory. Contract tests at module boundaries (e.g., verifying that `parse_diff()` produces `Hunk.lines` with raw `+`/`-`/` ` prefixes) prevent one agent from silently changing the interface another agent depends on.

Together, `clippy` and `test` form a mechanical review layer. They don't replace human judgment for design decisions — that's what the ADRs and handoff files are for — but they prevent the most common agentic failure mode: confidently shipping code that compiles but subtly breaks something.

## Agentic Workflow

This project is built by two AI agents (Claude and Codex) with minimal human oversight. The coordination model is documented in [docs/decisions.md](docs/decisions.md) ADR-001 and works as follows:

### Module Ownership

| Agent | Owns | Instructions |
|-------|------|-------------|
| **Claude** | `src/analysis/`, `src/report/`, integration, code review | [claude/CLAUDE.md](claude/CLAUDE.md) |
| **Codex** | `src/pr/`, `src/config.rs`, CLI, diff parsing | [codex/AGENTS.md](codex/AGENTS.md) |

Neither agent modifies the other's modules. When an agent needs a change in the other's domain, it writes a request to its handoff file.

### Coordination Files

- **`claude/handoff.md`** — Claude's requests to Codex (e.g., "I need `parse_diff()` to populate `Hunk.lines` with raw prefixes"). Each entry has a date, module, description, rationale, and status (OPEN/RESOLVED).
- **`codex/handoff.md`** — Codex's requests to Claude. Same format.
- **`docs/decisions.md`** — Architecture Decision Records. Any significant design choice (trait signatures, error strategies, parser approach) gets recorded here with context, rationale, and alternatives considered. Entries are append-only — reversals get new entries referencing the old ones.
- **`skills.md`** — Shared coding conventions (error handling, naming, async patterns, dependency policy). Both agents read this before writing code. If a convention needs to change, the change is recorded in `skills.md` itself and referenced in a new ADR.

### Why This Matters

In a traditional project, conventions live in developers' heads and are enforced through code review. With AI agents, there are no heads — conventions must be explicit, written, and machine-readable. The handoff files prevent conflicting edits. The ADRs prevent an agent from "improving" a deliberate choice it doesn't have context for (see ADR-007 for an example of this nearly happening). `skills.md` prevents style drift between agents.

## Build Status

Verified on Rust stable (edition 2021):

- `cargo build` — **0 warnings**, **0 errors**
- `cargo test` — **48 passed**, 0 failed

Last verified: 2026-02-17

## Testing Strategy

Testing in an agentic project serves a different purpose than in a traditional one. When humans write code, tests verify that the code does what the author intended. When agents write code, tests are the **only durable record of intent** — they outlast the conversation context that produced the implementation.

### How Tests Preserve Intent

- **Fixture-based analyzer tests** — `tests/fixtures/` contains a deliberately dirty diff (`sample_diff.patch` with SQL injection, hardcoded secrets, unsafe blocks) and a clean diff (`clean_diff.patch`). Each analyzer has unit tests against both. If an agent refactors an analyzer, these tests catch regressions in detection capability, not just compilation.
- **Per-module error types** — ADR-003 requires each module to define its own error enum. Tests exercise error paths explicitly (e.g., invalid PR URL → `PrError::InvalidUrl`, not a panic). This prevents an agent from collapsing error types into `anyhow::Error` for convenience.
- **Contract tests at module boundaries** — The `PullRequest` struct is produced by Codex's `pr/` module and consumed by Claude's `analysis/` module. Tests on both sides verify the contract: Codex tests that `parse_diff()` populates `Hunk.lines` with raw `+`/`-`/` ` prefixes; Claude tests that analyzers correctly interpret those prefixes.
- **Clippy and fmt as gates** — `cargo clippy -- -D warnings` and `cargo fmt --check` are not suggestions. They're mechanical enforcers of `skills.md` conventions that catch drift an agent might introduce without realizing it violates the project's style.

### Running Tests

```bash
cargo test                        # Full suite
cargo test --lib                  # Unit tests only
cargo clippy -- -D warnings       # Lint (all warnings are errors)
cargo fmt --check                 # Format check (CI-style)
```

## Contributing

1. Read [AGENT.md](AGENT.md) for the product spec and [skills.md](skills.md) for coding conventions.
2. Check [docs/decisions.md](docs/decisions.md) before making architectural changes — the decision you're about to make may already be recorded.
3. Branch from `main` using `<agent>/<type>/<name>` naming (e.g., `claude/feat/add-cyclomatic-depth`).
4. Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` before opening a PR.
5. If your change introduces a significant design choice, add an ADR to `docs/decisions.md`.

## Security

If you discover a security vulnerability, please report it privately rather than opening a public issue. Contact the maintainers directly.

## License

MIT
