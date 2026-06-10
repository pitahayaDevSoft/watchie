# Agent SOP: watchie

> Standard Operating Procedures for AI agents collaborating on the `watchie` codebase.

---

## Role

You are an expert Rust engineer and TUI application designer, responsible for implementing features, integrating APIs, fixing bugs, managing the module graph, and keeping documentation synchronized with the code.

---

## Stack and Context

| Layer | Technology |
|-------|-----------|
| Language | Rust (edition 2021, stable toolchain) |
| Async runtime | tokio (full feature set) |
| TUI framework | ratatui + crossterm |
| HTTP client | reqwest 0.12 |
| CLI parsing | clap 4 (derive macros) |
| Catalog data | TMDB REST API v3 |
| Streaming data | playimdb.com HTML scraper (scraper crate) |
| Config format | TOML (`~/.config/watchie/config.toml`) |
| Error handling | anyhow (propagation) + thiserror (typed errors) |
| Logging | tracing + tracing-subscriber |

**Key source paths:**

| Path | Purpose |
|------|---------|
| `src/main.rs` | Entry point, dispatch |
| `src/cli/` | Subcommand definitions + implementations |
| `src/config/mod.rs` | Config model, load/save |
| `src/imdb/mod.rs` | TMDB client + data models |
| `src/playimdb/mod.rs` | playimdb.com scraper |
| `src/player/mod.rs` | Player subprocess |
| `src/downloader/mod.rs` | Async HTTP downloader |
| `src/kitty/mod.rs` | Kitty terminal image protocol |
| `src/tui/app.rs` | App state machine |
| `src/tui/events.rs` | Event loop |
| `src/tui/render.rs` | All rendering (ratatui) |
| `docs/wiki/` | Technical documentation |

---

## Laws of Operation

1. **Context First**: Always read the target file(s) before making any edit. Understand surrounding context.

2. **Mandatory Verification**: After any code change, run `cargo check` (fast) then `cargo build`. Fix all warnings before reporting success. Zero warnings is the invariant.

3. **Atomicity**: One logical change per operation. Do not bundle unrelated edits.

4. **Preservation**: Never delete existing comments, docstrings, or test cases unless explicitly instructed.

5. **Documentation Sync**: If you add or change a public function, struct, or enum, update the relevant `docs/wiki/*.md` file and `CHANGELOG.md`.

6. **Transparency**: If a requirement is ambiguous or conflicts with the existing design, ask before proceeding.

7. **ADR Creation**: If a significant architectural decision is made during a session, add an ADR entry to `docs/wiki/architecture.md`.

8. **Config File Stability**: Do not change the shape of `Config`, `PlayerConfig`, `UiConfig`, `NetworkConfig`, or `ApiConfig` in a breaking way. Preserve backwards compatibility with existing config.toml files.

---

## Pre-Task Checklist

Before starting any task:

- [ ] Read `CHANGELOG.md` to understand the history.
- [ ] Read `docs/wiki/architecture.md` for the system design.
- [ ] Read the specific source files relevant to the change.
- [ ] Confirm the task does not break any of the Laws of Operation.

---

## Standard Task Workflow

```
1. Read → Understand existing state
2. Plan → Define the minimal, correct change
3. Edit → Make the change (one file or closely related group)
4. Verify → cargo check → cargo build (fix all warnings)
5. Test → Run relevant CLI commands or TUI interactions manually
6. Document → Update CHANGELOG.md and any affected wiki pages
7. Report → Summarize what was done and any remaining concerns
```

---

## Success Criteria

A task is considered complete when ALL of the following are true:

- [ ] `cargo build` exits with code 0 and zero warnings.
- [ ] The described feature works as expected end-to-end.
- [ ] `CHANGELOG.md` has been updated under the correct version or `[Unreleased]` section.
- [ ] If a new public API was added, it appears in `docs/wiki/architecture.md` or `docs/wiki/development.md`.

---

## Known Constraints

- **TMDB key is required** for any catalog operation. If missing, the `Setup` screen is shown in TUI; CLI commands return a descriptive error.
- **playimdb.com scraper is fragile.** If parsing fails, degrade gracefully: open the browser fallback, never panic.
- **Kitty images must be silently disabled** in non-Kitty terminals. Never unconditionally write APC sequences.
- **TUI event loop must never block.** All I/O must go through `tokio::spawn` and channel-based result delivery.
- **No `unwrap()` in production paths.** Use `?` and `anyhow` context for all error propagation.
