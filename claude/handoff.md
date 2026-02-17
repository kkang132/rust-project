# Claude → Codex Handoff

Log requests here when you need changes in Codex-owned modules (`src/pr/`, `src/config.rs`, CLI args).

Format:
```
## [DATE] Request Title
- **Module:** file path
- **What I need:** description of the change
- **Why:** context for why it's needed
- **Status:** OPEN | RESOLVED
```

## [2026-02-17] Scaffolding Complete — Codex Modules Ready for Implementation

- **Module:** `src/pr/mod.rs`, `src/pr/diff.rs`, `src/pr/types.rs`, `src/config.rs`, `src/main.rs`
- **What I need:** Codex to implement all `todo!()` stubs in Codex-owned modules
- **Why:** Full project scaffolding is in place. All files compile structurally but contain `todo!()` placeholders. Codex owns PR fetching, diff parsing, config loading, and CLI wiring.
- **Status:** RESOLVED

### Implementation Order (Suggested)

1. **`src/config.rs`** — Start here. Implement `Config::load()`, `Config::load_from()`, and `github_token()`. These are self-contained. Uncomment the test in `test_parse_config_toml`.

2. **`src/pr/types.rs`** — Types are already fully defined. No work needed unless you want to add `Deserialize` derives for DiffFile/Hunk (currently only PullRequest has it since DiffFile/Hunk are parsed from diff text, not JSON).

3. **`src/pr/diff.rs`** — Implement `parse_diff()` to parse unified diff format. Test fixtures are in `tests/fixtures/sample_diff.patch` and `tests/fixtures/clean_diff.patch`. Uncomment and complete the 4 tests.

4. **`src/pr/mod.rs`** — Implement `parse_pr_url()` and `fetch_pull_request()`. The fetch function needs `reqwest` async calls to GitHub API. Uncomment and complete the 2 tests.

5. **`src/main.rs`** — Uncomment the TODO blocks to wire everything together. This depends on all other modules being functional.

### Contract Notes

- `PullRequest` struct is shared between Codex (produces it) and Claude (consumes it in analyzers). The struct is defined in `src/pr/types.rs`. If you need to change it, note it in `codex/handoff.md` so Claude can adapt the analyzers.
- `Config` is consumed by `fetch_pull_request()`. The struct shape is defined and should not need changes.
- Test fixtures in `tests/fixtures/` include a "dirty" diff (`sample_diff.patch` with SQL injection, hardcoded secrets, unsafe blocks) and a "clean" diff (`clean_diff.patch`). Use these for testing.

### Intent Preservation Notes

- The `Analyzer` trait in `src/analysis/mod.rs` expects `&PullRequest` — analyzers iterate over `pr.files` and their `hunks.lines` to scan for patterns. Ensure `DiffFile.hunks` and `Hunk.lines` are properly populated by `parse_diff()`.
- Each `Hunk.lines` entry should be the raw diff line including the `+`/`-`/` ` prefix character. Analyzers depend on this to distinguish additions from deletions.
- `PullRequest.files_changed`, `.additions`, `.deletions` should be totals matching the GitHub API response, not re-derived from the diff.

## [2026-02-17] README: Replace Placeholder URLs Once Repo Is Hosted

- **Module:** `README.md`
- **What I need:** Once the GitHub repo URL is known, update the following placeholders in `README.md`:
  1. Replace `<repo-url>` in the "Build manually" section with the actual clone URL
  2. Uncomment the badge block at the top and replace `OWNER` with the actual GitHub org/user
  3. Update the sample `pr-analyzer` commands if the repo URL is used as an example target
- **Why:** README currently avoids hardcoding a GitHub location since the repo hasn't been published yet. These are the only places that need updating.
- **Status:** OPEN
