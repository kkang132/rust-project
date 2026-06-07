# pr-analyzer

<!-- Uncomment and update once hosted on GitHub:
[![Build Status](https://img.shields.io/github/actions/workflow/status/OWNER/pr-analyzer/ci.yml?branch=main)](https://github.com/OWNER/pr-analyzer/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/pr-analyzer.svg)](https://crates.io/crates/pr-analyzer)
-->

A command-line tool that takes a GitHub pull request URL and returns a structured risk assessment across three dimensions: **security**, **complexity**, and **style/architecture conformance**. The wider aim is to show what current frontier models and human steering can do together in early 2026. Some call the practice agentic engineering.

## What This Project Demonstrates

This repository is an agentic engineering case study. The visible product is a Rust CLI. The deeper object is the control system around it: ownership boundaries, handoff files, ADRs, fixture tests, lint gates, and written conventions.

The project should not be read as "AI generated code." It should be read as a human principal setting the workflow, constraints, and review surfaces. Agents execute inside those boundaries.

Rust is central to the demonstration. A common objection is that LLMs do not understand memory allocation, aliasing, or lifetime pressure. This project accepts that objection. It answers by moving those concerns into the toolchain. Ownership, borrowing, typed errors, and `Send + Sync` constraints become mechanical checks. The human judgement lies in choosing a language and workflow where agent output is bounded before it is trusted.

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
- Parses GitHub PRs via the REST API, taking both metadata and the unified diff
- Zero-config startup: set `GITHUB_TOKEN` and run
- Optional `.pr-analyzer.toml` for team-specific thresholds
- Terminal and Markdown output formats
- Fast native binary suitable for CI integration

## Why Rust

This project exercises Rust's three core advantages:

1. **Memory safety without garbage collection.** Parsing and diffstat analysis run over potentially large diffs with no risk of buffer overflows, use-after-free, or data races. The custom parser in `src/pr/diff.rs` handles untrusted input from GitHub without a collection pause. Invalid input produces a typed `PrError` rather than a crash.

2. **Fearless concurrency.** The three analyzers run as concurrent async tasks under `tokio::join!`. Ownership guarantees at compile time that no analyzer can mutate shared state. The guarantee is structural, not a convention. Adding a fourth analyzer needs no synchronisation code.

3. **Zero-cost abstractions.** The `Analyzer` trait lets each assessment share an interface at no runtime cost. Each analyzer is a concrete type, so the compiler resolves calls directly rather than through a lookup. The `async_trait` macro adds one small allocation per call to box the returned future, a known limitation of async traits rather than a design choice, and the cost is negligible for a command-line tool. The binary starts at once and analyzes diffs in milliseconds, which makes it usable as a CI gate without slowing the developer feedback loop.

## Installation

### From source

```bash
cargo install --path .
```

### Build manually

```bash
git clone github.com/kkang132/rust-project
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

Optional: place a `.pr-analyzer.toml` in the repo root to customise security patterns, style layers, and similar. See the Configuration section of [SPEC.md](SPEC.md) for the schema.

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
| [SPEC.md](SPEC.md) | Design spec: product requirements, architecture, data flow, analysis details |
| [skills.md](skills.md) | Coding conventions: error handling, naming, async patterns, linting |
| [docs/decisions.md](docs/decisions.md) | Architecture Decision Records (ADRs) |
| [claude/](claude/) | Claude agent instructions and handoff notes |
| [codex/](codex/) | Codex agent instructions and handoff notes |

`SPEC.md` is the authoritative source for what this tool does and how it is structured. Start there.

## Development Environment

This project is developed in [JetBrains Air](https://air.dev/) using isolated git worktrees. Each agent operates in its own worktree, so Claude and Codex can edit files concurrently without merge conflicts or stepping on each other's in-progress changes. Air orchestrates the sessions; the worktrees provide the isolation.

### How the Rust Toolchain Bounds Agent Edits

AI agents have no taste, no institutional memory, and no sense that something feels wrong. They will confidently produce code that compiles but violates a project invariant. Two mechanisms act here as hard boundaries.

- **`cargo clippy -- -D warnings`.** Every Clippy warning becomes a build-breaking error. This catches the mistakes agents make routinely: variables left unused after a refactor, redundant clones, fallible operations that should use `?` instead of `.unwrap()`, and the style violations defined in `skills.md`. Because Clippy runs as a gate rather than a suggestion, an agent cannot land code that drifts from the conventions even when its own training would prefer another style.

- **`cargo test`.** Tests in an agentic project are more than regression checks. They are the only durable record of intent. When an agent refactors a function, the conversation that explained why it works a certain way is gone, and only the tests remain. The fixture tests in `tests/fixtures/sample_diff.patch` encode specific detection expectations, so if an agent rewrites the security analyzer and breaks SQL injection detection the test fails rather than a reviewer's memory. Contract tests at module boundaries, such as checking that `parse_diff()` produces `Hunk.lines` with raw `+`, `-`, and space prefixes, stop one agent from silently changing an interface another depends on.

Together, `clippy` and `test` form a mechanical review layer. They do not replace human judgement on design decisions, which is what the ADRs and handoff files are for. What they prevent is the most common agentic failure: shipping code that compiles but quietly breaks something.

## Agentic Workflow

This project is built by two AI agents, Claude and Codex, with minimal human oversight. The coordination model is recorded in [docs/decisions.md](docs/decisions.md) under ADR-001 and works as follows.

### Module Ownership

| Agent | Owns | Instructions |
|-------|------|-------------|
| **Claude** | `src/analysis/`, `src/report/`, integration, code review | [claude/CLAUDE.md](claude/CLAUDE.md) |
| **Codex** | `src/pr/`, `src/config.rs`, CLI, diff parsing | [codex/AGENTS.md](codex/AGENTS.md) |

Neither agent modifies the other's modules. When an agent needs a change in the other's domain, it writes a request to its handoff file.

### Coordination Files

- **`claude/handoff.md`**: Claude's requests to Codex, for example asking that `parse_diff()` populate `Hunk.lines` with raw prefixes. Each entry carries a date, module, description, rationale, and a status of OPEN or RESOLVED.
- **`codex/handoff.md`**: Codex's requests to Claude, in the same format.
- **`docs/decisions.md`**: the Architecture Decision Records. Any significant design choice, such as a trait signature, error strategy, or parser approach, is recorded here with its context, rationale, and the alternatives considered. Entries are append-only; a reversal gets a new entry that references the old one.
- **`skills.md`**: the shared coding conventions for error handling, naming, async patterns, and dependency policy. Both agents read it before writing code. A change to a convention is recorded in `skills.md` itself and referenced from a new ADR.

### Why This Matters

In a traditional project, conventions live in developers' heads and are enforced through review. With agents there are no heads, so conventions must be explicit, written, and machine-readable. The handoff files prevent conflicting edits. The ADRs stop an agent from improving away a deliberate choice it lacks the context for, a case ADR-007 records nearly happening. `skills.md` holds the line against style drift between agents.

## Build Status

Verified on Rust stable (edition 2021):

- `cargo build`: 0 warnings, 0 errors
- `cargo test`: 48 passed, 0 failed

Last verified: 2026-02-17

## Testing Strategy

Testing in an agentic project serves a different purpose than in a traditional one. When humans write code, tests verify that the code does what the author intended. When agents write code, tests are the only durable record of intent. They outlast the conversation that produced the implementation.

### How Tests Preserve Intent

- **Fixture-based analyzer tests.** `tests/fixtures/` holds a deliberately dirty diff (`sample_diff.patch`, with SQL injection, hardcoded secrets, and unsafe blocks) and a clean diff (`clean_diff.patch`). Each analyzer is tested against both. When an agent refactors an analyzer, these tests catch a regression in detection, not merely in compilation.
- **Per-module error types.** ADR-003 requires each module to define its own error enum. Tests exercise the error paths explicitly, so an invalid PR URL yields `PrError::InvalidUrl` rather than a panic. This stops an agent from collapsing the typed errors into `anyhow::Error` for convenience.
- **Contract tests at module boundaries.** The `PullRequest` struct is produced by Codex's `pr/` module and consumed by Claude's `analysis/` module. Both sides test the contract: Codex checks that `parse_diff()` populates `Hunk.lines` with raw `+`, `-`, and space prefixes, and Claude checks that the analyzers read those prefixes correctly.
- **Clippy and fmt as gates.** `cargo clippy -- -D warnings` and `cargo fmt --check` are not suggestions. They enforce the `skills.md` conventions mechanically and catch drift an agent introduces without seeing that it breaks the project's style.

### Running Tests

```bash
cargo test                        # Full suite
cargo test --lib                  # Unit tests only
cargo clippy -- -D warnings       # Lint (all warnings are errors)
cargo fmt --check                 # Format check (CI-style)
```

## Contributing

1. Read [SPEC.md](SPEC.md) for the product spec and [skills.md](skills.md) for coding conventions.
2. Check [docs/decisions.md](docs/decisions.md) before any architectural change. The decision you are about to make may already be recorded.
3. Branch from `main` using `<agent>/<type>/<name>` naming (e.g., `claude/feat/add-cyclomatic-depth`).
4. Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` before opening a PR.
5. If your change introduces a significant design choice, add an ADR to `docs/decisions.md`.

## Security

If you discover a security vulnerability, please report it privately rather than opening a public issue. Contact the maintainers directly.

## License

MIT
