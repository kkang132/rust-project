# Claude Agent Instructions — pr-analyzer

## Your Role

You own the **analysis engine** and **report formatting** modules. You also handle final integration across the codebase.

## Assigned Modules

- `src/analysis/` — `mod.rs`, `security.rs`, `complexity.rs`, `style.rs`
- `src/report/` — `mod.rs`, `types.rs`
- Integration work in `src/main.rs` (wiring analyzers to the runner)

## Constraints

- Read `AGENT.md` at the project root for the full product specification.
- Read `skills.md` at the project root for Rust conventions used in this project.
- Do NOT modify files in `src/pr/` or `src/config.rs` — those belong to Codex.
- If you need changes in Codex-owned modules, document the request in `claude/handoff.md`.

## Architecture Rules

- Every analyzer implements the `Analyzer` trait defined in `src/analysis/mod.rs`.
- Analyzers must be `Send + Sync` so they can run concurrently via `tokio::join!`.
- Use `thiserror` for error types. No `unwrap()` in library code.
- Return structured `AnalysisResult` values — never print directly from analyzers.
- Report formatting is the only module that writes to stdout or files.

## Workflow

Follow the full workflow defined in the repo-root `AGENTS.md`. The short version:

1. **Branch**: Create a branch from `main` using `claude/<type>/<name>` naming. Use a worktree if practical.
2. **Implement**: Check `AGENT.md` for scope. Modify only your assigned modules.
3. **Test**: Write unit tests in `#[cfg(test)]` blocks. Run `cargo clippy -- -D warnings` and `cargo fmt`.
4. **Commit**: Use the WHY/HOW/SCOPE commit message format from `AGENTS.md`.
5. **Review request**: Produce a structured review request (Intent, Changes, Testing, Risks, Diff).
6. **Self-review**: Switch to reviewer mode. Evaluate against the 8-point checklist in `AGENTS.md`.
7. **Merge or iterate**: If approved, squash/rebase onto `main` with `Reviewed-by` metadata. If changes needed, iterate on the same branch.
8. **Handoff**: If blocked on a Codex-owned module, write the blocker to `claude/handoff.md`.
9. **ADR**: If you made a significant architectural decision, record it in `docs/adr/`.
