# Code Hygiene and Git Workflow

This document defines the coding standards, git discipline, and quality gates for the `watchie` project. These rules are mandatory for all contributors, human and AI.

---

## Rust Code Standards

### Formatting

All code **must** be formatted with `rustfmt` before committing:

```bash
cargo fmt
```

The project uses the default `rustfmt` configuration (no custom `rustfmt.toml`). CI will reject un-formatted code.

### Linting

All code **must** pass `clippy` with zero warnings:

```bash
cargo clippy -- -D warnings
```

Do not suppress clippy warnings with `#[allow(...)]` unless you add a comment explaining why the suppression is justified.

### Compiler Warnings

Zero warnings is a hard invariant. This includes:
- `dead_code`
- `unused_variables`
- `unused_imports`
- `unused_mut`

If a field or function is part of a planned feature, mark it with `#[allow(dead_code)]` and open a tracking issue. Do not merge dead code silently.

### Error Handling

- Use `anyhow::Result` for all functions that can fail.
- Use `anyhow::Context::context()` / `.with_context()` to add descriptive context to errors — never just `?` on its own for non-trivial operations.
- Never `unwrap()` in production code paths. Reserve `unwrap()` for tests and truly infallible operations with a comment.
- Use `thiserror` for typed errors in library-style modules where callers need to match on error variants.

### Async Code

- All async functions must be on the `tokio` runtime.
- Never use `std::thread::sleep` in async code; use `tokio::time::sleep`.
- Never block the TUI event thread with synchronous I/O. Use `tokio::task::spawn_blocking` for blocking syscalls.

### Documentation

- All public functions, structs, and enums must have a `///` doc comment.
- Non-obvious code blocks must have inline `//` comments explaining the *why*, not just the *what*.
- Preserve all existing comments during edits unless they are factually incorrect.

---

## Conventional Commits

All commit messages **must** follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### Types

| Type | When to use |
|------|-------------|
| `feat` | A new user-facing feature |
| `fix` | A bug fix |
| `docs` | Documentation-only change |
| `style` | Formatting, whitespace, no logic change |
| `refactor` | Code restructuring with no feature change |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `chore` | Build system, dependency updates, tooling |
| `ci` | CI/CD pipeline changes |

### Scopes (optional but recommended)

| Scope | Module |
|-------|--------|
| `tui` | `src/tui/` |
| `cli` | `src/cli/` |
| `imdb` | `src/imdb/` |
| `playimdb` | `src/playimdb/` |
| `player` | `src/player/` |
| `downloader` | `src/downloader/` |
| `kitty` | `src/kitty/` |
| `config` | `src/config/` |
| `docs` | `docs/` or `README.md` |

### Examples

```
feat(tui): add scrollbar to movie detail screen
fix(playimdb): handle 404 response gracefully on title lookup
docs(wiki): update architecture ADR for TMDB migration
chore: bump reqwest to 0.12.5
refactor(imdb): extract genre ID map to a const table
```

---

## Branch Workflow

| Branch | Purpose | Rules |
|--------|---------|-------|
| `main` | Production, always releasable | Linear history only. No direct commits. |
| `feat/*` | New features | Branch from `main`. PR to `main`. |
| `fix/*` | Bug fixes | Branch from `main`. PR to `main`. |
| `docs/*` | Documentation updates | Branch from `main`. PR to `main`. |
| `chore/*` | Maintenance, deps | Branch from `main`. PR to `main`. |

### Naming Convention

```
feat/kitty-poster-preview
fix/playimdb-magnet-parser
docs/update-architecture-adr
chore/bump-ratatui-0.28
```

### Merge Policy

- All PRs require at least one passing `cargo build` (CI gate).
- Prefer `git merge --ff-only` for a clean linear history.
- **Banned:** `git push --force` to `main` under any circumstances.
- **Banned:** Merge commits on `main` — use rebase if needed.

---

## CHANGELOG Maintenance

The `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [Unreleased]

### Added
- ...

### Changed
- ...

### Fixed
- ...

## [X.Y.Z] - YYYY-MM-DD
```

Rules:
- Every PR that changes behavior, API, or fixes a bug must update `CHANGELOG.md`.
- Documentation-only PRs do not require a changelog entry.
- The `[Unreleased]` section is promoted to a version number at release time.

---

## Release Procedure

1. Verify `cargo build --release` succeeds with zero warnings.
2. Promote `[Unreleased]` in `CHANGELOG.md` to the new version number.
3. Update `VERSION` file.
4. Commit: `chore(release): v0.2.0`
5. Tag: `git tag -a v0.2.0 -m "Release v0.2.0"`
6. Push tag: `git push origin v0.2.0`

---

## What is Banned

| Action | Why |
|--------|-----|
| `git push --force` to `main` | History rewrite breaks collaborators |
| Committing `target/` directory | Binary artifacts, massive size |
| Committing `config.toml` with real API keys | Secret exposure |
| `unwrap()` in non-test production paths | Panics are user-hostile |
| Silencing clippy without a comment | Hides real bugs |
| Leaving `TODO:` comments without an issue link | Creates untracked debt |
