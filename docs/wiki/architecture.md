# Architecture

This document describes the internal design of `watchie`: its module layout, data flow, state machine, and the architectural decisions that shaped the current implementation.

---

## High-Level System Diagram

```
┌──────────────────────────────────────────────────────────┐
│                       main.rs                            │
│  ┌──────────────────────┐  ┌───────────────────────┐    │
│  │   No subcommand      │  │   Subcommand present  │    │
│  │   → tui::run()       │  │   → cli::commands::*  │    │
│  └──────────┬───────────┘  └──────────┬────────────┘    │
└─────────────┼────────────────────────┼─────────────────-─┘
              │                        │
        ┌─────▼───────┐         ┌──────▼────────┐
        │  tui/ layer │         │  cli/ layer   │
        │  (ratatui)  │         │  (clap args)  │
        └──────┬──────┘         └──────┬────────┘
               │                      │
    ┌──────────▼──────────────────────▼──────────┐
    │              Service layer                  │
    │  imdb/       playimdb/   player/  downloader│
    │  (TMDB API)  (scraper)  (mpv/vlc)  (reqwest)│
    └──────────────────────────────────────────────┘
    │              config/    kitty/               │
    │              (TOML)     (Kitty protocol)     │
    └──────────────────────────────────────────────┘
```

---

## Module Reference

### `main.rs`

The binary entry point. Responsibilities:
1. Initialize `tracing_subscriber` with `RUST_LOG` support and a default `watchie=info` filter.
2. Parse CLI arguments via `clap::Parser` from the `Cli` struct.
3. Load `Config` via `Config::load()` (creates default config if absent).
4. Dispatch: if no subcommand is given, call `tui::run(config)`; otherwise call the matching `cli::commands::*` function.

### `config/mod.rs`

Manages persistent configuration serialized to TOML.

**Config file location:** `~/.config/watchie/config.toml`

**Struct hierarchy:**

```
Config
├── download_dir: PathBuf
├── player: PlayerConfig
│   ├── command: String          (auto-detected player binary)
│   └── extra_args: Vec<String>
├── ui: UiConfig
│   ├── page_size: usize         (default: 20)
│   ├── kitty_images: bool       (auto-detected from $TERM)
│   └── theme: String            ("dark" | "light")
├── network: NetworkConfig
│   ├── timeout_secs: u64        (default: 15)
│   ├── max_retries: u8          (default: 3)
│   └── user_agent: String
└── api: ApiConfig
    └── tmdb_key: String
```

`Config::load()` reads the file, or writes out a freshly-generated default if none exists. `Config::save()` serializes via `toml::to_string_pretty`.

**Auto-detection at default construction:**
- `player.command`: calls `player::detect_best_player()` which probes `PATH` for `mpv`, `vlc`, `mplayer`, `celluloid`, `totem`, `smplayer` in order.
- `ui.kitty_images`: calls `kitty::is_kitty()` which checks `$TERM == "xterm-kitty"` or `$KITTY_WINDOW_ID` is set.
- `download_dir`: `dirs::download_dir()` + `/watchie` subdirectory.

### `imdb/mod.rs`

Houses the **data models** and the **TMDB API client** (`ImdbClient`). Despite the module name referring to "imdb" (for historical reasons), all network access is via the official TMDB REST API v3.

#### Data Models

| Type | Purpose |
|------|---------|
| `Movie` | Full metadata for a single title (23+ fields) |
| `SearchResult` | Lightweight item for list views |
| `Category` | Static name + ID pair |
| `ContentType` | `Movie`, `Series`, `MiniSeries`, `ShortFilm`, `Documentary`, `Unknown` |

**`Movie` fields:** `id`, `title`, `year`, `rating`, `votes`, `genres`, `runtime`, `plot`, `director`, `cast`, `poster_url`, `imdb_url`, `content_type`, `episodes`, `seasons`, `language`, `country`, `awards`, `budget`, `gross`, `release_date`, `metascore`, `tagline`, `keywords`.

#### `CATEGORIES` constant

A static slice of 20 `Category` entries covering:

| Category | TMDB Mapping |
|----------|-------------|
| Popular Movies | `movie/popular` |
| Top Rated Movies | `movie/top_rated` |
| Top Rated TV | `tv/top_rated` |
| Action | `discover/movie?with_genres=28` |
| Adventure | `discover/movie?with_genres=12` |
| Animation | `discover/movie?with_genres=16` |
| Comedy | `discover/movie?with_genres=35` |
| Crime | `discover/movie?with_genres=80` |
| Documentary | `discover/movie?with_genres=99` |
| Drama | `discover/movie?with_genres=18` |
| Fantasy | `discover/movie?with_genres=14` |
| Horror | `discover/movie?with_genres=27` |
| Mystery | `discover/movie?with_genres=9648` |
| Romance | `discover/movie?with_genres=10749` |
| Sci-Fi | `discover/movie?with_genres=878` |
| Thriller | `discover/movie?with_genres=53` |
| Western | `discover/movie?with_genres=37` |
| Box Office | `trending/movie/week` |
| Coming Soon | `movie/upcoming` |
| Award Winners | `discover/movie?sort_by=vote_count.desc` |

#### `ImdbClient` Methods

| Method | Description |
|--------|-------------|
| `new()` | Constructs client; reads TMDB key from `$TMDB_API_KEY` env var, falling back to `config.api.tmdb_key` |
| `search(query, limit)` | `GET /3/search/multi` — returns movies and TV shows matching query |
| `get_movie(id)` | Resolves any ID format: IMDb `tt*`, `movie:N`, `tv:N`, bare integer |
| `get_category(id, limit)` | Fetches a TMDB endpoint for the given category ID |
| `get_poster(url)` | Downloads raw image bytes from TMDB image CDN |

**ID resolution in `get_movie`:**
1. `tt*` → `GET /3/find/{id}?external_source=imdb_id` → TMDB ID → detail request
2. `movie:N` → `GET /3/movie/N?append_to_response=credits,external_ids,keywords`
3. `tv:N` → `GET /3/tv/N?append_to_response=credits,external_ids,keywords`
4. Bare integer → assumed movie

**Detail responses** include appended `credits` (cast + crew), `external_ids` (to build `imdb_url`), and `keywords`.

### `playimdb/mod.rs`

HTML scraper against [playimdb.com](https://playimdb.com). Requires no API key.

#### Data Models

| Type | Fields |
|------|--------|
| `StreamInfo` | `title`, `stream_url`, `direct_url`, `file_size`, `qualities`, `torrent_links` |
| `Quality` | `label` (e.g. "1080p"), `url`, `size_bytes`, `format` ("mp4"/"mkv") |
| `TorrentLink` | `label`, `magnet`, `torrent_url`, `size_bytes`, `seeders` |

#### `PlayImdbClient` Methods

| Method | Description |
|--------|-------------|
| `get_stream_info(imdb_id)` | Fetches `playimdb.com/title/{imdb_id}` and parses links |
| `search_by_title(title, year)` | Fallback: `playimdb.com/search?q={title+year}` |
| `open_in_browser(url)` | Opens a URL with `open::that()` (system default browser) |

**Parsing strategy for `parse_stream_page`:**
1. Find `<h1>` variants for the title.
2. Walk all `<a href>` elements; classify as "quality link" if the text contains resolution keywords or the href ends in `.mp4`/`.mkv`.
3. Separate pass for magnet (`magnet:`) and torrent (`.torrent`) links — seeders and size extracted via regex.
4. Look for `<iframe src>` to capture embedded player URLs.

**Fallback chain (both TUI and CLI):**
```
1. GET playimdb.com/title/{imdb_id}
   └── if no streams found:
2. GET TMDB for title + year → search playimdb.com by title
   └── if still no streams:
3. Open playimdb.com page in system browser
```

### `player/mod.rs`

Thin async wrapper around `tokio::process::Command`.

| Function | Description |
|----------|-------------|
| `play(url, config)` | Spawns the configured player with `extra_args` + URL; waits for exit |
| `detect_best_player()` | Probes `PATH` via `which::which` for: `mpv`, `vlc`, `mplayer`, `celluloid`, `totem`, `smplayer` |
| `build_command_string(config, url)` | Returns the full launch command as a human-readable string for display |

### `downloader/mod.rs`

Async streaming download with progress reporting.

| Method | Description |
|--------|-------------|
| `new()` | Constructs `reqwest::Client` with infinite timeout (downloads) |
| `download(url, dest, on_progress)` | Streams response body to file; calls `on_progress(downloaded, total)` per chunk |
| `probe_size(url)` | Issues a `HEAD` request to get `Content-Length` without downloading |
| `build_dest(config, filename)` | Joins `config.download_dir` with a sanitized filename |

Filenames are sanitized by replacing `/ \ : * ? " < > |` with `_`.

Speed is formatted by `format_speed(bytes_per_sec)` → `B/s`, `KB/s`, or `MB/s`.

### `kitty/mod.rs`

Implements the [Kitty Terminal Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) for inline image rendering.

| Function | Description |
|----------|-------------|
| `is_kitty()` | Returns `true` if `$TERM == "xterm-kitty"` OR `$KITTY_WINDOW_ID` is set |
| `display_image_bytes(data, cols, rows)` | Decodes raw image bytes (JPEG/PNG/WebP) and displays inline |
| `display_image(img, cols, rows)` | Encodes `DynamicImage` to PNG, base64-encodes, writes APC escape sequences |
| `clear_images()` | Sends `ESC_Ga=d ESC\` to delete all displayed images |
| `resize_for_terminal(img, max_cols, max_rows)` | Thumbnails the image to fit `max_cols*8 × max_rows*16` pixels |

The protocol uses chunked base64 encoding with 4096-byte chunks per APC frame:
```
ESC_G{params};{base64-chunk}ESC\
```

### `tui/mod.rs`

Sets up and tears down the raw terminal. Calls `crossterm` to enter alternate screen and enable mouse capture, constructs `ratatui::Terminal`, then delegates to `events::run_event_loop`. On return (quit or error), restores terminal state.

### `tui/app.rs`

Contains the `App` struct — the single source of truth for all TUI state.

#### Screens (state machine)

```
Home ──→ CategoryList ──→ MovieList ──→ MovieDetail ──→ StreamSelect
  ↑                                                         │
  └─────────────────────────────────────────────────────────┘
                                              ↓
                                       DownloadProgress
Search ──→ (results as MovieList)
Help   ──→ Home (on Esc)
Setup  ──→ Home (after key set)
```

#### Key `App` Fields

| Field | Type | Purpose |
|-------|------|---------|
| `screen` | `Screen` | Current active screen |
| `input_mode` | `InputMode` | `Normal` or `Searching` |
| `loading` | `LoadingState` | `Idle`, `Loading(msg)`, or `Error(msg)` |
| `selected_category` | `usize` | Cursor in category / home list |
| `selected_movie` | `usize` | Cursor in movie list |
| `scroll_offset` | `usize` | Scroll window start for movie list |
| `detail_scroll` | `usize` | Line scroll offset in detail view |
| `selected_quality` | `usize` | Cursor in stream select list |
| `movie_list` | `Vec<SearchResult>` | Current category or search page |
| `current_movie` | `Option<Movie>` | Fully loaded detail for selected title |
| `stream_info` | `Option<StreamInfo>` | Parsed playimdb data |
| `poster_cache` | `HashMap<String, Vec<u8>>` | Raw image bytes keyed by ID |
| `download_progress` | `Option<DownloadProgress>` | Live progress for download screen |
| `kitty_image_drawn` | `RefCell<Option<String>>` | Tracks drawn poster ID to prevent flicker |

#### Navigation

`move_up()` / `move_down()` dispatch on `self.screen` to update the appropriate selection index. `update_scroll_movie()` keeps the scroll window aligned. `go_back()` implements the back-navigation stack, clearing any Kitty images before transitioning.

### `tui/events.rs`

The main event loop (`run_event_loop`). On every 16 ms tick:
1. Render the current frame via `render::draw`.
2. Poll for a `crossterm::Event`.

Key mappings handled:

| Key | Screen | Action |
|-----|--------|--------|
| `q`, `Ctrl+C` | Any | Quit |
| `?`, `F1` | Any | `→ Screen::Help` |
| `Esc`, `b` | Any | `app.go_back()` |
| `j`/`↓`, `k`/`↑` | Any list | `move_down()` / `move_up()` |
| `PageDown/Up` | List | Scroll by `page_size` |
| `g`/`Home`, `G`/`End` | List | Jump to first/last |
| `Enter` | CategoryList | Load category, `→ Screen::MovieList` |
| `Enter` | MovieList | Load detail, `→ Screen::MovieDetail` |
| `Enter` | MovieDetail | Load streams, `→ Screen::StreamSelect` |
| `Enter` | StreamSelect | Play selected stream |
| `/`, `s` | Normal | `→ Screen::Search`, `InputMode::Searching` |
| `c` | Any | `→ Screen::CategoryList` |
| `r` | MovieList | Reload category |
| `p` | MovieList/Detail | Fetch streams + play first |
| `d` | MovieList/Detail | Fetch streams + download first |
| `w` | MovieDetail | Load stream sources (pre-fetch) |
| `i` | Any | Toggle Kitty image preview |
| `Char` | Searching mode | Append to `search_query`, trigger search |
| `Backspace` | Searching | Remove last char from query |
| `Enter` | Searching | Accept query, `→ Screen::Search` |

Async tasks (API calls, downloads) are dispatched via `tokio::spawn` and their results sent back via a `tokio::sync::mpsc` channel to avoid blocking the event loop.

### `tui/render.rs`

All ratatui rendering logic. `draw(f, app)` is the top-level function that:
1. Fills background with `C_BG = Rgb(10, 10, 18)`.
2. Splits the frame into 3 rows: header (3 lines), body (remaining), footer (3 lines).
3. Calls `draw_header`, `draw_body`, `draw_footer`.
4. Overlays `draw_loading_overlay` when `LoadingState::Loading`.

**Color palette:**

| Constant | RGB | Usage |
|----------|-----|-------|
| `C_BG` | `(10, 10, 18)` | Root background |
| `C_SURFACE` | `(20, 20, 32)` | Panel backgrounds |
| `C_SURFACE2` | `(28, 28, 42)` | Nested panels |
| `C_BORDER` | `(48, 48, 72)` | Default borders |
| `C_ACCENT` | `(255, 177, 0)` | Amber — selections, logo, active |
| `C_ACCENT2` | `(80, 200, 255)` | Sky blue — highlights |
| `C_TEXT` | `(220, 220, 230)` | Primary text |
| `C_MUTED` | `(120, 120, 150)` | Labels, separators |
| `C_GREEN` | `(100, 220, 100)` | Success indicators |
| `C_RED` | `(255, 90, 90)` | Error indicators |
| `C_PURPLE` | `(180, 120, 255)` | Special accents |

`draw_body` delegates to one of: `draw_home`, `draw_category_list`, `draw_movie_list`, `draw_movie_detail`, `draw_stream_select`, `draw_download_progress`, `draw_search`, `draw_help`, `draw_setup` depending on `app.screen`.

`draw_movie_detail` triggers Kitty poster rendering when `config.ui.kitty_images` is `true` and poster bytes are cached.

---

## Architectural Decision Records

### ADR-001: TMDB instead of IMDb scraping

**Status:** Accepted  
**Date:** 2025-06

**Context:** Direct scraping of imdb.com was blocked consistently by Cloudflare bot-detection, returning empty or error pages on every request regardless of user-agent spoofing.

**Decision:** Use the TMDB REST API v3 (free tier, 1M requests/month) as the sole data source for catalog browsing and movie metadata. IMDb IDs are still supported as input by using TMDB's `/find` endpoint with `external_source=imdb_id`.

**Consequences:**
- A free TMDB API key is now a hard requirement for any catalog operation.
- The first-run `Setup` screen guides users to register and enter the key.
- The module is named `imdb` internally for historical reasons; this is cosmetic only.

---

### ADR-002: playimdb.com as streaming backend

**Status:** Accepted  
**Date:** 2025-06

**Context:** playimdb.com provides a public HTML interface for finding streaming and torrent links for IMDb-identified titles. It requires no authentication.

**Decision:** Scrape playimdb.com using the `scraper` crate (CSS selector-based HTML parsing). Primary lookup is `playimdb.com/title/{imdb_id}`; fallback is title-search.

**Consequences:**
- The scraper is inherently fragile to site layout changes.
- IMDb IDs (not TMDB IDs) are required for the primary lookup, so the ID resolution pipeline must obtain the IMDb ID when starting from a TMDB ID.
- If no streams are found, the browser fallback ensures the user is never left with nothing.

---

### ADR-003: Async runtime — Tokio

**Status:** Accepted  
**Date:** 2025-06

**Context:** The application makes multiple HTTP requests (TMDB + playimdb), runs file downloads, and must keep the TUI responsive at all times.

**Decision:** Use `tokio` with the `full` feature set as the single async runtime. Blocking tasks are delegated to `tokio::task::spawn_blocking` where necessary.

**Consequences:**
- The TUI event loop (`run_event_loop`) runs on the main Tokio task, polling events at ~16 ms intervals.
- API calls and downloads are spawned as separate tasks and communicate results back via `mpsc` channels.
- `reqwest` is chosen as the HTTP client for native `tokio` integration.

---

### ADR-004: Kitty Graphics Protocol for poster images

**Status:** Accepted  
**Date:** 2025-06

**Context:** Text-only terminals cannot natively display images. The Kitty terminal emulator offers a rich, well-documented graphics protocol via APC escape sequences.

**Decision:** Implement the Kitty Graphics Protocol directly (no external library) using chunked base64-encoded PNG transmission. Detection is automatic via `$TERM`/`$KITTY_WINDOW_ID`. The feature is silently disabled in other terminals.

**Consequences:**
- Image display is not guaranteed to be perfectly aligned within ratatui's layout; a `kitty_image_drawn` guard prevents flicker from re-rendering.
- `clear_images()` is called on screen transitions to clean up.
- The `image` crate is used for JPEG/PNG/WebP decoding and resizing (1 cell ≈ 8×16 px).

---

### ADR-005: Single binary, dual mode

**Status:** Accepted  
**Date:** 2025-06

**Context:** Users need both interactive browsing (TUI) and scriptable one-shot commands (CLI) from a single tool.

**Decision:** The binary examines `Cli::command` (from `clap`). If `None`, launch TUI. If `Some(subcommand)`, run the corresponding CLI function and exit. Both modes share all service modules.

**Consequences:**
- No separate `watchie-tui` / `watchie-cli` binaries.
- Config is loaded once at startup and passed into both paths.
- Adding new subcommands requires changes only in `cli/mod.rs` (clap definition) and `cli/commands.rs` (implementation).
