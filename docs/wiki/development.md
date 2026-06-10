# Development Guide

This document covers everything you need to set up a local development environment, understand the testing strategy, add new features, and submit changes to `watchie`.

---

## Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust | stable (2021 edition) | `curl https://sh.rustup.rs -sSf \| sh` |
| Cargo | (bundled with Rust) | — |
| TMDB API key | free | [themoviedb.org/settings/api](https://www.themoviedb.org/settings/api) |
| A video player | any | `mpv` recommended |

Optional but recommended for Kitty image previews:

| Tool | Why |
|------|-----|
| [Kitty terminal](https://sw.kovidgoyal.net/kitty/) | Inline poster image support |

---

## Quick Start

```bash
# Clone
git clone https://github.com/pitahayaDevSoft/watchie
cd watchie

# Set your TMDB API key (persisted to config)
cargo run -- config set-tmdb-key YOUR_KEY

# Run TUI in development mode
cargo run

# Or run a CLI command
cargo run -- search "Blade Runner"
```

---

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `TMDB_API_KEY` | TMDB API key — takes priority over `config.toml` value |
| `RUST_LOG` | Log level filter (e.g. `watchie=debug`, `watchie=trace`) |
| `KITTY_WINDOW_ID` | Set automatically by Kitty; enables image protocol |
| `TERM` | Must be `xterm-kitty` for Kitty image auto-detection |

Enable debug logging:

```bash
RUST_LOG=watchie=debug cargo run -- search "test"
```

---

## Project Layout

```
watchie/
├── src/
│   ├── main.rs              # Entry point, dispatch logic
│   ├── cli/
│   │   ├── mod.rs           # Clap subcommand definitions (Cli, Commands, ConfigAction)
│   │   └── commands.rs      # CLI command implementations
│   ├── config/
│   │   └── mod.rs           # Config struct, load/save, auto-detection
│   ├── imdb/
│   │   └── mod.rs           # TMDB API client + data models
│   ├── playimdb/
│   │   └── mod.rs           # playimdb.com HTML scraper
│   ├── player/
│   │   └── mod.rs           # Media player subprocess launcher
│   ├── downloader/
│   │   └── mod.rs           # Async streaming HTTP downloader
│   ├── kitty/
│   │   └── mod.rs           # Kitty terminal graphics protocol
│   └── tui/
│       ├── mod.rs           # Terminal setup/teardown
│       ├── app.rs           # Application state (App struct, Screen, navigation)
│       ├── events.rs        # Main event loop, keyboard dispatch
│       ├── render.rs        # All ratatui rendering functions
│       └── widgets.rs       # Custom widget helpers
├── docs/
│   └── wiki/
│       ├── architecture.md  # System design, ADRs
│       ├── development.md   # This file
│       ├── agent-sop.md     # AI agent standard operating procedures
│       └── hygiene.md       # Code quality, linting, formatting standards
├── Cargo.toml
├── Cargo.lock
├── README.md
├── CHANGELOG.md
├── VERSION
└── LICENSE
```

---

## Build Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Debug build |
| `cargo build --release` | Optimized release build (LTO, strip) |
| `cargo run` | Run TUI in debug mode |
| `cargo run -- <subcommand>` | Run a CLI subcommand |
| `cargo check` | Fast syntax/type check (no codegen) |
| `cargo clippy` | Run linter |
| `cargo fmt` | Format all code |
| `cargo test` | Run unit tests |
| `cargo install --path .` | Install binary to `~/.cargo/bin/` |

---

## Release Profile

The `[profile.release]` in `Cargo.toml` is configured for maximum binary size reduction and performance:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

This typically produces a binary under 10 MB.

---

## Adding a New CLI Subcommand

1. **Declare the variant** in `src/cli/mod.rs` inside the `Commands` enum:

```rust
/// Brief description for --help
#[command(name = "my-cmd", about = "Does something cool")]
MyCmd {
    #[arg(short, long)]
    my_flag: bool,
},
```

2. **Implement the handler** in `src/cli/commands.rs`:

```rust
pub async fn my_cmd(my_flag: bool, config: &Config) -> Result<()> {
    // implementation
    Ok(())
}
```

3. **Dispatch it** in `src/main.rs`:

```rust
Some(Commands::MyCmd { my_flag }) => {
    cli::commands::my_cmd(my_flag, &config).await?;
}
```

---

## Adding a New TUI Screen

1. **Add the variant** to `Screen` enum in `src/tui/app.rs`.

2. **Handle navigation** in `app.go_back()` and in the event handlers in `src/tui/events.rs`.

3. **Add key handlers** in `events.rs` in the appropriate `match app.screen { … }` block.

4. **Implement the render function** in `src/tui/render.rs`:

```rust
fn draw_my_screen(f: &mut Frame, app: &App, area: Rect) {
    // Use the color constants (C_BG, C_ACCENT, etc.) from the top of the file
}
```

5. **Call it from `draw_body`** by adding a match arm:

```rust
Screen::MyScreen => draw_my_screen(f, app, area),
```

6. **Update the header breadcrumb** in `draw_header`:

```rust
Screen::MyScreen => " My Screen".to_string(),
```

---

## Adding a New Category

Categories are a static slice in `src/imdb/mod.rs`:

```rust
pub const CATEGORIES: &[Category] = &[
    // ... existing entries ...
    Category { name: "My New Category", id: "my-category-id" },
];
```

Then handle the new `id` in `ImdbClient::get_category`:

```rust
"my-category-id" => (
    format!("https://api.themoviedb.org/3/…?api_key={}", key),
    true // is_movie
),
```

---

## TMDB API Reference

Endpoints used by `ImdbClient`:

| Endpoint | Usage |
|----------|-------|
| `GET /3/search/multi` | Keyword search (movies + TV) |
| `GET /3/find/{id}?external_source=imdb_id` | Resolve IMDb tt-ID to TMDB ID |
| `GET /3/movie/{id}?append_to_response=credits,external_ids,keywords` | Movie detail |
| `GET /3/tv/{id}?append_to_response=credits,external_ids,keywords` | TV detail |
| `GET /3/movie/popular` | Popular movies |
| `GET /3/movie/top_rated` | Top rated movies |
| `GET /3/tv/top_rated` | Top rated TV |
| `GET /3/movie/upcoming` | Coming soon |
| `GET /3/trending/movie/week` | Box office / trending |
| `GET /3/discover/movie?with_genres={id}` | Genre-filtered lists |
| TMDB image CDN: `https://image.tmdb.org/t/p/w500{path}` | Poster images |

Full API docs: [developers.themoviedb.org](https://developer.themoviedb.org/docs)

---

## Kitty Image Protocol Notes

The `kitty/mod.rs` module implements the protocol from scratch. Key points:

- Images are transmitted as base64-encoded PNG data in 4096-byte APC chunks.
- The `f=100` format field indicates PNG.
- `a=T` on the first chunk means "transmit and display".
- `m=1` means "more chunks follow"; `m=0` means "last chunk".
- `c`/`r` fields specify cell width and height (0 = auto-size).
- `a=d` deletes all images.

If you change the image rendering logic, test in a real Kitty terminal. Xterm, Alacritty, and others will silently ignore the APC sequences.

---

## Tracing / Logging

watchie uses `tracing` + `tracing-subscriber`. The default filter is `watchie=info`.

To log at a different level:

```bash
RUST_LOG=watchie=debug cargo run
RUST_LOG=watchie=trace cargo run -- search "test"
```

Tracing output goes to stderr. In TUI mode it is hidden behind the alternate screen; use `RUST_LOG` output redirection for debugging:

```bash
RUST_LOG=watchie=debug cargo run 2>watchie.log
```

---

## Dependency Overview

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | Async runtime |
| `clap` | 4 | CLI argument parsing |
| `ratatui` | 0.28 | TUI rendering |
| `crossterm` | 0.28 | Cross-platform terminal control |
| `reqwest` | 0.12 | HTTP client (async + blocking) |
| `scraper` | 0.21 | HTML parsing for playimdb scraper |
| `serde` | 1 | Serialization/deserialization |
| `serde_json` | 1 | JSON parsing for TMDB responses |
| `toml` | 0.8 | Config file serialization |
| `dirs` | 5 | Platform config/download directories |
| `anyhow` | 1 | Error handling |
| `thiserror` | 1 | Custom error types |
| `unicode-width` | 0.1 | Correct terminal column widths |
| `fuzzy-matcher` | 0.3 | Fuzzy matching (future use) |
| `open` | 5 | Open URLs/files in system default app |
| `which` | 6 | Player binary detection |
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log formatting + env filter |
| `futures` | 0.3 | Stream utilities |
| `chrono` | 0.4 | Date/time (serde support) |
| `url` | 2 | URL parsing and manipulation |
| `indicatif` | 0.17 | Progress bars (CLI) |
| `regex` | 1 | Size/seeder extraction from HTML |
| `base64` | 0.22 | Image encoding for Kitty protocol |
| `image` | 0.25 | Image decoding and resizing |
| `urlencoding` | 2 | Query parameter encoding |

---

## Common Issues

### `TMDB API key is missing`

Run `watchie config set-tmdb-key YOUR_KEY` or set `TMDB_API_KEY=YOUR_KEY` in your environment.

### `No streams found`

playimdb.com availability varies by title. The fallback chain tries title search, then opens the browser. If even the browser shows nothing, the title may not be indexed there.

### Kitty images not showing

Check that `$TERM` is `xterm-kitty` or that `$KITTY_WINDOW_ID` is set. You can force-enable with `watchie config show` and editing `kitty_images = true` in the config file.

### Player not found

Set your player: `watchie config set-player mpv`. Make sure the binary is in `$PATH`.

### Build fails on `image` crate

The `image` crate links against system JPEG/PNG libraries on some platforms. Install `libjpeg-dev` and `libpng-dev` if needed.
