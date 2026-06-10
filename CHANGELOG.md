# Changelog

All notable changes to this project will be documented in this file.
Format: [keepachangelog.com](https://keepachangelog.com) · Versioning: [semver.org](https://semver.org)

## [0.1.0] - 2026-06-10

### Added
- Repository structure following the FMG Repository Development Bible.
- Setup / API Key configuration wizard screen in the TUI when no TMDB key is configured.
- Fallback search-by-title query to playimdb.com if direct IMDb ID lookup yields no results.
- Command-line logging of the full player launch command string in CLI and TUI.
- Kitty graphics poster preview rendering inside the TUI movie detail page.

### Changed
- Migrated metadata retrieval from raw IMDb scraping to the official TMDB API to bypass Cloudflare bot protection.
- Streamlined media player and Kitty terminal detection helpers across modules to eliminate duplication.
- Upgraded the status bar error renderer to output specific error reasons instead of a generic string.

### Fixed
- Fixed all compiler warnings, unused fields, and dead-code warnings across the entire watchie crate.
