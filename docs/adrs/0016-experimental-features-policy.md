# ADR-0016: Experimental Features Policy and Gating

## Status

Accepted

## Date

2025-01-31T00:21:00Z

## Context

Certain capabilities are not yet considered stable and require explicit opt-in. The CLI exposes an `--experimental` flag and command-specific guards to prevent accidental usage of unstable features.

## Decision

- Experimental features MUST require the top-level `--experimental` flag.
- Commands guarded as experimental MUST be rejected when the flag is not present.
- Help text MUST clearly mark experimental commands and flags.

## Mechanisms

- Privilege level is captured at parse time (`Privilege::{Normal, Experimental}`).
- Validation step enforces experimental gating in `CallArgs::validate()` by rejecting specific commands when privilege is `Normal`.
// Note: The `diff` capability is stable and MUST NOT be gated behind `--experimental`.

## Consequences

### Positive
- Prevents accidental adoption of unstable features
- Clear user intent and auditability in CI
- Enables iteration without breaking stable workflows

### Negative
- Slightly more complex CLI UX
- Requires maintaining the guarded list until stabilization

## Implementation

- Parse `--experimental` at root command.
- In `CallArgs::validate()`, return an error if a guarded command/flag is used without experimental privileges.
- Docstrings and README MUST denote experimental commands explicitly.

## References

- `args::{Privilege, CallArgs::validate}`
- `docs/README.md` diff command usage
