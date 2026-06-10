# Changelog

All notable changes to this project will be documented in this file.
Format: [keepachangelog.com](https://keepachangelog.com) · Versioning: [semver.org](https://semver.org)

---

## [Unreleased]

### Added
- (nothing yet)

---

## [0.1.0] - 2026-06-10

### Added

- **TUI mode**: Full-screen interactive browser with 9 screens:
  - `Home` — category panel + welcome message
  - `CategoryList` — all 20 browsable categories
  - `MovieList` — scrollable title list with year, rating, and content type
  - `MovieDetail` — complete metadata view: rating, votes, runtime, genres, directors, cast (top 5), full plot, tagline, budget, gross, keywords, languages, countries, awards, release date, IMDb URL
  - `StreamSelect` — quality/torrent link chooser from playimdb.com
  - `DownloadProgress` — live progress bar with speed and estimated size
  - `Search` — live full-text search via TMDB `/search/multi`
  - `Help` — all key bindings reference
  - `Setup` — first-run wizard for TMDB API key configuration

- **CLI mode** with 6 subcommands:
  - `search <query> [--limit N]` — keyword search with tabular output
  - `info <id|title>` — full metadata for any IMDb tt-ID or title name
  - `play <id|title>` — stream directly into the configured media player
  - `download <id|title> [--output DIR] [--player]` — download with progress or stream; shows file size via HTTP HEAD before downloading
  - `top [--category CAT] [--limit N]` — browse charts and genres
  - `config show|set-tmdb-key|set-download-dir|set-player|path` — persistent config management

- **TMDB API integration** (`src/imdb/mod.rs`):
  - Full movie and TV detail fetch with credits, external IDs, and keywords
  - Genre-filtered discover endpoints for all 20 categories
  - IMDb tt-ID → TMDB ID resolution via `/find` endpoint
  - Poster image download from TMDB CDN

- **playimdb.com integration** (`src/playimdb/mod.rs`):
  - Stream quality links (1080p, 720p, 480p, HD, BluRay, WEB, .mp4/.mkv)
  - Magnet link extraction with seeder count and file size (via regex)
  - Torrent file link extraction
  - Embedded player iframe detection
  - Title search fallback when direct ID lookup returns no results
  - Browser fallback as last resort

- **Kitty terminal image protocol** (`src/kitty/mod.rs`):
  - Auto-detection via `$TERM`/`$KITTY_WINDOW_ID`
  - Inline poster rendering on the movie detail screen
  - In-memory poster cache to avoid re-fetching
  - Graceful no-op in non-Kitty terminals

- **Async streaming downloader** (`src/downloader/mod.rs`):
  - Progress callback (bytes downloaded, total size, speed)
  - Pre-download size probe via HTTP HEAD
  - Filename sanitization
  - Human-readable speed formatting (B/s, KB/s, MB/s)

- **Media player auto-detection** (`src/player/mod.rs`):
  - Probes PATH for: `mpv`, `vlc`, `mplayer`, `celluloid`, `totem`, `smplayer`
  - Configurable extra arguments
  - Build-command-string helper for display

- **TOML configuration** (`src/config/mod.rs`):
  - Auto-created at `~/.config/watchie/config.toml` on first run
  - Sections: `download_dir`, `[player]`, `[ui]`, `[network]`, `[api]`
  - TMDB key via config or `TMDB_API_KEY` environment variable

- **Repository structure** following the FMG Repository Development Bible:
  - `README.md` — installation, configuration, full usage reference
  - `CHANGELOG.md` — release history (this file)
  - `VERSION` — single-source version string
  - `docs/wiki/architecture.md` — module reference, system diagram, 5 ADRs
  - `docs/wiki/development.md` — setup guide, how-to sections, dependency table
  - `docs/wiki/hygiene.md` — code standards, Conventional Commits, branch/release workflow
  - `docs/wiki/agent-sop.md` — AI agent operating procedures
  - `docs/wiki/index.md` — documentation index

### Changed

- Migrated all metadata retrieval from direct IMDb scraping (blocked by Cloudflare) to the official TMDB REST API v3.
- Unified TMDB key resolution: environment variable `TMDB_API_KEY` takes priority over config file value.
- Upgraded status bar renderer to display specific error messages instead of a generic fallback string.
- Streamlined player detection and Kitty detection helpers to eliminate module-level duplication.

### Fixed

- Resolved all `dead_code`, `unused_variables`, `unused_imports`, and `unused_mut` compiler warnings across the entire crate.
- Fixed duplicate function definitions that caused compilation errors after the initial dead-code additions.
- Corrected `go_back()` navigation to always clear Kitty images before transitioning away from the detail screen.
