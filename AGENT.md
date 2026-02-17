# PR Analyzer — Product Specification

## Overview

`pr-analyzer` is a CLI tool that takes a Pull Request URL and returns a structured risk assessment. It evaluates three dimensions of risk: **security**, **complexity**, and **style/architecture conformance**.

```
pr-analyzer https://github.com/org/repo/pull/42
```

## Why Rust

This project is built in Rust to exercise its three core advantages:

1. **Memory safety without garbage collection** — PR content parsing and diffstat analysis operate on potentially large diffs with zero risk of buffer overflows, use-after-free, or data races.
2. **Fearless concurrency** — The three risk assessments (security, complexity, style) run as concurrent async tasks, demonstrating safe parallelism via Rust's ownership model.
3. **Zero-cost abstractions** — Trait-based analysis pipeline lets each risk analyzer implement a shared interface with no runtime overhead, producing a fast native binary suitable for CI integration.

## User Flow

```
$ pr-analyzer https://github.com/org/repo/pull/42

PR #42: "Add OAuth2 login flow"
Author: alice | Files changed: 7 | +320 -45

═══ Security Risk Assessment ═══
Risk Level: HIGH
• New dependency `oauth2-lite` has no security audit history
• Raw SQL interpolation detected in migrations/003_add_tokens.sql
• Secrets handling in auth/config.rs does not use SecureString

═══ Complexity Assessment ═══
Risk Level: MEDIUM
• 2 new dependencies added (oauth2-lite, base64-url)
• Cyclomatic complexity increase: +12 across 3 files
• New module `auth/` adds 4 public types to the API surface

═══ Style & Architecture Assessment ═══
Risk Level: LOW
• All files follow existing module conventions
• No architectural boundary violations detected
• Minor: auth/config.rs uses `unwrap()` where codebase prefers `?` operator

═══ Overall Risk: HIGH ═══
```

## Architecture

```
src/
├── main.rs              # CLI entry point, arg parsing (clap)
├── pr/
│   ├── mod.rs           # PR data fetching (GitHub API via reqwest)
│   ├── diff.rs          # Diff parsing and file-level metadata
│   └── types.rs         # PR, File, Hunk structs
├── analysis/
│   ├── mod.rs           # Analyzer trait + concurrent runner
│   ├── security.rs      # Security risk analyzer
│   ├── complexity.rs    # Complexity risk analyzer
│   └── style.rs         # Style/architecture risk analyzer
├── report/
│   ├── mod.rs           # Report formatting and output
│   └── types.rs         # RiskLevel, Finding, Report structs
└── config.rs            # Configuration loading (.pr-analyzer.toml)
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  CLI (main.rs)                                                  │
│  pr-analyzer <PR_URL> [--output <path>]                         │
└──────────────────────────┬──────────────────────────────────────┘
                           │ parse URL → (owner, repo, pr_number)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  PR Fetcher (pr/mod.rs)                                         │
│  ┌─────────────────────┐   ┌──────────────────────┐            │
│  │ GitHub REST API      │   │ Raw Diff             │            │
│  │ GET /repos/.../pulls │   │ Accept: vnd.github.  │            │
│  │ → PR metadata (JSON) │   │ diff → unified diff  │            │
│  └─────────┬───────────┘   └──────────┬───────────┘            │
│            └──────────┬───────────────┘                         │
│                       ▼                                         │
│            ┌─────────────────────┐                              │
│            │ PullRequest struct  │                              │
│            │ - metadata          │                              │
│            │ - files: Vec<File>  │                              │
│            │ - hunks per file    │                              │
│            └─────────────────────┘                              │
└──────────────────────────┬──────────────────────────────────────┘
                           │ &PullRequest
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  Analysis Runner (analysis/mod.rs)                              │
│  tokio::join! — all three run concurrently                      │
│                                                                 │
│  ┌─────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │ Security    │  │ Complexity       │  │ Style/Architecture │ │
│  │ Analyzer    │  │ Analyzer         │  │ Analyzer           │ │
│  │             │  │                  │  │                    │ │
│  │ → patterns  │  │ → dep count      │  │ → naming rules     │ │
│  │ → secrets   │  │ → lines changed  │  │ → error patterns   │ │
│  │ → unsafe    │  │ → API surface    │  │ → lint checks      │ │
│  │ → injections│  │ → nesting depth  │  │ → boundary checks  │ │
│  └──────┬──────┘  └────────┬─────────┘  └─────────┬──────────┘ │
│         │                  │                       │            │
│         └──────────────────┼───────────────────────┘            │
│                            ▼                                    │
│              Vec<AnalysisResult>                                │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  Report Builder (report/mod.rs)                                 │
│                                                                 │
│  Vec<AnalysisResult> → Report                                   │
│                                                                 │
│  ┌─────────────────────┐   ┌──────────────────────┐            │
│  │ Terminal formatter   │   │ Markdown formatter   │            │
│  │ (default: stdout)   │   │ (--output <path>)    │            │
│  └─────────────────────┘   └──────────────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

### Key Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `reqwest` | HTTP client for GitHub API |
| `tokio` | Async runtime for concurrent analysis |
| `serde` / `serde_json` | JSON deserialization of API responses |
| `toml` | Config file parsing |
| `colored` | Terminal output formatting |

## Core Trait

```rust
#[async_trait]
pub trait Analyzer: Send + Sync {
    fn name(&self) -> &str;
    async fn analyze(&self, pr: &PullRequest) -> Result<AnalysisResult, AnalysisError>;
}
```

All three analyzers implement this trait. The runner executes them concurrently via `tokio::join!` and merges results into a single `Report`.

## Analysis Details

### 1. Security Risk Analyzer

Scans for:
- New dependencies without known audit status
- Patterns indicating SQL injection, command injection, XSS
- Hardcoded secrets or credentials
- Unsafe code blocks introduced
- Permission/scope changes in config files

### 2. Complexity Analyzer

Evaluates:
- Number of new dependencies added (parses Cargo.toml, package.json, etc.)
- Lines added/removed ratio
- Number of files changed
- New public API surface (exported types, functions)
- Nesting depth increases

### 3. Style & Architecture Analyzer

Checks:
- File placement matches existing module structure
- Naming conventions (snake_case, module naming patterns)
- Error handling patterns (unwrap vs ? operator, consistent Result usage)
- Import organization
- Architectural boundary violations (e.g., data layer importing from UI layer)
- Lint violations: flags common clippy-style issues in the diff (e.g., `unwrap()`, unnecessary `clone()`, missing `#[must_use]`, `todo!()` macros left in)

## Configuration

Optional `.pr-analyzer.toml` in the repo root:

```toml
[github]
# Token read from GITHUB_TOKEN env var by default

[security]
# Additional regex patterns to flag
patterns = ["TODO.*security", "FIXME.*auth"]

[style]
# Directories that define architectural layers
layers = ["api", "domain", "infra"]
# Allowed dependency direction: api -> domain -> infra
```

## MVP Scope

The MVP delivers:

- [x] CLI accepts a GitHub PR URL
- [x] Fetches PR metadata and diff via GitHub REST API
- [x] Runs three analyzers concurrently
- [x] Outputs a formatted terminal report with risk levels
- [x] Supports `--output <path>` flag to persist report to a file (markdown format)
- [x] Reads `GITHUB_TOKEN` from environment

### Out of Scope for MVP

- GitLab / Bitbucket support
- LLM-based analysis
- CI integration (GitHub Action wrapper)
- Web UI or TUI dashboard
- Custom rule plugins

## Agent Task Breakdown

This project uses two async agents. Claude serves as both implementer and reviewer. See `AGENTS.md` at the repo root for the full workflow.

| Agent | Primary Responsibility |
|-------|----------------------|
| **Claude** | Architecture, `analysis/` module, report formatting, integration, **code review for all agents** |
| **Codex** | PR fetching (`pr/` module), diff parsing, CLI setup, config |

Workflow: branch → commit (WHY/HOW/SCOPE) → review request → Claude reviews → squash/rebase onto `main`. No direct commits to `main`.

## Build & Run

```bash
cargo build --release
export GITHUB_TOKEN="ghp_..."

# Terminal output (default)
./target/release/pr-analyzer https://github.com/org/repo/pull/42

# Persist to file
./target/release/pr-analyzer https://github.com/org/repo/pull/42 --output report.md
```

## Testing Strategy

- Unit tests per analyzer with fixture diffs
- Integration test with a known public PR
- `cargo clippy` and `cargo fmt` enforced
