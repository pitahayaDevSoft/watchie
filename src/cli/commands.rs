use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::cli::ConfigAction;
use crate::config::Config;
use crate::downloader::Downloader;
use crate::imdb::{ImdbClient, Movie, CATEGORIES};
use crate::playimdb::{format_size, PlayImdbClient};
use crate::player;

// ─── Search ───────────────────────────────────────────────────────────────────

pub async fn search(query: &str, limit: usize, _config: &Config) -> Result<()> {
    let client = ImdbClient::new()?;
    println!("🔍 Searching IMDB for \"{}\"…", query);
    let results = client.search(query, limit).await?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!(
        "\n{:<12} {:<40} {:<6} {:<10} {}",
        "IMDB ID", "Title", "Year", "Rating", "Type"
    );
    println!("{}", "─".repeat(80));

    for r in &results {
        let year = r.year.map(|y| y.to_string()).unwrap_or_else(|| "–".to_string());
        let rating = r.rating.map(|rt| format!("★ {:.1}", rt)).unwrap_or_else(|| "–".to_string());
        println!(
            "{:<12} {:<40} {:<6} {:<10} {}",
            r.id,
            truncate(&r.title, 39),
            year,
            rating,
            r.content_type
        );
    }

    println!("\n💡 Use `watchie info <IMDB_ID>` for full details");
    println!("💡 Use `watchie play <IMDB_ID>` to stream now");
    Ok(())
}

// ─── Info ─────────────────────────────────────────────────────────────────────

pub async fn info(id: &str, config: &Config) -> Result<()> {
    let id = resolve_id(id, config).await?;
    let client = ImdbClient::new()?;
    println!("📖 Fetching info for {}…", id);
    let movie = client.get_movie(&id).await?;
    print_movie_info(&movie);
    Ok(())
}

fn print_movie_info(m: &Movie) {
    let hr = "═".repeat(60);
    println!("\n{}", hr);
    println!("  🎬  {} ({})", m.title, m.year.map(|y| y.to_string()).unwrap_or_else(|| "–".into()));
    println!("{}", hr);

    if let Some(tl) = &m.tagline {
        println!("  \"{}\"", tl);
        println!();
    }

    let rating_str = m.rating.map(|r| format!("★ {:.1}/10", r)).unwrap_or_else(|| "–".into());
    let votes_str = m.votes.map(|v| format!("({} votes)", format_votes(v))).unwrap_or_default();
    println!("  Rating:    {} {}", rating_str, votes_str);

    if let Some(meta) = m.metascore {
        println!("  Metascore: {}/100", meta);
    }

    println!("  Type:      {}", m.content_type);

    if let Some(rt) = m.runtime {
        println!("  Runtime:   {} min", rt);
    }

    if !m.genres.is_empty() {
        println!("  Genres:    {}", m.genres.join(", "));
    }

    if !m.director.is_empty() {
        println!("  Director:  {}", m.director.join(", "));
    }

    if !m.cast.is_empty() {
        let cast_display: Vec<_> = m.cast.iter().take(5).collect();
        println!("  Cast:      {}", cast_display.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    if !m.language.is_empty() {
        println!("  Language:  {}", m.language.join(", "));
    }

    if !m.country.is_empty() {
        println!("  Country:   {}", m.country.join(", "));
    }

    if let Some(rd) = &m.release_date {
        println!("  Released:  {}", rd);
    }

    if let Some(plot) = &m.plot {
        println!();
        println!("  Plot:");
        for line in textwrap(plot, 56) {
            println!("    {}", line);
        }
    }

    if !m.keywords.is_empty() {
        println!();
        let kw: Vec<_> = m.keywords.iter().take(8).collect();
        println!("  Keywords:  {}", kw.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    println!();
    println!("  IMDB:  {}", m.imdb_url);
    println!("{}", hr);
}

// ─── Download / Play ──────────────────────────────────────────────────────────

pub async fn download_or_play(
    id: &str,
    output: Option<&str>,
    play_mode: bool,
    config: &Config,
) -> Result<()> {
    let id = resolve_id(id, config).await?;
    let _imdb_client = ImdbClient::new()?;
    let play_client = PlayImdbClient::new()?;

    println!("🔎 Looking up \"{}\" on playimdb.com…", id);
    let mut stream_info = play_client.get_stream_info(&id).await?;

    if stream_info.qualities.is_empty() && stream_info.torrent_links.is_empty() {
        if let Ok(movie) = _imdb_client.get_movie(&id).await {
            println!("   No direct stream for ID. Searching playimdb.com by title: \"{}\"…", movie.title);
            if let Ok(search_results) = play_client.search_by_title(&movie.title, movie.year).await {
                if let Some(first_match) = search_results.first() {
                    println!("   Found match via title search: {}", first_match.title);
                    if let Ok(detail) = play_client.get_stream_info(&first_match.stream_url).await {
                        stream_info = detail;
                    }
                }
            }
        }
    }

    if stream_info.qualities.is_empty() && stream_info.torrent_links.is_empty() {
        println!("⚠️  No streams found on playimdb.com for {}.", id);
        println!("   Opening browser on playimdb.com…");
        play_client.open_in_browser(&stream_info.stream_url).await?;
        return Ok(());
    }

    println!("\n📽️  Found: {}", stream_info.title);
    println!("   Stream page: {}", stream_info.stream_url);

    // Show quality options
    if !stream_info.qualities.is_empty() {
        println!("\n  Available qualities:");
        for (i, q) in stream_info.qualities.iter().enumerate() {
            let size_str = q
                .size_bytes
                .map(|s| format!(" ({})", format_size(s)))
                .unwrap_or_default();
            println!("    [{}] {} {}{}", i + 1, q.label, q.format, size_str);
        }
    }

    if !stream_info.torrent_links.is_empty() {
        println!("\n  Torrent/Magnet links:");
        for (i, t) in stream_info.torrent_links.iter().enumerate() {
            let size_str = t
                .size_bytes
                .map(|s| format!(" ({})", format_size(s)))
                .unwrap_or_default();
            let seeds = t
                .seeders
                .map(|s| format!(" 🌱{}", s))
                .unwrap_or_default();
            println!("    [M{}] {}{}{}", i + 1, t.label, size_str, seeds);
        }
    }

    // Pick first quality automatically (CLI mode — no interaction)
    if play_mode {
        if let Some(q) = stream_info.qualities.first() {
            let cmd_str = player::build_command_string(config, &q.url);
            println!("\n▶️  Opening with: {}…", cmd_str);
            player::play(&q.url, config).await?;
        } else if let Some(t) = stream_info.torrent_links.first() {
            if let Some(magnet) = &t.magnet {
                println!("\n🧲 Opening magnet link…");
                open::that(magnet)?;
            }
        } else {
            println!("\n🌐 Opening stream page in browser…");
            play_client.open_in_browser(&stream_info.stream_url).await?;
        }
    } else {
        // Download mode
        config.ensure_download_dir()?;
        if let Some(q) = stream_info.qualities.first() {
            let filename = format!(
                "{}.{}",
                stream_info.title.replace(' ', "."),
                q.format
            );
            let dest = if let Some(out) = output {
                PathBuf::from(out).join(&filename)
            } else {
                Downloader::build_dest(config, &filename)
            };

            // Probe size first
            let downloader = Downloader::new()?;
            if let Ok(Some(size)) = downloader.probe_size(&q.url).await {
                println!("\n📦 File size: {}", format_size(size));
            }

            println!("⬇️  Downloading to {}…", dest.display());

            use std::sync::{Arc, Mutex};
            use std::time::Instant;
            let start = Instant::now();
            let last_print = Arc::new(Mutex::new(Instant::now()));

            downloader
                .download(&q.url, &dest, move |downloaded, total| {
                    let mut lp = last_print.lock().unwrap();
                    if lp.elapsed().as_millis() > 500 {
                        *lp = Instant::now();
                        let elapsed = start.elapsed().as_secs_f64();
                        let speed = if elapsed > 0.0 {
                            downloaded as f64 / elapsed
                        } else {
                            0.0
                        };
                        if let Some(tot) = total {
                            let pct = downloaded * 100 / tot;
                            print!(
                                "\r  [{:>3}%] {} / {} — {}       ",
                                pct,
                                format_size(downloaded),
                                format_size(tot),
                                crate::downloader::format_speed(speed)
                            );
                        } else {
                            print!(
                                "\r  {} downloaded — {}       ",
                                format_size(downloaded),
                                crate::downloader::format_speed(speed)
                            );
                        }
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                })
                .await?;

            println!("\n✅ Download complete: {}", dest.display());
        } else {
            println!("No direct download links found. Opening browser…");
            play_client.open_in_browser(&stream_info.stream_url).await?;
        }
    }

    Ok(())
}

pub async fn play(id: &str, config: &Config) -> Result<()> {
    download_or_play(id, None, true, config).await
}

// ─── Top / Charts ─────────────────────────────────────────────────────────────

pub async fn top(category: Option<&str>, limit: usize, _config: &Config) -> Result<()> {
    let cat_id = category.unwrap_or("moviemeter");
    let cat_name = CATEGORIES
        .iter()
        .find(|c| c.id == cat_id || c.name.to_lowercase() == cat_id.to_lowercase())
        .map(|c| c.name)
        .unwrap_or(cat_id);

    let client = ImdbClient::new()?;
    println!("📊 Fetching {} (top {})…", cat_name, limit);
    let results = client.get_category(cat_id, limit).await?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!(
        "\n{:>4}  {:<12} {:<40} {:<6} {}",
        "#", "IMDB ID", "Title", "Year", "Rating"
    );
    println!("{}", "─".repeat(75));

    for (i, r) in results.iter().enumerate() {
        let year = r.year.map(|y| y.to_string()).unwrap_or_else(|| "–".to_string());
        let rating = r
            .rating
            .map(|rt| format!("★ {:.1}", rt))
            .unwrap_or_else(|| "–".to_string());
        println!(
            "{:>4}  {:<12} {:<40} {:<6} {}",
            i + 1,
            r.id,
            truncate(&r.title, 39),
            year,
            rating
        );
    }

    println!("\nCategories available:");
    for cat in CATEGORIES {
        println!("  {:20} ({})", cat.name, cat.id);
    }

    Ok(())
}

// ─── Config subcommand ────────────────────────────────────────────────────────

pub async fn config_cmd(action: ConfigAction, current: &Config) -> Result<()> {
    match action {
        ConfigAction::Show => {
            println!("{}", toml::to_string_pretty(current)?);
        }
        ConfigAction::SetDownloadDir { path } => {
            let mut cfg = current.clone();
            cfg.download_dir = PathBuf::from(&path);
            cfg.save()?;
            println!("✅ Download directory set to: {}", path);
        }
        ConfigAction::SetPlayer { command } => {
            let mut cfg = current.clone();
            cfg.player.command = command.clone();
            cfg.save()?;
            println!("✅ Player set to: {}", command);
        }
        ConfigAction::SetTmdbKey { key } => {
            let mut cfg = current.clone();
            cfg.api.tmdb_key = key.clone();
            cfg.save()?;
            println!("✅ TMDB API key set successfully.");
        }
        ConfigAction::Path => {
            println!("{}", Config::config_path()?.display());
        }
    }
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// If the string looks like an IMDB tt-ID, return it as-is.
/// Otherwise search IMDB and return the first match's ID.
async fn resolve_id(input: &str, _config: &Config) -> Result<String> {
    if input.starts_with("tt") && input.len() >= 7 {
        return Ok(input.to_string());
    }
    // Try search
    let client = ImdbClient::new()?;
    let results = client.search(input, 1).await?;
    if let Some(r) = results.into_iter().next() {
        println!("  → Resolved \"{}\" to: {} ({})", input, r.title, r.id);
        Ok(r.id)
    } else {
        bail!("Could not find an IMDB entry for \"{}\"", input)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn format_votes(v: u64) -> String {
    if v >= 1_000_000 {
        format!("{:.1}M", v as f64 / 1_000_000.0)
    } else if v >= 1_000 {
        format!("{:.0}K", v as f64 / 1_000.0)
    } else {
        v.to_string()
    }
}

fn textwrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut current = String::new();
    for word in words {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
