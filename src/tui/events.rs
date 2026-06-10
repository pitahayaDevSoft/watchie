use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;

use super::app::{App, InputMode, LoadingState, Screen, StatusStyle};
use super::render;
use crate::downloader::Downloader;
use crate::imdb::ImdbClient;
use crate::playimdb::PlayImdbClient;
use crate::player;

// ─── Async messages ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AppMsg {
    MovieListLoaded(Vec<crate::imdb::SearchResult>),
    MovieDetailLoaded(crate::imdb::Movie),
    StreamInfoLoaded(crate::playimdb::StreamInfo),
    PosterLoaded(String, Vec<u8>), // (imdb_id, bytes)
    SearchDone(Vec<crate::imdb::SearchResult>),
    DownloadProgress(u64, Option<u64>, f64),
    DownloadDone(String),
    Error(String),
    EpisodesLoaded(Vec<crate::imdb::Episode>),
    PlayImdbStatusUpdated(super::app::PlayImdbStatus),
}

pub async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMsg>();

    // Check if TMDB API Key is configured
    let api_key = std::env::var("TMDB_API_KEY")
        .unwrap_or_else(|_| app.config.api.tmdb_key.clone());

    if api_key.trim().is_empty() {
        app.screen = Screen::Setup;
        app.loading = LoadingState::Idle;
    } else {
        // Load initial category (Popular Movies)
        let tx2 = tx.clone();
        tokio::spawn(async move {
            match ImdbClient::new() {
                Ok(client) => match client.get_category("moviemeter", 50).await {
                    Ok(results) => {
                        let _ = tx2.send(AppMsg::MovieListLoaded(results));
                    }
                    Err(e) => {
                        let _ = tx2.send(AppMsg::Error(e.to_string()));
                    }
                },
                Err(e) => {
                    let _ = tx2.send(AppMsg::Error(e.to_string()));
                }
            }
        });
        app.loading = LoadingState::Loading("Loading Popular Movies…".to_string());
        app.list_title = "Popular Movies".to_string();
        app.screen = Screen::MovieList;
    }

    loop {
        // If we are no longer in MovieDetail, clear any drawn kitty images
        if app.screen != Screen::MovieDetail {
            let mut drawn = app.kitty_image_drawn.borrow_mut();
            if drawn.is_some() {
                let _ = crate::kitty::clear_images();
                *drawn = None;
            }
        }

        // Draw
        terminal.draw(|f| render::draw(f, app))?;

        // Handle async messages (non-blocking)
        while let Ok(msg) = rx.try_recv() {
            handle_msg(app, msg, tx.clone());
        }

        // Poll for keyboard events (16ms = ~60fps)
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key, tx.clone()).await? {
                    return Ok(()); // quit
                }
            }
        }
    }
}

fn handle_msg(app: &mut App, msg: AppMsg, tx: mpsc::UnboundedSender<AppMsg>) {
    match msg {
        AppMsg::MovieListLoaded(list) => {
            app.movie_list = list;
            app.selected_movie = 0;
            app.scroll_offset = 0;
            app.loading = LoadingState::Idle;
        }
        AppMsg::MovieDetailLoaded(movie) => {
            app.current_movie = Some(movie);
            app.detail_scroll = 0;
            app.loading = LoadingState::Idle;
            app.screen = Screen::MovieDetail;
            app.playimdb_status = super::app::PlayImdbStatus::Checking;

            // Spawn background task to check playimdb availability
            if let Some(ref m) = app.current_movie {
                let id = m.id.clone();
                let title = m.title.clone();
                let year = m.year;
                let tx2 = tx.clone();
                tokio::spawn(async move {
                    match PlayImdbClient::new() {
                        Ok(client) => {
                            let mut found = false;
                            if let Ok(info) = client.get_stream_info(&id).await {
                                if !info.qualities.is_empty() || !info.torrent_links.is_empty() || info.direct_url.is_some() {
                                    found = true;
                                }
                            }
                            if !found {
                                if let Ok(search_results) = client.search_by_title(&title, year).await {
                                    if let Some(first_match) = search_results.first() {
                                        if let Ok(info) = client.get_stream_info(&first_match.stream_url).await {
                                            if !info.qualities.is_empty() || !info.torrent_links.is_empty() || info.direct_url.is_some() {
                                                found = true;
                                            }
                                        }
                                    }
                                }
                            }
                            let status = if found {
                                super::app::PlayImdbStatus::Available
                            } else {
                                super::app::PlayImdbStatus::NotAvailable
                            };
                            let _ = tx2.send(AppMsg::PlayImdbStatusUpdated(status));
                        }
                        Err(e) => {
                            let _ = tx2.send(AppMsg::PlayImdbStatusUpdated(super::app::PlayImdbStatus::Error(e.to_string())));
                        }
                    }
                });
            }
        }
        AppMsg::EpisodesLoaded(episodes) => {
            app.episode_list = episodes;
            app.selected_episode = 0;
            app.loading = LoadingState::Idle;
            app.screen = Screen::EpisodeList;
        }
        AppMsg::StreamInfoLoaded(info) => {
            app.stream_info = Some(info);
            app.selected_quality = 0;
            app.loading = LoadingState::Idle;
            app.screen = Screen::StreamSelect;
        }
        AppMsg::PosterLoaded(id, bytes) => {
            app.poster_cache.insert(id, bytes);
        }
        AppMsg::PlayImdbStatusUpdated(status) => {
            app.playimdb_status = status;
        }
        AppMsg::SearchDone(results) => {
            app.search_results = results;
            app.selected_movie = 0;
            app.scroll_offset = 0;
            app.loading = LoadingState::Idle;
            app.screen = Screen::Search;
        }
        AppMsg::DownloadProgress(downloaded, total, speed) => {
            if let Some(ref mut dp) = app.download_progress {
                dp.downloaded = downloaded;
                dp.total = total;
                dp.speed = speed;
            }
        }
        AppMsg::DownloadDone(path) => {
            app.set_status(
                format!("✅ Downloaded: {}", path),
                StatusStyle::Success,
            );
            app.download_progress = None;
            app.screen = Screen::MovieDetail;
        }
        AppMsg::Error(e) => {
            app.loading = LoadingState::Error(e.clone());
            app.set_status(format!("❌ {}", e), StatusStyle::Error);
        }
    }
}

/// Returns true if the app should quit.
async fn handle_key(
    app: &mut App,
    key: KeyEvent,
    tx: mpsc::UnboundedSender<AppMsg>,
) -> Result<bool> {
    // Global quit
    if key.code == KeyCode::Char('q') && app.input_mode == InputMode::Normal {
        return Ok(true);
    }
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_key(app, key, tx).await?,
        InputMode::Searching => handle_search_key(app, key, tx).await?,
    }

    Ok(false)
}

async fn handle_normal_key(
    app: &mut App,
    key: KeyEvent,
    tx: mpsc::UnboundedSender<AppMsg>,
) -> Result<()> {
    if app.screen == Screen::Setup {
        return Ok(());
    }
    match key.code {
        // Movement
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::PageUp => {
            for _ in 0..app.config.ui.page_size {
                app.move_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..app.config.ui.page_size {
                app.move_down();
            }
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.selected_movie = 0;
            app.scroll_offset = 0;
            app.selected_category = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            let len = app.current_list().len();
            if len > 0 {
                app.selected_movie = len - 1;
                let page = app.config.ui.page_size;
                app.scroll_offset = if len > page { len - page } else { 0 };
            }
        }

        // Back / Escape
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('b') => {
            app.go_back();
        }

        // Enter — context action
        KeyCode::Enter => {
            on_enter(app, tx).await?;
        }

        // Search
        KeyCode::Char('/') | KeyCode::Char('s') => {
            app.input_mode = InputMode::Searching;
            app.search_query.clear();
            app.set_status("Type to search, Enter to confirm, Esc to cancel", StatusStyle::Info);
        }

        // Categories
        KeyCode::Char('c') => {
            app.screen = Screen::CategoryList;
        }

        // Refresh list
        KeyCode::Char('r') => {
            reload_current_category(app, tx).await;
        }

        // Play action from movie list (shortcut)
        KeyCode::Char('p') => {
            if app.screen == Screen::EpisodeList {
                load_episode_stream_info(app, tx).await;
            } else {
                play_selected(app, tx, true).await?;
            }
        }

        // Download action from movie list (shortcut)
        KeyCode::Char('d') => {
            if app.screen == Screen::EpisodeList {
                load_episode_stream_info(app, tx).await;
            } else {
                play_selected(app, tx, false).await?;
            }
        }

        // Open stream select from detail
        KeyCode::Char('w') => {
            if app.screen == Screen::MovieDetail {
                load_stream_info(app, tx).await;
            }
        }

        // Toggle Kitty images
        KeyCode::Char('i') => {
            app.config.ui.kitty_images = !app.config.ui.kitty_images;
            let state = if app.config.ui.kitty_images { "on" } else { "off" };
            app.set_status(format!("Kitty images: {}", state), StatusStyle::Info);
        }

        // Help
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.screen = Screen::Help;
        }

        // Open standalone stream in browser
        KeyCode::Char('o') => {
            if app.screen == Screen::MovieDetail || app.screen == Screen::StreamSelect || app.screen == Screen::EpisodeList {
                open_current_in_browser(app).await;
            }
        }

        _ => {}
    }
    Ok(())
}

async fn handle_search_key(
    app: &mut App,
    key: KeyEvent,
    tx: mpsc::UnboundedSender<AppMsg>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.clear_status();
        }
        KeyCode::Enter => {
            let query = app.search_query.clone();
            if !query.is_empty() {
                app.input_mode = InputMode::Normal;
                app.loading = LoadingState::Loading(format!("Searching \"{}\"…", query));
                let tx2 = tx.clone();
                tokio::spawn(async move {
                    match ImdbClient::new() {
                        Ok(client) => match client.search(&query, 50).await {
                            Ok(r) => { let _ = tx2.send(AppMsg::SearchDone(r)); }
                            Err(e) => { let _ = tx2.send(AppMsg::Error(e.to_string())); }
                        }
                        Err(e) => { let _ = tx2.send(AppMsg::Error(e.to_string())); }
                    }
                });
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
    Ok(())
}

// ─── Action helpers ───────────────────────────────────────────────────────────

async fn on_enter(app: &mut App, tx: mpsc::UnboundedSender<AppMsg>) -> Result<()> {
    match app.screen.clone() {
        Screen::Home | Screen::CategoryList => {
            let cat = &app.categories[app.selected_category];
            load_category(app, cat.id, cat.name, tx).await;
        }
        Screen::MovieList | Screen::Search => {
            if let Some(result) = app.selected_result().cloned() {
                load_movie_detail(app, result.id.clone(), tx.clone()).await;
                // Also prefetch poster
                if let Some(poster_url) = result.poster_url.clone() {
                    if !app.poster_cache.contains_key(&result.id) {
                        let id = result.id.clone();
                        let tx3 = tx.clone();
                        tokio::spawn(async move {
                            if let Ok(client) = ImdbClient::new() {
                                if let Ok(bytes) = client.download_poster(&poster_url).await {
                                    let _ = tx3.send(AppMsg::PosterLoaded(id, bytes));
                                }
                            }
                        });
                    }
                }
            }
        }
        Screen::MovieDetail => {
            if let Some(movie) = &app.current_movie {
                if movie.content_type == crate::imdb::ContentType::Series {
                    app.season_list = movie.season_list.clone();
                    app.selected_season = 0;
                    app.screen = Screen::SeasonList;
                } else {
                    load_stream_info(app, tx).await;
                }
            }
        }
        Screen::SeasonList => {
            if let Some(movie) = &app.current_movie {
                if let Some(season) = app.season_list.get(app.selected_season) {
                    let tv_id = movie.id.clone();
                    let season_num = season.season_number;
                    app.loading = LoadingState::Loading(format!("Loading Season {} episodes…", season_num));
                    let tx2 = tx.clone();
                    tokio::spawn(async move {
                        match ImdbClient::new() {
                            Ok(client) => match client.get_season(&tv_id, season_num).await {
                                Ok(episodes) => {
                                    let _ = tx2.send(AppMsg::EpisodesLoaded(episodes));
                                }
                                Err(e) => {
                                    let _ = tx2.send(AppMsg::Error(e.to_string()));
                                }
                            },
                            Err(e) => {
                                let _ = tx2.send(AppMsg::Error(e.to_string()));
                            }
                        }
                    });
                }
            }
        }
        Screen::EpisodeList => {
            load_episode_stream_info(app, tx).await;
        }
        Screen::StreamSelect => {
            // Enter on stream → play or download based on selected item
            execute_stream_action(app, tx, true).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn load_category(app: &mut App, id: &str, name: &str, tx: mpsc::UnboundedSender<AppMsg>) {
    app.loading = LoadingState::Loading(format!("Loading {}…", name));
    app.list_title = name.to_string();
    app.screen = Screen::MovieList;
    let id = id.to_string();
    tokio::spawn(async move {
        match ImdbClient::new() {
            Ok(client) => match client.get_category(&id, 100).await {
                Ok(r) => { let _ = tx.send(AppMsg::MovieListLoaded(r)); }
                Err(e) => { let _ = tx.send(AppMsg::Error(e.to_string())); }
            }
            Err(e) => { let _ = tx.send(AppMsg::Error(e.to_string())); }
        }
    });
}

async fn load_movie_detail(app: &mut App, id: String, tx: mpsc::UnboundedSender<AppMsg>) {
    app.loading = LoadingState::Loading("Loading movie details…".to_string());
    tokio::spawn(async move {
        match ImdbClient::new() {
            Ok(client) => match client.get_movie(&id).await {
                Ok(m) => { let _ = tx.send(AppMsg::MovieDetailLoaded(m)); }
                Err(e) => { let _ = tx.send(AppMsg::Error(e.to_string())); }
            }
            Err(e) => { let _ = tx.send(AppMsg::Error(e.to_string())); }
        }
    });
}

async fn load_stream_info(app: &mut App, tx: mpsc::UnboundedSender<AppMsg>) {
    if let Some(movie) = &app.current_movie {
        let id = movie.id.clone();
        let title = movie.title.clone();
        let year = movie.year;
        app.loading = LoadingState::Loading("Fetching stream sources…".to_string());
        tokio::spawn(async move {
            match PlayImdbClient::new() {
                Ok(client) => {
                    let mut res = client.get_stream_info(&id).await;
                    if let Ok(ref info) = res {
                        if info.qualities.is_empty() && info.torrent_links.is_empty() {
                            // Try search by title as fallback
                            if let Ok(search_results) = client.search_by_title(&title, year).await {
                                if let Some(first_match) = search_results.first() {
                                    if let Ok(detail) = client.get_stream_info(&first_match.stream_url).await {
                                        res = Ok(detail);
                                    }
                                }
                            }
                        }
                    }
                    match res {
                        Ok(info) => { let _ = tx.send(AppMsg::StreamInfoLoaded(info)); }
                        Err(e) => { let _ = tx.send(AppMsg::Error(e.to_string())); }
                    }
                }
                Err(e) => { let _ = tx.send(AppMsg::Error(e.to_string())); }
            }
        });
    }
}

async fn load_episode_stream_info(app: &mut App, tx: mpsc::UnboundedSender<AppMsg>) {
    if let Some(movie) = &app.current_movie {
        if let Some(episode) = app.episode_list.get(app.selected_episode) {
            let tv_id = movie.id.clone();
            let series_title = movie.title.clone();
            let season = episode.season_number;
            let ep_num = episode.episode_number;
            let ep_name = episode.name.clone();
            let year = movie.year;

            app.loading = LoadingState::Loading(format!("Fetching streams for S{:02}E{:02}…", season, ep_num));
            tokio::spawn(async move {
                match ImdbClient::new() {
                    Ok(imdb_client) => {
                        let mut ep_imdb_id = None;
                        if let Ok(id) = imdb_client.get_episode_imdb_id(&tv_id, season, ep_num).await {
                            ep_imdb_id = Some(id);
                        }

                        match PlayImdbClient::new() {
                            Ok(play_client) => {
                                let mut res = Err(anyhow::anyhow!("No stream found"));
                                
                                if let Some(ref id) = ep_imdb_id {
                                    res = play_client.get_stream_info(id).await;
                                }
                                
                                if res.is_err() || res.as_ref().map(|info| info.qualities.is_empty() && info.torrent_links.is_empty()).unwrap_or(true) {
                                    let query = format!("{} S{:02}E{:02}", series_title, season, ep_num);
                                    if let Ok(search_results) = play_client.search_by_title(&query, year).await {
                                        if let Some(first_match) = search_results.first() {
                                            if let Ok(detail) = play_client.get_stream_info(&first_match.stream_url).await {
                                                res = Ok(detail);
                                            }
                                        }
                                    }
                                }
                                
                                if res.is_err() || res.as_ref().map(|info| info.qualities.is_empty() && info.torrent_links.is_empty()).unwrap_or(true) {
                                    if let Ok(search_results) = play_client.search_by_title(&series_title, year).await {
                                        if let Some(first_match) = search_results.first() {
                                            if let Ok(detail) = play_client.get_stream_info(&first_match.stream_url).await {
                                                res = Ok(detail);
                                            }
                                        }
                                    }
                                }

                                match res {
                                    Ok(mut info) => {
                                        info.title = format!("{} - S{:02}E{:02} - {}", series_title, season, ep_num, ep_name);
                                        let _ = tx.send(AppMsg::StreamInfoLoaded(info));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(AppMsg::Error(e.to_string()));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(AppMsg::Error(e.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AppMsg::Error(e.to_string()));
                    }
                }
            });
        }
    }
}

async fn reload_current_category(app: &mut App, tx: mpsc::UnboundedSender<AppMsg>) {
    let cat = &app.categories[app.selected_category];
    load_category(app, cat.id, cat.name, tx).await;
}

async fn play_selected(
    app: &mut App,
    tx: mpsc::UnboundedSender<AppMsg>,
    _play_mode: bool,
) -> Result<()> {
    if let Some(result) = app.selected_result().cloned() {
        load_movie_detail(app, result.id.clone(), tx.clone()).await;
        // After detail loads, stream selection will appear via enter
    }
    Ok(())
}

async fn execute_stream_action(
    app: &mut App,
    tx: mpsc::UnboundedSender<AppMsg>,
    play_mode: bool,
) -> Result<()> {
    if let Some(info) = &app.stream_info {
        let qi = app.selected_quality;
        let num_qualities = info.qualities.len();
        let referer = crate::playimdb::extract_origin(&info.stream_url);

        if qi < num_qualities {
            let url = info.qualities[qi].url.clone();
            let config = app.config.clone();
            let referer_clone = referer.clone();
            if play_mode {
                let cmd_str = player::build_command_string(&config, &url, referer.as_deref());
                app.set_status(
                    format!("▶️  Launching: {}…", cmd_str),
                    StatusStyle::Info,
                );
                tokio::spawn(async move {
                    let _ = player::play(&url, referer_clone.as_deref(), &config).await;
                });
            } else {
                // Download
                let filename = format!(
                    "{}.{}",
                    info.title.replace(' ', "."),
                    info.qualities[qi].format
                );
                let dest = Downloader::build_dest(&config, &filename);
                app.download_progress = Some(super::app::DownloadProgress {
                    filename: dest.display().to_string(),
                    downloaded: 0,
                    total: None,
                    speed: 0.0,
                });
                app.screen = Screen::DownloadProgress;

                let tx2 = tx.clone();
                let dest_str = dest.display().to_string();
                let referer_dl = referer_clone.clone();
                tokio::spawn(async move {
                    config.ensure_download_dir().ok();
                    match Downloader::new(&config) {
                        Ok(dl) => {
                            use std::sync::{Arc, Mutex};
                            use std::time::Instant;
                            let start = Arc::new(Mutex::new(Instant::now()));
                            let tx3 = tx2.clone();
                            let _ = dl
                                .download(&url, referer_dl.as_deref(), &dest, move |dl, tot| {
                                    let elapsed = start.lock().unwrap().elapsed().as_secs_f64();
                                    let speed = if elapsed > 0.0 { dl as f64 / elapsed } else { 0.0 };
                                    let _ = tx3.send(AppMsg::DownloadProgress(dl, tot, speed));
                                })
                                .await;
                            let _ = tx2.send(AppMsg::DownloadDone(dest_str));
                        }
                        Err(e) => {
                            let _ = tx2.send(AppMsg::Error(e.to_string()));
                        }
                    }
                });
            }
        } else {
            // Torrent/magnet link
            let ti = qi - num_qualities;
            if let Some(link) = info.torrent_links.get(ti) {
                if let Some(magnet) = &link.magnet {
                    let magnet = magnet.clone();
                    tokio::spawn(async move {
                        let _ = open::that(&magnet);
                    });
                    app.set_status("🧲 Opening magnet link…", StatusStyle::Info);
                } else if let Some(turl) = &link.torrent_url {
                    let turl = turl.clone();
                    tokio::spawn(async move {
                        let _ = open::that(&turl);
                    });
                    app.set_status("🧲 Opening torrent…", StatusStyle::Info);
                }
            }
        }
    }
    Ok(())
}

async fn open_current_in_browser(app: &mut App) {
    let mut target_url = None;
    let base_url = if let Some(ref url) = app.config.api.playimdb_url {
        url.trim_end_matches('/').to_string()
    } else {
        "https://playimdb.com".to_string()
    };

    if app.screen == Screen::StreamSelect {
        if let Some(ref info) = app.stream_info {
            target_url = Some(info.stream_url.clone());
        }
    } else if app.screen == Screen::EpisodeList {
        if let (Some(movie), Some(episode)) = (&app.current_movie, app.episode_list.get(app.selected_episode)) {
            target_url = Some(format!("{}/title/{}/{}/{}", base_url, movie.id, episode.season_number, episode.episode_number));
        }
    } else if let Some(ref movie) = app.current_movie {
        target_url = Some(format!("{}/title/{}", base_url, movie.id));
    }

    if let Some(url) = target_url {
        app.set_status(format!("🌐 Opening browser: {}…", url), StatusStyle::Info);
        tokio::spawn(async move {
            let _ = open::that(&url);
        });
    }
}
