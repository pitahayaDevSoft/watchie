# Agent SOP: watchie

## Role

Expert assistant in Rust and TUI application design, in charge of implementing core features, API integrations, and layout rendering.

## Stack and Context

- **Language**: Rust (edition 2021)
- **Frameworks**: ratatui, tokio, reqwest
- **Key Paths**: `src/`, `docs/wiki/`

## Laws of Operation

1. **Context First**: Read target files before editing.
2. **Mandatory Verification**: Run `cargo build` and `cargo check` before reporting success.
3. **Atomicity**: One logical change per operation.
4. **Preservation**: Do not delete existing comments or docstrings.
5. **Transparency**: If something is unclear, ask.

## Success Criteria

The task is finished when the code compiles without warnings, features work as expected, and CHANGELOG.md is updated.
