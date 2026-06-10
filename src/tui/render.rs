use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState,
        Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use super::app::{App, InputMode, LoadingState, Screen, StatusStyle};
use crate::playimdb::format_size;

// ─── Color palette ────────────────────────────────────────────────────────────

const C_BG: Color = Color::Rgb(10, 10, 18);
const C_SURFACE: Color = Color::Rgb(20, 20, 32);
const C_SURFACE2: Color = Color::Rgb(28, 28, 42);
const C_BORDER: Color = Color::Rgb(48, 48, 72);
const C_ACCENT: Color = Color::Rgb(255, 177, 0); // amber
const C_ACCENT2: Color = Color::Rgb(80, 200, 255); // sky blue
const C_TEXT: Color = Color::Rgb(220, 220, 230);
const C_MUTED: Color = Color::Rgb(120, 120, 150);
const C_GREEN: Color = Color::Rgb(100, 220, 100);
const C_RED: Color = Color::Rgb(255, 90, 90);
const C_PURPLE: Color = Color::Rgb(180, 120, 255);

// ─── Main draw ────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(C_BG)),
        area,
    );

    // Main layout: header + body + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // body
            Constraint::Length(3),  // footer
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    // Loading overlay
    if let LoadingState::Loading(msg) = &app.loading {
        draw_loading_overlay(f, area, msg);
    }
}

// ─── Header ───────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(30),
        ])
        .split(inner);

    // Logo + breadcrumb
    let breadcrumb = match app.screen {
        Screen::Home => " Home".to_string(),
        Screen::CategoryList => " Categories".to_string(),
        Screen::MovieList => format!(" {}", app.list_title),
        Screen::MovieDetail => {
            let title = app.current_movie.as_ref().map(|m| m.title.as_str()).unwrap_or("Detail");
            format!(" Movies › {}", title)
        }
        Screen::SeasonList => {
            let title = app.current_movie.as_ref().map(|m| m.title.as_str()).unwrap_or("Detail");
            format!(" TV Series › {} › Seasons", title)
        }
        Screen::EpisodeList => {
            let title = app.current_movie.as_ref().map(|m| m.title.as_str()).unwrap_or("Detail");
            let season_num = app.season_list.get(app.selected_season).map(|s| s.season_number).unwrap_or(1);
            format!(" TV Series › {} › Season {}", title, season_num)
        }
        Screen::StreamSelect => " Stream Select".to_string(),
        Screen::DownloadProgress => " Downloading…".to_string(),
        Screen::Search => format!(" Search: {}", app.search_query),
        Screen::Help => " Help".to_string(),
        Screen::Setup => " Setup Required".to_string(),
    };

    let logo = Span::styled(
        "🎬 WATCHIE",
        Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
    );
    let sep = Span::styled("  ›  ", Style::default().fg(C_MUTED));
    let breadcrumb_span = Span::styled(breadcrumb, Style::default().fg(C_TEXT));

    f.render_widget(
        Paragraph::new(Line::from(vec![logo, sep, breadcrumb_span]))
            .alignment(Alignment::Left),
        chunks[0],
    );

    // Search input or mode hint
    let hint = if app.input_mode == InputMode::Searching {
        Span::styled(
            format!(" / {}█", app.search_query),
            Style::default().fg(C_ACCENT2).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" / to search  ? help", Style::default().fg(C_MUTED))
    };
    f.render_widget(
        Paragraph::new(Line::from(hint)).alignment(Alignment::Right),
        chunks[1],
    );
}

// ─── Body ─────────────────────────────────────────────────────────────────────

fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    match app.screen {
        Screen::Home => draw_home(f, app, area),
        Screen::CategoryList => draw_category_list(f, app, area),
        Screen::MovieList | Screen::Search => draw_movie_list(f, app, area),
        Screen::MovieDetail => draw_movie_detail(f, app, area),
        Screen::SeasonList => draw_season_list(f, app, area),
        Screen::EpisodeList => draw_episode_list(f, app, area),
        Screen::StreamSelect => draw_stream_select(f, app, area),
        Screen::DownloadProgress => draw_download_progress(f, app, area),
        Screen::Help => draw_help(f, app, area),
        Screen::Setup => draw_setup(f, app, area),
    }
}

// ─── Home screen ──────────────────────────────────────────────────────────────

fn draw_home(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Category list on left
    draw_category_panel(f, app, chunks[0]);

    // Quick stats / art on right
    draw_home_art(f, app, chunks[1]);
}

fn draw_category_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" 📂 Categories ", Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_selected = i == app.selected_category;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD).bg(C_SURFACE2)
            } else {
                Style::default().fg(C_TEXT)
            };
            ListItem::new(format!("{}{}", prefix, cat.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_category));

    f.render_stateful_widget(List::new(items).block(block), area, &mut state);
}

fn draw_home_art(f: &mut Frame, _app: &App, area: Rect) {
    let art = vec![
        "",
        "  ╔══════════════════════════════╗",
        "  ║                              ║",
        "  ║   🎬  Browse IMDB Catalog    ║",
        "  ║   🌐  Stream via playimdb    ║",
        "  ║   ⬇️   Download in any dir   ║",
        "  ║   🖼️   Kitty image previews  ║",
        "  ║                              ║",
        "  ╚══════════════════════════════╝",
        "",
        "  Quick Keys:",
        "  Enter  → Open / Select",
        "  /      → Search IMDB",
        "  c      → Browse categories",
        "  p      → Play selected",
        "  d      → Download selected",
        "  ?      → Help",
        "  q      → Quit",
    ];

    let lines: Vec<Line> = art
        .iter()
        .map(|l| {
            if l.contains("🎬") || l.contains("🌐") || l.contains("⬇️") || l.contains("🖼️") {
                Line::from(Span::styled(*l, Style::default().fg(C_ACCENT2)))
            } else if l.starts_with("  Quick") {
                Line::from(Span::styled(*l, Style::default().fg(C_ACCENT).bold()))
            } else if l.contains("→") {
                let parts: Vec<&str> = l.splitn(2, "→").collect();
                Line::from(vec![
                    Span::styled(parts[0], Style::default().fg(C_ACCENT).bold()),
                    Span::styled("→", Style::default().fg(C_MUTED)),
                    Span::styled(parts.get(1).copied().unwrap_or(""), Style::default().fg(C_TEXT)),
                ])
            } else {
                Line::from(Span::styled(*l, Style::default().fg(C_MUTED)))
            }
        })
        .collect();

    let block = Block::default()
        .title(Span::styled(" 🎥 Watchie ", Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

// ─── Category list ────────────────────────────────────────────────────────────

fn draw_category_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" 📂 Browse Categories ", Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_selected = i == app.selected_category;
            let icon = category_icon(cat.id);
            let style = if is_selected {
                Style::default().fg(C_BG).bg(C_ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_TEXT)
            };
            let line = format!(" {} {} ", icon, cat.name);
            ListItem::new(line).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_category));
    f.render_stateful_widget(List::new(items).block(block), area, &mut state);
}

fn category_icon(id: &str) -> &'static str {
    match id {
        "top" | "toptv" => "⭐",
        "moviemeter" => "🔥",
        "boxoffice" => "💰",
        "comingsoon" => "📅",
        "oscar-winners" => "🏆",
        id if id.contains("action") => "💥",
        id if id.contains("adventure") => "🗺️",
        id if id.contains("animation") => "🎨",
        id if id.contains("comedy") => "😂",
        id if id.contains("crime") => "🔫",
        id if id.contains("documentary") => "📹",
        id if id.contains("drama") => "🎭",
        id if id.contains("fantasy") => "🧙",
        id if id.contains("horror") => "👻",
        id if id.contains("mystery") => "🔍",
        id if id.contains("romance") => "❤️",
        id if id.contains("sci-fi") => "🚀",
        id if id.contains("thriller") => "😰",
        id if id.contains("western") => "🤠",
        _ => "🎬",
    }
}

// ─── Movie list ───────────────────────────────────────────────────────────────

fn draw_movie_list(f: &mut Frame, app: &App, area: Rect) {
    let list = app.current_list();
    let title_text = if app.screen == Screen::Search {
        format!(" 🔍 Results for \"{}\" ({}) ", app.search_query, list.len())
    } else {
        format!(" 🎬 {} ({}) ", app.list_title, list.len())
    };

    let block = Block::default()
        .title(Span::styled(title_text, Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if list.is_empty() {
        f.render_widget(
            Paragraph::new("\n  No results yet. Press Enter on a category or use / to search.")
                .style(Style::default().fg(C_MUTED))
                .alignment(Alignment::Left),
            inner,
        );
        return;
    }

    // Column header
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{:<4} ", "#"), Style::default().fg(C_MUTED)),
            Span::styled(format!("{:<45}", "Title"), Style::default().fg(C_MUTED)),
            Span::styled(format!("{:<6}", "Year"), Style::default().fg(C_MUTED)),
            Span::styled(format!("{:<8}", "Rating"), Style::default().fg(C_MUTED)),
            Span::styled("Type", Style::default().fg(C_MUTED)),
        ])),
        header_chunks[0],
    );

    let visible = header_chunks[1].height as usize;
    let start = app.scroll_offset;
    let end = (start + visible).min(list.len());
    let slice = &list[start..end];

    let items: Vec<ListItem> = slice
        .iter()
        .enumerate()
        .map(|(rel_i, r)| {
            let abs_i = start + rel_i;
            let is_selected = abs_i == app.selected_movie;

            let year = r.year.map(|y| y.to_string()).unwrap_or_else(|| "─".into());
            let rating = r
                .rating
                .map(|rt| format!("★{:.1}", rt))
                .unwrap_or_else(|| "─".into());
            let title = truncate(&r.title, 44);
            let ctype = r.content_type.to_string();
            let ctype_short = if ctype.len() > 8 { &ctype[..8] } else { &ctype };

            let num_str = format!("{:>3}. ", abs_i + 1);

            if is_selected {
                ListItem::new(Line::from(vec![
                    Span::styled(num_str, Style::default().fg(C_ACCENT).bold()),
                    Span::styled(
                        format!("{:<45}", title),
                        Style::default().fg(C_BG).bg(C_ACCENT).bold(),
                    ),
                    Span::styled(
                        format!("{:<6}", year),
                        Style::default().fg(C_BG).bg(C_ACCENT),
                    ),
                    Span::styled(
                        format!("{:<8}", rating),
                        Style::default().fg(C_BG).bg(C_ACCENT),
                    ),
                    Span::styled(
                        ctype_short.to_owned(),
                        Style::default().fg(C_BG).bg(C_ACCENT),
                    ),
                ]))
            } else {
                let title_color = if abs_i % 2 == 0 { C_TEXT } else { Color::Rgb(200, 200, 215) };
                ListItem::new(Line::from(vec![
                    Span::styled(num_str, Style::default().fg(C_MUTED)),
                    Span::styled(format!("{:<45}", title), Style::default().fg(title_color)),
                    Span::styled(format!("{:<6}", year), Style::default().fg(C_MUTED)),
                    Span::styled(
                        format!("{:<8}", rating),
                        Style::default().fg(C_ACCENT),
                    ),
                    Span::styled(ctype_short.to_string(), Style::default().fg(C_PURPLE)),
                ]))
            }
        })
        .collect();

    let mut state = ListState::default();
    let selected_in_view = if app.selected_movie >= start {
        Some(app.selected_movie - start)
    } else {
        None
    };
    state.select(selected_in_view);

    f.render_stateful_widget(List::new(items), header_chunks[1], &mut state);

    // Scrollbar
    let mut scroll_state = ScrollbarState::new(list.len()).position(app.scroll_offset);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(C_BORDER)),
        area.inner(Margin { horizontal: 0, vertical: 1 }),
        &mut scroll_state,
    );
}

// ─── Movie detail ─────────────────────────────────────────────────────────────

fn draw_movie_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(movie) = &app.current_movie else {
        f.render_widget(
            Paragraph::new("Loading…").style(Style::default().fg(C_MUTED)),
            area,
        );
        return;
    };

    // Split: left = info, right = poster (if kitty)
    let (info_area, poster_area_opt) = if app.config.ui.kitty_images
        && app.poster_cache.contains_key(&movie.id)
        && area.width > 80
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(24)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Info panel
    let block = Block::default()
        .title(Span::styled(
            format!(" 🎬 {} ", movie.title),
            Style::default().fg(C_ACCENT).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(info_area);
    f.render_widget(block, info_area);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // Title + year
    lines.push(Line::from(vec![
        Span::styled(&movie.title, Style::default().fg(C_ACCENT).bold().add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled(
            movie.year.map(|y| format!("({})", y)).unwrap_or_default(),
            Style::default().fg(C_MUTED),
        ),
    ]));

    if let Some(tl) = &movie.tagline {
        lines.push(Line::from(Span::styled(
            format!("\"{}\"", tl),
            Style::default().fg(C_ACCENT2).add_modifier(Modifier::ITALIC),
        )));
    }

    lines.push(Line::from(""));

    // Rating row
    let rating_str = movie
        .rating
        .map(|r| format!("★ {:.1}/10", r))
        .unwrap_or_else(|| "No rating".into());
    let votes_str = movie
        .votes
        .map(|v| format!(" ({} votes)", fmt_votes(v)))
        .unwrap_or_default();
    
    let mut rating_line = vec![
        Span::styled("  Rating:  ", Style::default().fg(C_MUTED)),
        Span::styled(rating_str, Style::default().fg(C_ACCENT).bold()),
        Span::styled(votes_str, Style::default().fg(C_MUTED)),
    ];

    rating_line.push(Span::styled("    PlayIMDb: ", Style::default().fg(C_MUTED)));
    match &app.playimdb_status {
        super::app::PlayImdbStatus::Unknown => {
            rating_line.push(Span::styled("❓ Unknown", Style::default().fg(C_MUTED)));
        }
        super::app::PlayImdbStatus::Checking => {
            rating_line.push(Span::styled("⏳ Checking...", Style::default().fg(C_ACCENT2).bold()));
        }
        super::app::PlayImdbStatus::Available => {
            rating_line.push(Span::styled("🟢 Available", Style::default().fg(C_GREEN).bold()));
        }
        super::app::PlayImdbStatus::NotAvailable => {
            rating_line.push(Span::styled("🔴 Not Found", Style::default().fg(C_RED).bold()));
        }
        super::app::PlayImdbStatus::Error(e) => {
            rating_line.push(Span::styled(format!("⚠️ Error ({})", e), Style::default().fg(C_RED)));
        }
    }
    
    lines.push(Line::from(rating_line));

    // Type + runtime
    let runtime_str = movie
        .runtime
        .map(|r| format!("{} min", r))
        .unwrap_or_else(|| "–".into());
    lines.push(Line::from(vec![
        Span::styled("  Type:    ", Style::default().fg(C_MUTED)),
        Span::styled(movie.content_type.to_string(), Style::default().fg(C_PURPLE)),
        Span::styled("   Runtime: ", Style::default().fg(C_MUTED)),
        Span::styled(runtime_str, Style::default().fg(C_TEXT)),
    ]));

    // Genres
    if !movie.genres.is_empty() {
        let genres: Vec<Span> = movie
            .genres
            .iter()
            .flat_map(|g| {
                vec![
                    Span::styled(g, Style::default().fg(C_ACCENT2)),
                    Span::styled(" · ", Style::default().fg(C_MUTED)),
                ]
            })
            .collect();
        let mut genre_line = vec![Span::styled("  Genres:  ", Style::default().fg(C_MUTED))];
        genre_line.extend(genres);
        lines.push(Line::from(genre_line));
    }

    // Director
    if !movie.director.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Director: ", Style::default().fg(C_MUTED)),
            Span::styled(movie.director.join(", "), Style::default().fg(C_TEXT)),
        ]));
    }

    // Cast
    if !movie.cast.is_empty() {
        let cast_list = movie.cast.iter().take(6).cloned().collect::<Vec<_>>().join(", ");
        lines.push(Line::from(vec![
            Span::styled("  Cast:    ", Style::default().fg(C_MUTED)),
            Span::styled(cast_list, Style::default().fg(C_TEXT)),
        ]));
    }

    // Language / country
    if !movie.language.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Language: ", Style::default().fg(C_MUTED)),
            Span::styled(movie.language.join(", "), Style::default().fg(C_TEXT)),
        ]));
    }

    if let Some(rd) = &movie.release_date {
        lines.push(Line::from(vec![
            Span::styled("  Released: ", Style::default().fg(C_MUTED)),
            Span::styled(rd, Style::default().fg(C_TEXT)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Plot:",
        Style::default().fg(C_MUTED).bold(),
    )));

    if let Some(plot) = &movie.plot {
        let max_width = (inner.width as usize).saturating_sub(4);
        for line in wrap_text(plot, max_width) {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(C_TEXT),
            )));
        }
    }

    lines.push(Line::from(""));

    // Keywords
    if !movie.keywords.is_empty() {
        let kw: Vec<Span> = movie
            .keywords
            .iter()
            .take(8)
            .flat_map(|k| {
                vec![
                    Span::styled(format!("[{}]", k), Style::default().fg(C_MUTED)),
                    Span::styled(" ", Style::default()),
                ]
            })
            .collect();
        let mut kw_line = vec![Span::styled("  Tags:   ", Style::default().fg(C_MUTED))];
        kw_line.extend(kw);
        lines.push(Line::from(kw_line));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  IMDB:   ", Style::default().fg(C_MUTED)),
        Span::styled(&movie.imdb_url, Style::default().fg(C_ACCENT2)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  [ Enter / w ]  Fetch streams from playimdb.com",
            Style::default().fg(C_GREEN).bold(),
        ),
    ]));

    // Scroll
    let total_lines = lines.len();
    let visible = inner.height as usize;
    let scroll = app.detail_scroll.min(total_lines.saturating_sub(visible));

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        inner,
    );

    // Scrollbar
    if total_lines > visible {
        let mut sb_state = ScrollbarState::new(total_lines).position(scroll);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(C_BORDER)),
            info_area.inner(Margin { horizontal: 0, vertical: 1 }),
            &mut sb_state,
        );
    }

    // Poster (Kitty) placeholder — actual image is written to stdout directly
    if let Some(poster_area) = poster_area_opt {
        let poster_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_SURFACE));
        let inner_poster = poster_block.inner(poster_area);
        f.render_widget(poster_block, poster_area);
        f.render_widget(
            Paragraph::new("\n\n\n  🖼️\n  Poster")
                .alignment(Alignment::Center)
                .style(Style::default().fg(C_MUTED)),
            inner_poster,
        );

        // Render Kitty graphics poster if available and not already drawn
        if let Some(bytes) = app.poster_cache.get(&movie.id) {
            let mut drawn = app.kitty_image_drawn.borrow_mut();
            if drawn.as_ref() != Some(&movie.id) {
                let _ = crate::kitty::clear_images();
                let _ = crossterm::queue!(
                    std::io::stdout(),
                    crossterm::cursor::MoveTo(inner_poster.x, inner_poster.y)
                );
                let _ = crate::kitty::display_image_bytes(bytes, inner_poster.width as u32, inner_poster.height as u32);
                *drawn = Some(movie.id.clone());
            }
        }
    }
}

// ─── Stream select ────────────────────────────────────────────────────────────

fn draw_stream_select(f: &mut Frame, app: &App, area: Rect) {
    let Some(info) = &app.stream_info else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    // Header info
    let header = Block::default()
        .title(Span::styled(" 🌐 Stream Sources ", Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE));
    let header_inner = header.inner(chunks[0]);
    f.render_widget(header, chunks[0]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Source: ", Style::default().fg(C_MUTED)),
            Span::styled(&info.stream_url, Style::default().fg(C_ACCENT2)),
        ])),
        header_inner,
    );

    // Options list
    let block = Block::default()
        .title(Span::styled(
            " ↑↓ Navigate  Enter=Play  D=Download  Esc=Back ",
            Style::default().fg(C_MUTED),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let mut items: Vec<ListItem> = Vec::new();

    // Quality options
    if !info.qualities.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  — Direct Streams —",
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))));
        for (i, q) in info.qualities.iter().enumerate() {
            let is_selected = i == app.selected_quality;
            let size_str = q
                .size_bytes
                .map(|s| format!("  📦 {}", format_size(s)))
                .unwrap_or_default();
            let content = format!(
                "  ▶  {:20}  [{}]{}",
                q.label, q.format, size_str
            );
            let style = if is_selected {
                Style::default().fg(C_BG).bg(C_ACCENT).bold()
            } else {
                Style::default().fg(C_TEXT)
            };
            items.push(ListItem::new(content).style(style));
        }
    }

    // Torrent links
    if !info.torrent_links.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  — Torrent / Magnet —",
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))));
        for (i, t) in info.torrent_links.iter().enumerate() {
            let abs_i = info.qualities.len() + i;
            let is_selected = abs_i == app.selected_quality;
            let size_str = t
                .size_bytes
                .map(|s| format!("  📦 {}", format_size(s)))
                .unwrap_or_default();
            let seeds = t
                .seeders
                .map(|s| format!("  🌱{}", s))
                .unwrap_or_default();
            let content = format!("  🧲  {}{}{}", truncate(&t.label, 30), size_str, seeds);
            let style = if is_selected {
                Style::default().fg(C_BG).bg(C_GREEN).bold()
            } else {
                Style::default().fg(C_GREEN)
            };
            items.push(ListItem::new(content).style(style));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  No streams found. Press Esc to go back.",
            Style::default().fg(C_RED),
        ))));
    }

    let mut state = ListState::default();
    state.select(Some(app.selected_quality));
    f.render_stateful_widget(List::new(items).block(block), chunks[1], &mut state);
}

// ─── Download progress ────────────────────────────────────────────────────────

fn draw_download_progress(f: &mut Frame, app: &App, area: Rect) {
    let Some(dp) = &app.download_progress else {
        return;
    };

    let block = Block::default()
        .title(Span::styled(" ⬇️  Downloading ", Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(format!("  File: {}", dp.filename))
            .style(Style::default().fg(C_TEXT)),
        chunks[0],
    );

    let ratio = dp
        .total
        .map(|t| dp.downloaded as f64 / t as f64)
        .unwrap_or(0.0);
    let pct = (ratio * 100.0) as u16;

    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(Style::default().fg(C_ACCENT).bg(C_SURFACE2))
            .percent(pct)
            .label(format!(
                "{} / {}",
                format_size(dp.downloaded),
                dp.total.map(format_size).unwrap_or_else(|| "?".into())
            )),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new(format!(
            "  Speed: {}   Ctrl+C to cancel",
            crate::downloader::format_speed(dp.speed)
        ))
        .style(Style::default().fg(C_MUTED)),
        chunks[2],
    );
}

// ─── Help screen ──────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " ❓ Watchie Help ",
            Style::default().fg(C_ACCENT).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let sections: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("Navigation", vec![
            ("j / ↓", "Move down"),
            ("k / ↑", "Move up"),
            ("PgDn / PgUp", "Page down / up"),
            ("g / Home", "Go to top"),
            ("G / End", "Go to bottom"),
            ("Enter", "Select / Open"),
            ("Esc / b / ←", "Go back"),
        ]),
        ("Browsing", vec![
            ("/  or  s", "Open search"),
            ("c", "Browse categories"),
            ("r", "Refresh current list"),
            ("i", "Toggle Kitty image preview"),
        ]),
        ("Streaming & Downloads", vec![
            ("p", "Play selected title"),
            ("d", "Download selected title"),
            ("w", "Fetch stream info (from detail)"),
            ("Enter (stream)", "Play stream in media player"),
            ("D (stream)", "Download stream"),
        ]),
        ("General", vec![
            ("?  /  F1", "Show this help"),
            ("q", "Quit"),
            ("Ctrl+C", "Force quit"),
        ]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (section, keys) in &sections {
        lines.push(Line::from(Span::styled(
            format!("  {} ", section),
            Style::default().fg(C_ACCENT).bold(),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(40)),
            Style::default().fg(C_BORDER),
        )));
        for (key, desc) in keys {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:18}", key), Style::default().fg(C_ACCENT2).bold()),
                Span::styled(desc.to_string(), Style::default().fg(C_TEXT)),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  Config file: ", Style::default().fg(C_MUTED)),
        Span::styled(
            crate::config::Config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".into()),
            Style::default().fg(C_ACCENT2),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Download dir: ", Style::default().fg(C_MUTED)),
        Span::styled(
            app.config.download_dir.display().to_string(),
            Style::default().fg(C_ACCENT2),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Player: ", Style::default().fg(C_MUTED)),
        Span::styled(&app.config.player.command, Style::default().fg(C_ACCENT2)),
    ]));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ─── Footer ───────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(inner);

    // Status message
    let (msg, style) = if let Some(s) = &app.status_msg {
        let color = match app.status_style {
            StatusStyle::Info => C_ACCENT2,
            StatusStyle::Success => C_GREEN,
            StatusStyle::Error => C_RED,
        };
        (s.as_str(), Style::default().fg(color))
    } else {
        ("q=quit  ?=help  /=search  c=categories", Style::default().fg(C_MUTED))
    };

    f.render_widget(
        Paragraph::new(Span::styled(msg, style)).alignment(Alignment::Left),
        chunks[0],
    );

    // Loading indicator
    let loading_txt = match &app.loading {
        LoadingState::Loading(_) => Span::styled("⟳ Loading…", Style::default().fg(C_ACCENT)),
        LoadingState::Error(msg) => Span::styled(format!("✕ Error: {}", msg), Style::default().fg(C_RED)),
        LoadingState::Idle => Span::styled(
            format!(
                "{}{}",
                if app.config.ui.kitty_images { "🖼️ " } else { "" },
                format!("v{}", env!("CARGO_PKG_VERSION"))
            ),
            Style::default().fg(C_MUTED),
        ),
    };
    f.render_widget(
        Paragraph::new(loading_txt).alignment(Alignment::Right),
        chunks[1],
    );
}

// ─── Loading overlay ──────────────────────────────────────────────────────────

fn draw_loading_overlay(f: &mut Frame, area: Rect, msg: &str) {
    let popup = centered_rect(50, 20, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(popup);
    f.render_widget(block, popup);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("⟳", Style::default().fg(C_ACCENT).bold())),
            Line::from(""),
            Line::from(Span::styled(msg, Style::default().fg(C_TEXT))),
        ])
        .alignment(Alignment::Center),
        inner,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn fmt_votes(v: u64) -> String {
    if v >= 1_000_000 {
        format!("{:.1}M", v as f64 / 1_000_000.0)
    } else if v >= 1_000 {
        format!("{:.0}K", v as f64 / 1_000.0)
    } else {
        v.to_string()
    }
}

fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in s.split_whitespace() {
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

fn draw_setup(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT2))
        .style(Style::default().bg(C_SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Welcome to ", Style::default().fg(C_TEXT)),
            Span::styled("watchie", Style::default().fg(C_ACCENT).bold()),
            Span::styled("!", Style::default().fg(C_TEXT)),
        ]),
        Line::from(""),
        Line::from("   To browse, search, and view movies, a free TMDB API key is required."),
        Line::from("   IMDB is heavily protected, so watchie uses TMDB to fetch movie metadata."),
        Line::from(""),
        Line::from(vec![
            Span::styled("   How to get a key (100% Free & Instant):", Style::default().fg(C_TEXT).bold())
        ]),
        Line::from("   1. Create a free account at https://www.themoviedb.org"),
        Line::from("   2. Go to your Account Settings -> API"),
        Line::from("   3. Create an API Key (choose 'Developer' option)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("   How to configure watchie:", Style::default().fg(C_TEXT).bold())
        ]),
        Line::from(vec![
            Span::styled("   - Option A: ", Style::default().fg(C_MUTED)),
            Span::styled("Run: ", Style::default().fg(C_TEXT)),
            Span::styled("watchie config set-tmdb-key <YOUR_KEY>", Style::default().fg(C_ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("   - Option B: ", Style::default().fg(C_MUTED)),
            Span::styled("Set env var: ", Style::default().fg(C_TEXT)),
            Span::styled("export TMDB_API_KEY=<YOUR_KEY>", Style::default().fg(C_ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("   - Option C: ", Style::default().fg(C_MUTED)),
            Span::styled("Directly edit: ", Style::default().fg(C_TEXT)),
            Span::styled("~/.config/watchie/config.toml", Style::default().fg(C_ACCENT2)),
        ]),
        Line::from(""),
        Line::from("   Press 'q' or 'Ctrl+C' to exit watchie."),
    ];

    f.render_widget(Paragraph::new(text), inner);
}

// ─── TV Season List screen ───────────────────────────────────────────────────

fn draw_season_list(f: &mut Frame, app: &App, area: Rect) {
    let Some(movie) = &app.current_movie else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left: Seasons List
    let block = Block::default()
        .title(Span::styled(" 📅 Seasons ", Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let items: Vec<ListItem> = app
        .season_list
        .iter()
        .enumerate()
        .map(|(i, season)| {
            let is_selected = i == app.selected_season;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD).bg(C_SURFACE2)
            } else {
                Style::default().fg(C_TEXT)
            };
            ListItem::new(format!("{}{}", prefix, season.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_season));
    f.render_stateful_widget(List::new(items).block(block), chunks[0], &mut state);

    // Right: Selected Season Details
    let details_block = Block::default()
        .title(Span::styled(" ℹ️  Season Details ", Style::default().fg(C_ACCENT2).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));
    
    let inner_area = details_block.inner(chunks[1]);
    f.render_widget(details_block, chunks[1]);

    if let Some(season) = app.season_list.get(app.selected_season) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Show:         ", Style::default().fg(C_MUTED)),
                Span::styled(&movie.title, Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Title:        ", Style::default().fg(C_MUTED)),
                Span::styled(&season.name, Style::default().fg(C_TEXT).bold()),
            ]),
            Line::from(vec![
                Span::styled("Season Num:   ", Style::default().fg(C_MUTED)),
                Span::styled(season.season_number.to_string(), Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Episodes:     ", Style::default().fg(C_MUTED)),
                Span::styled(season.episode_count.to_string(), Style::default().fg(C_ACCENT2).bold()),
            ]),
        ];

        if let Some(ref air_date) = season.air_date {
            lines.push(Line::from(vec![
                Span::styled("Air Date:     ", Style::default().fg(C_MUTED)),
                Span::styled(air_date, Style::default().fg(C_TEXT)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Press [Enter] to browse episodes", Style::default().fg(C_GREEN).bold())));
        lines.push(Line::from(Span::styled("Press [Esc] to go back to series detail", Style::default().fg(C_MUTED))));

        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            inner_area,
        );
    }
}

// ─── TV Episode List screen ──────────────────────────────────────────────────

fn draw_episode_list(f: &mut Frame, app: &App, area: Rect) {
    let Some(movie) = &app.current_movie else {
        return;
    };
    let Some(season) = app.season_list.get(app.selected_season) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // Left: Episodes List
    let title_str = format!(" 📺 {} - Episodes ", season.name);
    let block = Block::default()
        .title(Span::styled(title_str, Style::default().fg(C_ACCENT).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));

    let items: Vec<ListItem> = app
        .episode_list
        .iter()
        .enumerate()
        .map(|(i, ep)| {
            let is_selected = i == app.selected_episode;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD).bg(C_SURFACE2)
            } else {
                Style::default().fg(C_TEXT)
            };
            ListItem::new(format!("{}E{:02}: {}", prefix, ep.episode_number, ep.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_episode));
    f.render_stateful_widget(List::new(items).block(block), chunks[0], &mut state);

    // Right: Selected Episode details
    let details_block = Block::default()
        .title(Span::styled(" ℹ️  Episode Details ", Style::default().fg(C_ACCENT2).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_SURFACE));
    
    let inner_area = details_block.inner(chunks[1]);
    f.render_widget(details_block, chunks[1]);

    if let Some(episode) = app.episode_list.get(app.selected_episode) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Show:         ", Style::default().fg(C_MUTED)),
                Span::styled(&movie.title, Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Season / Ep:  ", Style::default().fg(C_MUTED)),
                Span::styled(format!("Season {}, Episode {}", episode.season_number, episode.episode_number), Style::default().fg(C_TEXT).bold()),
            ]),
            Line::from(vec![
                Span::styled("Title:        ", Style::default().fg(C_MUTED)),
                Span::styled(&episode.name, Style::default().fg(C_ACCENT2).bold()),
            ]),
        ];

        if let Some(ref air) = episode.air_date {
            lines.push(Line::from(vec![
                Span::styled("Air Date:     ", Style::default().fg(C_MUTED)),
                Span::styled(air, Style::default().fg(C_TEXT)),
            ]));
        }

        if let Some(rating) = episode.vote_average {
            lines.push(Line::from(vec![
                Span::styled("Rating:       ", Style::default().fg(C_MUTED)),
                Span::styled(format!("★ {:.1}/10", rating), Style::default().fg(C_ACCENT).bold()),
            ]));
        }

        if let Some(runtime) = episode.runtime {
            lines.push(Line::from(vec![
                Span::styled("Runtime:      ", Style::default().fg(C_MUTED)),
                Span::styled(format!("{} min", runtime), Style::default().fg(C_TEXT)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Overview:", Style::default().fg(C_MUTED).bold())));

        if let Some(ref overview) = episode.overview {
            let max_w = inner_area.width.saturating_sub(4) as usize;
            for line in wrap_text(overview, max_w) {
                lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(C_TEXT))));
            }
        } else {
            lines.push(Line::from(Span::styled("  No overview available.", Style::default().fg(C_MUTED))));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Press [Enter] / [p] to Play Episode", Style::default().fg(C_GREEN).bold())));
        lines.push(Line::from(Span::styled("Press [d] to Download Episode", Style::default().fg(C_ACCENT2).bold())));
        lines.push(Line::from(Span::styled("Press [Esc] to go back to season list", Style::default().fg(C_MUTED))));

        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            inner_area,
        );
    }
}
