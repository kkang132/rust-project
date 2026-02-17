# Rust Conventions — pr-analyzer

This file defines the shared coding conventions for all agents working on this project.

## Error Handling

- Use `thiserror` to define error enums per module.
- Propagate errors with `?`. Never use `unwrap()` or `expect()` in library code.
- `unwrap()` is acceptable only in tests and `main()` after all validation.
- Each module defines its own `Error` enum (e.g., `PrError`, `AnalysisError`).

```rust
#[derive(Debug, thiserror::Error)]
pub enum PrError {
    #[error("GitHub API request failed: {0}")]
    ApiRequest(#[from] reqwest::Error),
    #[error("Invalid PR URL: {0}")]
    InvalidUrl(String),
}
```

## Naming

- Modules: `snake_case` (e.g., `diff.rs`, `security.rs`)
- Types: `PascalCase` (e.g., `PullRequest`, `RiskLevel`)
- Functions: `snake_case` (e.g., `fetch_diff`, `analyze_security`)
- Constants: `SCREAMING_SNAKE_CASE`
- No abbreviations in public API names. `pr` is acceptable as it's domain terminology.

## Module Structure

- One `mod.rs` per directory that re-exports public types.
- Types shared across a module go in `types.rs` within that module's directory.
- Tests live in the same file under `#[cfg(test)] mod tests { ... }`.

## Async

- Runtime: `tokio` with `#[tokio::main]` in `main.rs`.
- Use `async_trait` crate for async trait methods.
- Analyzers run concurrently via `tokio::join!` — they must be `Send + Sync`.

## Dependencies

Only use crates listed in `AGENT.md`. If you need a new dependency, document the justification in your agent's `handoff.md` before adding it.

| Crate | Version Policy |
|-------|---------------|
| `clap` | Latest stable, derive API |
| `reqwest` | With `json` and `rustls-tls` features |
| `tokio` | With `full` feature |
| `serde` | With `derive` feature |
| `serde_json` | Latest stable |
| `toml` | Latest stable |
| `colored` | Latest stable |
| `thiserror` | Latest stable |
| `async-trait` | Latest stable |

## Formatting & Linting

- Run `cargo fmt` before every commit.
- Run `cargo clippy -- -D warnings` — all warnings are errors.
- No `#[allow(...)]` attributes without a comment explaining why.

## Testing

- Unit tests in `#[cfg(test)]` blocks within each source file.
- Use `#[tokio::test]` for async test functions.
- Test fixtures (sample diffs, API responses) go in `tests/fixtures/`.
- Name tests descriptively: `test_detects_sql_injection_in_diff`, not `test_1`.
