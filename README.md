# watchie

> Terminal-native TUI/CLI client for browsing, streaming, and downloading movies and series via TMDB and playimdb.com.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](VERSION)

## Overview

`watchie` is a keyboard-driven terminal application written in Rust that provides a full movie and series catalog browser backed by the [TMDB API](https://www.themoviedb.org/), with integrated streaming and download capabilities via [playimdb.com](https://playimdb.com).

It runs in two modes:
- **TUI** (default) — interactive full-screen browser with category navigation, search, detail view, stream selection, and download progress display.
- **CLI** — scriptable subcommands for searching, fetching info, playing, and downloading titles directly from the shell.

When running inside [Kitty terminal](https://sw.kovidgoyal.net/kitty/), movie poster images are rendered inline using the Kitty Graphics Protocol.

## Installation

**Requirements:** Rust stable toolchain (edition 2021), Cargo.

```bash
# Clone
git clone https://github.com/pitahayaDevSoft/watchie
cd watchie

# Build optimized release binary
cargo build --release

# Install to $PATH
cargo install --path .
```

## Configuration

watchie auto-creates its config file at `~/.config/watchie/config.toml` on first run.

### API Key (Required)

watchie uses the TMDB API for all catalog data. A **free** API key is required:

1. Create a free account at [themoviedb.org](https://www.themoviedb.org/)
2. Go to **Account Settings → API** and generate a key (choose "Developer")
3. Configure it via one of these methods:

```bash
# Option A — CLI command (persists to config.toml)
watchie config set-tmdb-key YOUR_KEY_HERE

# Option B — Environment variable (session only)
export TMDB_API_KEY=YOUR_KEY_HERE

# Option C — Edit config directly
$EDITOR ~/.config/watchie/config.toml
```

### Full config.toml reference

```toml
download_dir = "/home/user/Downloads/watchie"

[player]
command = "mpv"        # auto-detected: mpv, vlc, mplayer, celluloid, totem, smplayer
extra_args = []        # additional flags passed to the player

[ui]
page_size = 20         # items visible per page in lists
kitty_images = false   # auto-detected from $TERM / $KITTY_WINDOW_ID
theme = "dark"

[network]
timeout_secs = 15      # HTTP request timeout (0 = unlimited for downloads)
max_retries = 3
user_agent = "Mozilla/5.0 ..."

[api]
tmdb_key = ""          # set via `watchie config set-tmdb-key`
```

## Usage

### TUI Mode

Run with no arguments to open the interactive browser:

```bash
watchie
```

#### Key Bindings

| Key | Context | Action |
|-----|---------|--------|
| `j` / `↓` | Any list | Move selection down |
| `k` / `↑` | Any list | Move selection up |
| `PageDown` | Any list | Jump one page down |
| `PageUp` | Any list | Jump one page up |
| `g` / `Home` | Any list | Jump to first item |
| `G` / `End` | Any list | Jump to last item |
| `Enter` | Category list | Open category movie list |
| `Enter` | Movie list | Open movie detail view |
| `Enter` | Movie detail | Open stream source selection |
| `Enter` | Stream select | Play selected stream/quality |
| `Esc` / `b` | Any screen | Go back |
| `/` or `s` | Any screen | Open search input |
| `c` | Any screen | Browse category list |
| `r` | Movie list | Reload current category |
| `p` | Movie list/detail | Fetch streams and play immediately |
| `d` | Movie list/detail | Fetch streams and download |
| `w` | Movie detail | Load stream sources |
| `i` | Any screen | Toggle Kitty inline image preview |
| `?` / `F1` | Any screen | Open help screen |
| `q` | Any screen | Quit |
| `Ctrl+C` | Any screen | Quit |

#### Screens

- **Home** — Category panel + quick stats sidebar
- **Category List** — All 20 browsable categories
- **Movie List** — Scrollable list with title, year, rating, type
- **Movie Detail** — Full metadata: rating, votes, runtime, genres, directors, cast, plot, tagline, budget, gross, keywords, languages, countries
- **Stream Select** — Lists parsed quality options and magnet/torrent links from playimdb.com
- **Download Progress** — Live progress bar with speed and size display
- **Search** — Full-text search across movies and TV shows
- **Help** — All keybindings reference
- **Setup** — Shown on first run without API key; guides through TMDB registration

### CLI Mode

#### Search

```bash
watchie search "Blade Runner" --limit 10
# Output: table with TMDB internal ID, title, year, rating, content type
```

#### Info

```bash
# By IMDb tt-ID
watchie info tt0083658

# By title (resolves via search)
watchie info "Blade Runner"
```

Displays: title, year, rating, metascore, type, runtime, genres, director, cast, languages, countries, release date, full plot, keywords, IMDb URL.

#### Play

```bash
# Stream directly into media player
watchie play tt0816692
watchie play "Interstellar"
```

Auto-resolves IMDb ID → finds streams on playimdb.com → opens first quality in configured player.

#### Download

```bash
# Download to configured download directory
watchie download tt0816692

# Custom output directory
watchie dl "Dune" --output ~/Videos/

# Stream instead of download
watchie download tt0816692 --player
```

Shows file size preview (via HTTP HEAD) before downloading. Progress printed to stdout.

#### Browse Top Charts

```bash
watchie top                         # Popular Movies (default)
watchie top --category top          # Top Rated Movies
watchie top --category toptv        # Top Rated TV
watchie top --category action       # Action genre
watchie top --category horror --limit 30
```

#### Config Management

```bash
watchie config show                          # Print current config as TOML
watchie config set-tmdb-key <KEY>           # Save TMDB API key
watchie config set-download-dir ~/Movies    # Set download directory
watchie config set-player vlc               # Set media player
watchie config path                         # Print config file path
```

## Architecture

See [docs/wiki/architecture.md](docs/wiki/architecture.md) for the full system design and ADRs.

```
src/
├── main.rs              — Entrypoint: dispatches TUI or CLI mode
├── cli/
│   ├── mod.rs           — Clap subcommand definitions
│   └── commands.rs      — CLI command implementations
├── config/mod.rs        — TOML config model, load/save, auto-detection
├── imdb/mod.rs          — TMDB API client (search, category, detail, poster)
├── playimdb/mod.rs      — playimdb.com scraper (stream info, torrent links)
├── player/mod.rs        — Media player subprocess launcher
├── downloader/mod.rs    — Async HTTP downloader with progress callbacks
├── kitty/mod.rs         — Kitty Graphics Protocol renderer
└── tui/
    ├── mod.rs           — Terminal setup/teardown
    ├── app.rs           — Application state machine
    ├── events.rs        — Event loop, keyboard handling, async task dispatch
    ├── render.rs        — All screen drawing functions (ratatui)
    └── widgets.rs       — Custom widget helpers
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

MIT — see [LICENSE](LICENSE).
