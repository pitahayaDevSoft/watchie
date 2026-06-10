use anyhow::Result;
use std::collections::HashMap;

use crate::config::Config;
use crate::imdb::{Movie, SearchResult, CATEGORIES};
use crate::playimdb::StreamInfo;

// ─── Application state ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    CategoryList,
    MovieList,
    MovieDetail,
    SeasonList,
    EpisodeList,
    StreamSelect,
    DownloadProgress,
    Search,
    Help,
    Setup,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Searching,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    Idle,
    Loading(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayImdbStatus {
    Unknown,
    Checking,
    Available,
    NotAvailable,
    Error(String),
}

pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub input_mode: InputMode,
    pub loading: LoadingState,
    pub playimdb_status: PlayImdbStatus,

    // Navigation
    pub selected_category: usize,
    pub selected_movie: usize,
    pub scroll_offset: usize,
    pub detail_scroll: usize,
    pub selected_quality: usize,
    pub selected_season: usize,
    pub selected_episode: usize,

    // Data
    pub categories: &'static [crate::imdb::Category],
    pub movie_list: Vec<SearchResult>,
    pub current_movie: Option<Movie>,
    pub stream_info: Option<StreamInfo>,
    pub season_list: Vec<crate::imdb::Season>,
    pub episode_list: Vec<crate::imdb::Episode>,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,

    // Poster cache (imdb_id → raw image bytes)
    pub poster_cache: HashMap<String, Vec<u8>>,

    // Status bar message
    pub status_msg: Option<String>,
    pub status_style: StatusStyle,

    // Download progress
    pub download_progress: Option<DownloadProgress>,

    // Page title for current movie list
    pub list_title: String,

    // Track if kitty image has been drawn to prevent redrawing/flickering
    pub kitty_image_drawn: std::cell::RefCell<Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusStyle {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: f64,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            screen: Screen::Home,
            input_mode: InputMode::Normal,
            loading: LoadingState::Idle,
            playimdb_status: PlayImdbStatus::Unknown,
            selected_category: 0,
            selected_movie: 0,
            scroll_offset: 0,
            detail_scroll: 0,
            selected_quality: 0,
            selected_season: 0,
            selected_episode: 0,
            categories: CATEGORIES,
            movie_list: Vec::new(),
            current_movie: None,
            stream_info: None,
            season_list: Vec::new(),
            episode_list: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            poster_cache: HashMap::new(),
            status_msg: Some("Welcome to Watchie! Press ? for help".to_string()),
            status_style: StatusStyle::Info,
            download_progress: None,
            list_title: String::new(),
            kitty_image_drawn: std::cell::RefCell::new(None),
        })
    }

    // ─── Navigation helpers ───────────────────────────────────────────────────

    pub fn move_up(&mut self) {
        match self.screen {
            Screen::Home | Screen::CategoryList => {
                if self.selected_category > 0 {
                    self.selected_category -= 1;
                }
            }
            Screen::MovieList | Screen::Search => {
                let list = if self.screen == Screen::Search {
                    &self.search_results
                } else {
                    &self.movie_list
                };
                if self.selected_movie > 0 {
                    self.selected_movie -= 1;
                    self.update_scroll_movie(list.len());
                }
            }
            Screen::StreamSelect => {
                if self.selected_quality > 0 {
                    self.selected_quality -= 1;
                }
            }
            Screen::MovieDetail => {
                if self.detail_scroll > 0 {
                    self.detail_scroll -= 1;
                }
            }
            Screen::SeasonList => {
                if self.selected_season > 0 {
                    self.selected_season -= 1;
                }
            }
            Screen::EpisodeList if self.selected_episode > 0 => {
                self.selected_episode -= 1;
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.screen {
            Screen::Home | Screen::CategoryList => {
                if self.selected_category + 1 < self.categories.len() {
                    self.selected_category += 1;
                }
            }
            Screen::MovieList | Screen::Search => {
                let len = if self.screen == Screen::Search {
                    self.search_results.len()
                } else {
                    self.movie_list.len()
                };
                if self.selected_movie + 1 < len {
                    self.selected_movie += 1;
                    self.update_scroll_movie(len);
                }
            }
            Screen::StreamSelect => {
                let count = self
                    .stream_info
                    .as_ref()
                    .map(|s| s.qualities.len() + s.torrent_links.len())
                    .unwrap_or(0);
                if self.selected_quality + 1 < count {
                    self.selected_quality += 1;
                }
            }
            Screen::MovieDetail => {
                self.detail_scroll += 1;
            }
            Screen::SeasonList => {
                if self.selected_season + 1 < self.season_list.len() {
                    self.selected_season += 1;
                }
            }
            Screen::EpisodeList if self.selected_episode + 1 < self.episode_list.len() => {
                self.selected_episode += 1;
            }
            _ => {}
        }
    }

    fn update_scroll_movie(&mut self, _total: usize) {
        let visible = self.config.ui.page_size;
        if self.selected_movie < self.scroll_offset {
            self.scroll_offset = self.selected_movie;
        } else if self.selected_movie >= self.scroll_offset + visible {
            self.scroll_offset = self.selected_movie + 1 - visible;
        }
    }

    pub fn go_back(&mut self) {
        if self.kitty_image_drawn.borrow().is_some() {
            let _ = crate::kitty::clear_images();
            *self.kitty_image_drawn.borrow_mut() = None;
        }
        self.screen = match self.screen {
            Screen::MovieList => Screen::CategoryList,
            Screen::MovieDetail => Screen::MovieList,
            Screen::SeasonList => Screen::MovieDetail,
            Screen::EpisodeList => Screen::SeasonList,
            Screen::StreamSelect => {
                if !self.episode_list.is_empty() {
                    Screen::EpisodeList
                } else {
                    Screen::MovieDetail
                }
            }
            Screen::Search => Screen::Home,
            Screen::Help => Screen::Home,
            _ => Screen::Home,
        };
        self.status_msg = None;
    }

    // ─── Status helpers ───────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: impl Into<String>, style: StatusStyle) {
        self.status_msg = Some(msg.into());
        self.status_style = style;
    }

    pub fn clear_status(&mut self) {
        self.status_msg = None;
    }

    // ─── Current item getters ─────────────────────────────────────────────────

    pub fn current_list(&self) -> &[SearchResult] {
        if self.screen == Screen::Search {
            &self.search_results
        } else {
            &self.movie_list
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.current_list().get(self.selected_movie)
    }
}
