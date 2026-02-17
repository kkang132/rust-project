# Decision Cost Tracking

Tracks the wall-clock time and API dollar cost of each architectural decision. Complements [decisions.md](decisions.md) with the operational dimension.

## How to Use

After making or revisiting a decision, add a row below. For $ cost, check the Claude/Codex session or API dashboard after the work completes. Times use `s` (seconds) and `m` (minutes).

## Cost Legend

| Column | Description |
|--------|-------------|
| **ADR** | Reference to the ADR in decisions.md |
| **Decision** | Short name |
| **Wall Time** | Approximate end-to-end time from start to verified |
| **Claude $** | Claude API cost for this decision (eval + impl + validation) |
| **Codex $** | Codex API cost for this decision |
| **Total $** | Claude $ + Codex $ |
| **Complexity Cost** | Subjective: Low / Medium / High — ongoing maintenance burden |
| **Notes** | Context |

## Decision Cost Log

| ADR | Decision | Wall Time | Claude $ | Codex $ | Total $ | Complexity Cost | Notes |
|-----|----------|-----------|----------|---------|---------|-----------------|-------|
| 001 | Two-Agent Development Model | ~30s | — | — | — | Low | Pre-implementation design; no API calls |
| 002 | Trait-Based Analyzer Architecture | ~2m | — | — | — | Low | Scaffolding only; trait + tokio::join! |
| 003 | Per-Module Error Types | ~1m | — | — | — | Low | thiserror boilerplate |
| 004 | Optional Configuration | ~1m | — | — | — | Low | TOML + env var; standard pattern |
| 005 | Risk Aggregation via Maximum | ~15s | — | — | — | Low | One-liner: max() on an enum |
| 006 | Unified Diff Parsing | ~5m | — | — | — | Medium | Custom parser ~200 lines; edge cases |
| 007 | Trait Signature & Naming Fix | ~15s | — | — | — | Low | Spec correction |
| 008 | Tracing & Observability | ~3m | — | — | — | Low | tracing + instrument macros |

> **—** = Cost not tracked. ADRs 001–008 were implemented before $ tracking was added.
> Going forward, record the cost shown in the Claude Code / Codex session summary after each decision.

## How to Get $ Cost

**Claude Code:** After a session, check the token usage summary. Approximate cost:
- Input: ~$15/M tokens (Opus), ~$3/M tokens (Sonnet/Haiku)
- Output: ~$75/M tokens (Opus), ~$15/M tokens (Sonnet/Haiku)

**Codex:** Check the API usage dashboard or session cost reported by the Codex CLI.

Record the per-session cost attributed to the decision. If a session covers multiple decisions, split proportionally or note "shared session" in Notes.

## Cumulative Stats

| Metric | Value |
|--------|-------|
| **Total decisions** | 8 |
| **Total tracked $** | $0 (tracking starts with ADR-009+) |
| **Decisions with rework** | 0 |
| **High complexity cost** | 0 |
| **Medium complexity cost** | 1 (ADR-006) |

---

*Update cumulative stats when adding new rows.*
