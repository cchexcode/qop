# ADR-0017: Documentation and Shell Completions Generation

## Status

Accepted

## Date

2025-01-31T00:24:00Z

## Context

The project generates user-facing documentation and shell completions to improve UX and discoverability. Both manpage and Markdown formats are supported, as well as completion scripts for common shells.

## Decision

- The CLI MUST provide a `man` command to render documentation into a specified output directory in either `manpages` or `markdown` format.
- The CLI MUST provide an `autocomplete` command to generate completion scripts for `bash`, `zsh`, `fish`, `elvish`, and `powershell`.

## Consequences

### Positive
- Discoverable and up-to-date docs tied to the current binary
- Easy integration into packages and CI artifacts

### Negative
- Requires contributors to keep docs and command definitions in sync

## Implementation

- `qop man --out <PATH> --format <manpages|markdown>`
  - Uses `clap_mangen` and `clap-markdown` via `reference::build_manpages` and `reference::build_markdown`.
- `qop autocomplete --out <PATH> --shell <SHELL>`
  - Uses `clap_complete` via `reference::build_shell_completion`.
- The top-level clap command derives help from current command definitions ensuring docs match the binary.

## References

- `main.rs` dispatch for `man` and `autocomplete`
- `args::ClapArgumentLoader::root_command()`
- `reference::{build_manpages, build_markdown, build_shell_completion}`
