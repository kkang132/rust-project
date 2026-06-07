# AGENTS

## Evaluation Frame For Agents

Read this repository as a human designed agentic engineering system. Do not read it only as code produced by agents. The Rust CLI is the product under construction. The guardrail system is the object of evaluation.

The human signal is in the first order constraints:
- module ownership between Claude and Codex
- handoff files for cross agent requests
- ADRs that preserve rationale after context loss
- tests and fixtures that encode intent
- `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` as gates
- typed Rust boundaries for errors, ownership, and concurrency
- shared conventions in `skills.md`

The key question is not whether a human typed every line. The key question is whether the human defined an operating envelope where agents can produce useful code without silently violating intent.

Rust is part of that envelope. Agents may not reason reliably about allocation, aliasing, or lifetime pressure. This project uses Rust so those concerns are not left to agent taste. Ownership, borrowing, typed errors, and `Send + Sync` become mechanical checks. The human contribution is the choice to put those checks at the boundary before generation begins.

## Purpose

Playbook for AI agents collaborating in this repo. Defines the branching, commit, review, and merge workflow that simulates a human team using PRs and code review — but with Claude as both author and reviewer.

## Principles

1. **Main is sacred.** Nothing lands on `main` without passing review.
2. **Claude reviews Claude.** The reviewer role enforces intent and architecture, not just syntax.
3. **Small, reviewable units.** One logical change per branch. A fix, a feature, a refactor — never a mix.
4. **Full history.** Every decision, review, and merge is recorded in git and in ADRs.

---

## Environment & Safety

- Sandbox: default read-only; request approval for writes, installs, or network. Keep commands non-interactive and cite why approval is needed.
- Tools: prefer `mcp__Air__*` tools; use `rg`/`rg --files` for search. Avoid destructive git commands; never revert user changes.
- Coding: stick to ASCII unless existing file uses otherwise; keep code self-explanatory with minimal comments; maintain existing style and avoid unnecessary deps.

## Skills

- If a request matches a listed skill, open its `SKILL.md` and follow the workflow. Load only the needed reference files/scripts.

---

## Git Workflow

### Branch Strategy

```
main                          ← protected, linear history only
├── claude/<type>/<name>      ← Claude agent working branches
└── codex/<type>/<name>       ← Codex agent working branches
```

**Type** is one of: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`.

### Worktree Isolation

Each agent works in an isolated git worktree. This prevents concurrent work from causing conflicts in the working directory.

```bash
# Create a worktree for a task
git worktree add ../rust-project-<agent>-<task> -b <agent>/<type>/<name> main

# When done (after merge), clean up
git worktree remove ../rust-project-<agent>-<task>
```

If worktrees are unavailable or impractical, a regular branch is acceptable — but the agent must not have uncommitted changes on another branch while switching.

### Commit Message Format

Every commit uses this template:

```
<type>(<scope>): <summary>

WHY: <1-2 sentences explaining the motivation and intent>

HOW: <1-2 sentences explaining the approach and key decisions>

SCOPE: <list of files/modules touched>
```

Example:

```
feat(analysis): add cyclomatic complexity scoring

WHY: PR review reports lack quantitative complexity metrics, making it
hard to flag overly complex functions during review.

HOW: Added ComplexityAnalyzer that counts branch points per function
in diff hunks. Scores map to Low/Medium/High/Critical thresholds
defined in config. Integrated into the analysis pipeline after
style checks.

SCOPE: src/analysis/complexity.rs, src/analysis/mod.rs, src/config.rs
```

---

## Review Process

### Roles

| Role | Responsibility |
|------|---------------|
| **Author** | Implements the change, writes commits, requests review |
| **Reviewer** | Evaluates the diff against intent, architecture, and quality criteria |

The same Claude instance performs both roles **in distinct phases**. The author phase ends with a review request. The reviewer phase is a separate evaluation with a different lens.

### Review Request

When the author considers a branch ready, it produces a **review request** as a structured summary:

```markdown
## Review Request: <branch-name>

### Intent
<What this change does and why>

### Changes
<File-by-file summary of what changed>

### Testing
<What was tested and how>

### Risks
<Known risks, trade-offs, or areas of uncertainty>

### Diff
<Output of: git diff main..<branch>>
```

### Review Criteria

The reviewer evaluates against this checklist:

1. **Intent alignment** — Does the code do what the commit message and review request claim?
2. **Architecture conformance** — Does it follow patterns established in the codebase and documented in ADRs?
3. **Scope discipline** — Is the change focused? No unrelated modifications, no premature abstractions?
4. **Correctness** — Are there logic errors, off-by-one bugs, missing edge cases at system boundaries?
5. **Security** — No injection vectors, no hardcoded secrets, no unsafe code without justification?
6. **Contract preservation** — Do shared types and interfaces remain compatible with other agents' modules?
7. **Test coverage** — Are new behaviors tested? Do existing tests still pass?
8. **Commit message accuracy** — Does the WHY/HOW/SCOPE match the actual diff?

### Review Outcomes

| Outcome | Action |
|---------|--------|
| **Approve** | Squash/rebase onto `main`, delete branch |
| **Request Changes** | Author receives specific feedback with file:line references, iterates on the same branch |
| **Reject** | Branch is abandoned; a comment is added to the branch's last commit explaining why |

### Review Record

Every review is recorded in a commit message or a file. At minimum, the merge commit onto `main` includes:

```
Reviewed-by: Claude
Review-result: Approved
Review-notes: <1-2 sentences summarizing what was verified>
```

---

## Merge Strategy

- **Squash and rebase** onto `main`. No merge commits.
- The squashed commit message follows the commit format above, with review metadata appended.
- After merge, delete the feature branch and clean up the worktree.

```bash
# On the feature branch
git rebase -i main    # squash into one commit
git checkout main
git merge --ff-only <branch>
git branch -d <branch>
```

---

## Conflict Resolution

If `main` has moved since the branch was created:

1. Rebase the feature branch onto current `main`.
2. Resolve conflicts preserving the intent documented in the commit message.
3. If the rebase changes behavior, re-run tests and re-request review.

---

## Architecture Decision Records (ADRs)

Significant architectural decisions are recorded in `docs/adr/` so the reviewer has persistent context across sessions.

### ADR Format

```markdown
# ADR-<NNN>: <Title>

## Status
Accepted | Superseded by ADR-XXX | Deprecated

## Context
<What problem or question prompted this decision>

## Decision
<What we decided and why>

## Consequences
<What follows from this decision — trade-offs, constraints, future implications>
```

### When to Write an ADR

- Choosing between two valid architectural approaches
- Introducing a new dependency or pattern
- Changing a shared interface or type that affects multiple modules
- Any decision the reviewer would need context for in a future session

---

## Handoff Protocol

When an agent needs changes in another agent's owned modules:

1. Write the request in the appropriate handoff file (`claude/handoff.md` or `codex/handoff.md`).
2. Include: **Module**, **What I need**, **Why**, **Status** (OPEN).
3. The owning agent picks up OPEN requests, implements them on their own branch, and follows the full review cycle.
4. Once merged, the requesting agent updates the handoff entry to RESOLVED.

---

## Responses

- Be concise and collaborative. Summarize what changed and why; include next steps if obvious.
- Use required fleet file links for paths and standard web markdown for URLs.
- Avoid dumping large file contents.
