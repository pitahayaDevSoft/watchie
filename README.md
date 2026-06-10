# watchie

> Browse TMDB catalog, stream and download titles — all from your terminal.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

## Overview

`watchie` is a fast, keyboard-driven terminal application (TUI + CLI) written in Rust that allows you to browse, search, and view movie or series details via the TMDB API, and stream or download video files via playimdb.com integration.

### Features
- **TUI Browser**: Navigate categories (Popular, Top Rated, Genre filters) and search results with keybindings.
- **Rich Movie Details**: Display ratings, plot overview, cast, director, budget, keywords, and spoken languages.
- **Media Player Integration**: Auto-detects and opens streams in mpv, vlc, or custom player commands.
- **Download Manager**: Background asynchronous downloading with progress bars and pre-download size previews.
- **Kitty Previews**: Renders high-quality inline poster graphics natively when running in Kitty terminal.

## Installation

```bash
# Clone the repository
git clone https://github.com/user/watchie
cd watchie

# Build and install to path
cargo install --path .
```

## Usage

### Configuration Setup
`watchie` uses the TMDB API for catalog browsing and search. A free API key is required:
1. Register at [themoviedb.org](https://www.themoviedb.org/) and generate a free API key.
2. Save it using the config command:
   ```bash
   watchie config set-tmdb-key <YOUR_KEY>
   ```

### TUI Mode
Simply run the binary without arguments:
```bash
watchie
```
- Use `j` / `k` (or arrows) to move, `Enter` to select, `Esc` / `b` to go back, `/` to search, and `q` to quit.

### CLI Mode
For scripting or fast execution:
```bash
# Search for a title
watchie search "Inception" --limit 5

# View full details of a title (handles IMDb tt-ID or TMDB formats)
watchie info tt1375666

# Stream a title directly in your media player
watchie play tt1375666

# Download a title
watchie download "Dune" --output ~/Videos/
```

## Architecture

`watchie` is built in Rust using `ratatui` for TUI rendering and `tokio` for async networking.
- Metadata is resolved using the **TMDB API** (and mapped to IMDb `tt` IDs).
- Stream URLs are parsed from **playimdb.com** and fed to local player sub-processes.
- Terminal graphics use the **Kitty Graphics Protocol** via absolute terminal cell writes.

Detailed architecture and design logs are recorded in [docs/wiki/architecture.md](docs/wiki/architecture.md).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for full change history.

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
