# ADR-0019: CLI Diagnostics (Stdout/Stderr), Prompts, and Exit Codes

## Status

Accepted

## Date

2025-01-31T01:00:00Z

## Context

`qop` is a CLI tool used both interactively by humans and non-interactively in CI. Existing ADRs cover error handling (anyhow), output formatting (human vs JSON), safety modes (confirmations, diff, dry-run), and command structure. However, there is no dedicated policy for diagnostics streams and exit codes. Today, messages are printed via `println!`, sometimes interleaving user prompts, progress notes, and structured output. This can make programmatic consumption brittle if stderr/stdout separation is unclear and if exit codes are inconsistent.

We need explicit guidelines to:
- Prevent JSON output from being contaminated by human diagnostics
- Ensure predictable exit codes for scripting/CI
- Clarify interaction rules for prompts and non-interactive operation
- Keep behavior consistent across subsystems

## Decision

1. Standard Streams
   - Human-facing progress messages, warnings, and prompts MUST be written to stderr.
   - Machine-readable output (e.g., JSON) MUST be written to stdout exclusively.
   - Human tabular output SHOULD be written to stdout, but MUST NOT be mixed with JSON in the same invocation.

2. Output Modes
   - When `--output json` (or equivalent) is selected, commands MUST emit only valid JSON to stdout and MUST redirect all diagnostics to stderr.
   - When `--output human`, commands MAY render tables and progress lines to stdout; errors and warnings still go to stderr.

3. Prompts and Non-Interactive Behavior
   - Interactive prompts (confirmations, diff prompts) MUST be written to stderr and read from stdin.
   - `--yes` MUST suppress prompts. In CI, callers SHOULD pass `--yes` to avoid blocking.
   - When `--diff` is used non-interactively without `--yes`, the tool MUST still prompt on stderr unless `--yes` is provided; if stdin is not a TTY and `--yes` is absent, the command MUST fail with a non-zero code rather than hang.

4. Exit Codes
   - 0: Success (operation completed as requested; no pending work is not an error).
   - 1: Generic error (I/O, parsing, DB, validation, runtime failure).
   - 2: Usage error (invalid flags/args, incompatible command combination).
   - 3: Unsafe operation blocked (e.g., locked migration without `--unlock`, non-linear history refused when user declined).
   - 4: Non-interactive prompt required but not allowed (stdin non-TTY without `--yes`).
   - Future codes MAY be added sparingly; they MUST be documented.

5. Consistency Across Subsystems
   - Postgres and SQLite subsystems MUST adhere to the same stream separation and exit code policy.
   - Shared confirmation and diff helpers in `core` MUST implement the stderr/stdout policy centrally.

## Consequences

### Positive
- Predictable automation: JSON on stdout only; diagnostics on stderr
- Clear, stable exit codes for CI pipelines and scripts
- Reduced risk of breaking parsers with mixed output
- Centralized behavior across subsystems

### Negative
- Requires touching many `println!` call sites to re-route messages to stderr where appropriate
- Slight complexity to maintain stream discipline

## Implementation

1. Stream Discipline
   - Replace user-facing progress `println!` with `eprintln!` where the message is not part of the primary data output.
   - In JSON mode, ensure stdout emits only JSON; move all other messages to stderr.

2. Core Helpers
   - `core::migration::prompt_for_confirmation_with_diff`:
     - Write prompts and helper lines to stderr; diff display may go to stdout only when `--output human` and not in JSON mode; otherwise send to stderr.
   - `core::migration::display_sql_migration`:
     - Accept a target stream parameter or provide two variants to support stderr in interactive flows and stdout in explicit human mode.

3. Exit Code Mapping
   - `main` continues returning `anyhow::Result<()>`.
   - Top-level execution wrapper MUST map error categories to exit codes:
     - Argument/parse errors → 2
     - User-declined or safety refusal → 3
     - Non-interactive prompt required → 4
     - All other errors → 1

4. Subsystems
   - Update `subsystem::<backend>::migration` and `repo` modules to route progress/warnings to stderr.
   - Ensure `list --output json` produces clean JSON with no extraneous stdout lines.

5. Backward Compatibility
   - Human mode retains current look-and-feel; only stream destinations change.
   - Document the exit code policy in `docs/README.md` and manpages.

## References

- ADR-0004: Error Handling with anyhow
- ADR-0012: Migration Safety Modes
- ADR-0018: Output Formatting (Human vs JSON)
- Clap documentation for non-interactive patterns

