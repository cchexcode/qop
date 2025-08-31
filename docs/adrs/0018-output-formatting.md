# ADR-0018: Output Formatting (Human vs JSON)

## Status

Accepted

## Date

2025-01-31T00:27:00Z

## Context

The CLI caters to both human operators and automation. Human-readable tabular output is preferred for terminals, while JSON output is required for programmatic consumption and CI pipelines.

## Decision

- Commands that list or summarize migrations MUST support `human` and `json` output modes.
- Human output SHOULD be a compact table rendering and use symbols for quick scanning.
- JSON output MUST be a stable, documented schema suitable for parsing.

## Consequences

### Positive
- Good UX for humans and machines
- Enables CI checks and dashboards via JSON

### Negative
- Requires maintaining multiple renderers

## Implementation

- Enum `OutputFormat::{Human, Json}` drives rendering.
- Human:
  - Render via `comfy_table` with headers: Migration ID, Remote, Local, Comment, Locked
  - Use symbols: "✅"/"❌" for booleans, "🔒" for locked
- JSON:
  - Emit array of objects: `{ id, remote: Option<DateTime<Utc>>, local: bool, comment: Option<String>, locked: bool }`

## References

- `core::service::list` human and json branches
- `subsystem::<backend>::commands::Output`
