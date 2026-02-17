# Codex Agent Instructions — pr-analyzer

## Your Role

You own the **PR data fetching**, **diff parsing**, **CLI setup**, and **configuration** modules.

## Assigned Modules

- `src/pr/` — `mod.rs`, `diff.rs`, `types.rs`
- `src/config.rs` — Configuration loading from `.pr-analyzer.toml` and env vars
- `src/main.rs` — CLI argument parsing with `clap` (shared with Claude for integration)

## Constraints

- Read `AGENT.md` at the project root for the full product specification.
- Read `skills.md` at the project root for Rust conventions used in this project.
- Do NOT modify files in `src/analysis/` or `src/report/` — those belong to Claude.
- If you need changes in Claude-owned modules, document the request in `codex/handoff.md`.

## Architecture Rules

- Use `reqwest` with async for all GitHub API calls.
- PR types (`PullRequest`, `DiffFile`, `Hunk`) live in `src/pr/types.rs`.
- Parse the GitHub PR URL to extract owner, repo, and PR number.
- Fetch both PR metadata (JSON) and the raw diff (Accept: application/vnd.github.diff).
- Use `thiserror` for error types. No `unwrap()` in library code.
- `GITHUB_TOKEN` is read from the environment; fail with a clear error if missing.
- CLI uses `clap` derive API. Support positional `<PR_URL>` and optional `--output <path>`.

## Workflow

Follow the full workflow defined in the repo-root `AGENTS.md`. The short version:

1. **Branch**: Create a branch from `main` using `codex/<type>/<name>` naming. Use a worktree if practical.
2. **Implement**: Check `AGENT.md` for scope. Modify only your assigned modules.
3. **Test**: Write unit tests in `#[cfg(test)]` blocks. Run `cargo clippy -- -D warnings` and `cargo fmt`.
4. **Commit**: Use the WHY/HOW/SCOPE commit message format from `AGENTS.md`.
5. **Review request**: Produce a structured review request (Intent, Changes, Testing, Risks, Diff).
6. **Review**: Claude reviews the branch against the 8-point checklist in `AGENTS.md`.
7. **Merge or iterate**: If approved, squash/rebase onto `main` with `Reviewed-by` metadata. If changes needed, iterate on the same branch.
8. **Handoff**: If blocked on a Claude-owned module, write the blocker to `codex/handoff.md`.
9. **ADR**: If you made a significant architectural decision, record it in `docs/adr/`.
