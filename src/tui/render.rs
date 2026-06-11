use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState,
        Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use super::app::{App, InputMode, LoadingState, Screen, StatusStyle};
use super::theme::Theme;
use crate::playimdb::format_size;

// ─── Main draw ────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App) {
    let theme = Theme::from_name(&app.config.ui.theme);
    let area = f.area();

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(theme.bg)),
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

    draw_header(f, app, chunks[0], &theme);
    draw_body(f, app, chunks[1], &theme);
    draw_footer(f, app, chunks[2], &theme);

    // Loading overlay
    if let LoadingState::Loading(msg) = &app.loading {
        draw_loading_overlay(f, area, msg, &theme);
    }
}

// ─── Header ───────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.surface));

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

    let is_kitty = crate::kitty::is_kitty();
    let logo_text = if is_kitty { "󰟖 WATCHIE" } else { "🎬 WATCHIE" };
    let sep_text = if is_kitty { "  󰁔  " } else { "  ›  " };

    let logo = Span::styled(
        logo_text,
        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
    );
    let sep = Span::styled(sep_text, Style::default().fg(theme.muted));
    let breadcrumb_span = Span::styled(breadcrumb, Style::default().fg(theme.text));

    f.render_widget(
        Paragraph::new(Line::from(vec![logo, sep, breadcrumb_span]))
            .alignment(Alignment::Left),
        chunks[0],
    );

    // Search input or mode hint
    let hint = if app.input_mode == InputMode::Searching {
        let cursor = if is_kitty { "󰇄" } else { "█" };
        Span::styled(
            format!(" / {}{}", app.search_query, cursor),
            Style::default().fg(theme.accent2).add_modifier(Modifier::BOLD),
        )
    } else {
        let search_icon = if is_kitty { "󰍉" } else { "/" };
        let help_icon = if is_kitty { "󰋖" } else { "?" };
        Span::styled(format!(" {} to search  {} help", search_icon, help_icon), Style::default().fg(theme.muted))
    };
    f.render_widget(
        Paragraph::new(Line::from(hint)).alignment(Alignment::Right),
        chunks[1],
    );
}

// ─── Body ─────────────────────────────────────────────────────────────────────

fn draw_body(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    match app.screen {
        Screen::Home => draw_home(f, app, area, theme),
        Screen::CategoryList => draw_category_list(f, app, area, theme),
        Screen::MovieList | Screen::Search => draw_movie_list(f, app, area, theme),
        Screen::MovieDetail => draw_movie_detail(f, app, area, theme),
        Screen::SeasonList => draw_season_list(f, app, area, theme),
        Screen::EpisodeList => draw_episode_list(f, app, area, theme),
        Screen::StreamSelect => draw_stream_select(f, app, area, theme),
        Screen::DownloadProgress => draw_download_progress(f, app, area, theme),
        Screen::Help => draw_help(f, app, area, theme),
        Screen::Setup => draw_setup(f, app, area, theme),
    }
}

// ─── Home screen ──────────────────────────────────────────────────────────────

fn draw_home(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Category list on left
    draw_category_panel(f, app, chunks[0], theme);

    // Quick stats / art on right
    draw_home_art(f, app, chunks[1], theme);
}

fn draw_category_panel(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let is_kitty = crate::kitty::is_kitty();
    let cat_icon = if is_kitty { "󰉋" } else { "📂" };
    let block = Block::default()
        .title(Span::styled(format!(" {} Categories ", cat_icon), Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_selected = i == app.selected_category;
            let prefix = if is_selected { if is_kitty { "󰁔 " } else { "▶ " } } else { "  " };
            let style = if is_selected {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD).bg(theme.surface2)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(format!("{}{}", prefix, cat.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_category));

    f.render_stateful_widget(List::new(items).block(block), area, &mut state);
}

fn draw_home_art(f: &mut Frame, _app: &App, area: Rect, theme: &Theme) {
    let is_kitty = crate::kitty::is_kitty();
    let art = if is_kitty {
        vec![
            "",
            "  ╔══════════════════════════════╗",
            "  ║                              ║",
            "  ║   󰟖  Browse IMDB Catalog    ║",
            "  ║   󰖟  Stream via playimdb    ║",
            "  ║   󰇚   Download in any dir   ║",
            "  ║   󰋩   Kitty image previews  ║",
            "  ║                              ║",
            "  ╚══════════════════════════════╝",
            "",
            "  Quick Keys:",
            "  󰘳  Enter  → Open / Select",
            "  󰍉  /      → Search IMDB",
            "  󰉋  c      → Browse categories",
            "  󰐊  p      → Play selected",
            "  󰇚  d      → Download selected",
            "  󰋖  ?      → Help",
            "  󰈆  q      → Quit",
        ]
    } else {
        vec![
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
        ]
    };

    let lines: Vec<Line> = art
        .iter()
        .map(|l| {
            if l.contains("󰟖") || l.contains("󰖟") || l.contains("󰇚") || l.contains("󰋩") ||
               l.contains("🎬") || l.contains("🌐") || l.contains("⬇️") || l.contains("🖼️") {
                Line::from(Span::styled(*l, Style::default().fg(theme.accent2)))
            } else if l.starts_with("  Quick") {
                Line::from(Span::styled(*l, Style::default().fg(theme.accent).bold()))
            } else if l.contains("→") {
                let parts: Vec<&str> = l.splitn(2, "→").collect();
                Line::from(vec![
                    Span::styled(parts[0], Style::default().fg(theme.accent).bold()),
                    Span::styled("→", Style::default().fg(theme.muted)),
                    Span::styled(parts.get(1).copied().unwrap_or(""), Style::default().fg(theme.text)),
                ])
            } else {
                Line::from(Span::styled(*l, Style::default().fg(theme.muted)))
            }
        })
        .collect();

    let movie_icon = if is_kitty { "󰟖" } else { "🎥" };
    let block = Block::default()
        .title(Span::styled(format!(" {} Watchie ", movie_icon), Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

// ─── Category list ────────────────────────────────────────────────────────────

fn draw_category_list(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let is_kitty = crate::kitty::is_kitty();
    let cat_icon = if is_kitty { "󰉋" } else { "📂" };
    
    // Left: Category List
    let block = Block::default()
        .title(Span::styled(format!(" {} Browse Categories ", cat_icon), Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_selected = i == app.selected_category;
            let icon = category_icon(cat.id);
            let style = if is_selected {
                Style::default().fg(theme.bg).bg(theme.accent).bold()
            } else {
                Style::default().fg(theme.text)
            };
            let line = format!(" {} {} ", icon, cat.name);
            ListItem::new(line).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_category));
    f.render_stateful_widget(List::new(items).block(block), chunks[0], &mut state);

    // Right: Category Preview/Info
    draw_category_preview(f, app, chunks[1], theme);
}

fn draw_category_preview(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let cat = &app.categories[app.selected_category];
    let is_kitty = crate::kitty::is_kitty();
    let icon = category_icon(cat.id);
    
    let block = Block::default()
        .title(Span::styled(format!(" {} Preview ", icon), Style::default().fg(theme.accent2).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let desc = match cat.id {
        "moviemeter" => "Trending movies right now based on IMDB traffic.",
        "top" => "Highest rated movies of all time.",
        "toptv" => "The greatest TV series ever made.",
        "boxoffice" => "The top-grossing movies in theaters this weekend.",
        "comingsoon" => "Anticipated movies hitting the screens very soon.",
        "oscar-winners" => "Legendary movies that have earned an Academy Award.",
        id if id.contains("action") => "Explosive thrills, fast-paced chases, and heroic feats.",
        id if id.contains("adventure") => "Epic journeys and explorations of unknown worlds.",
        id if id.contains("animation") => "Masterpieces of visual storytelling from top studios.",
        id if id.contains("comedy") => "The best humor, from sitcoms to satirical films.",
        _ => "Explore our curated catalog and stream in high quality.",
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("   {} ", icon), Style::default().fg(theme.accent).bold()),
            Span::styled(cat.name, Style::default().fg(theme.text).bold().add_modifier(Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Category Info:", Style::default().fg(theme.muted).bold())
        ]),
    ];

    let max_w = inner.width.saturating_sub(6) as usize;
    for line in wrap_text(desc, max_w) {
        lines.push(Line::from(format!("   {}", line)));
    }

    lines.push(Line::from(""));
    let ent_icon = if is_kitty { "󰘳" } else { "Enter" };
    lines.push(Line::from(vec![
        Span::styled(format!("   Press [{}] to explore {}...", ent_icon, cat.name), Style::default().fg(theme.accent2))
    ]));

    f.render_widget(Paragraph::new(lines), inner);
}

fn category_icon(id: &str) -> &'static str {
    let is_kitty = crate::kitty::is_kitty();
    match id {
        "top" | "toptv" => if is_kitty { "󰓎" } else { "⭐" },
        "moviemeter" => if is_kitty { "󰈸" } else { "🔥" },
        "boxoffice" => if is_kitty { "󰠭" } else { "💰" },
        "comingsoon" => if is_kitty { "󰃭" } else { "📅" },
        "oscar-winners" => if is_kitty { "󰘔" } else { "🏆" },
        id if id.contains("action") => if is_kitty { "󰓅" } else { "💥" },
        id if id.contains("adventure") => if is_kitty { "󰙠" } else { "🗺️" },
        id if id.contains("animation") => if is_kitty { "󰈄" } else { "🎨" },
        id if id.contains("comedy") => if is_kitty { "󰅴" } else { "😂" },
        id if id.contains("crime") => if is_kitty { "󰆠" } else { "🔫" },
        id if id.contains("documentary") => if is_kitty { "󰈫" } else { "📹" },
        id if id.contains("drama") => if is_kitty { "󰏤" } else { "🎭" },
        id if id.contains("fantasy") => if is_kitty { "󰝯" } else { "🧙" },
        id if id.contains("horror") => if is_kitty { "󰝟" } else { "👻" },
        id if id.contains("mystery") => if is_kitty { "󰍉" } else { "🔍" },
        id if id.contains("romance") => if is_kitty { "󰓏" } else { "❤️" },
        id if id.contains("sci-fi") => if is_kitty { "󰈡" } else { "🚀" },
        id if id.contains("thriller") => if is_kitty { "󰇄" } else { "😰" },
        id if id.contains("western") => if is_kitty { "󰖑" } else { "🤠" },
        _ => if is_kitty { "󰟖" } else { "🎬" },
    }
}

// ─── Movie list ───────────────────────────────────────────────────────────────

fn draw_movie_list(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let is_kitty = crate::kitty::is_kitty();
    let list = app.current_list();
    let title_text = if app.screen == Screen::Search {
        let search_icon = if is_kitty { "󰍉" } else { "🔍" };
        format!(" {} Results for \"{}\" ({}) ", search_icon, app.search_query, list.len())
    } else {
        let movie_icon = if is_kitty { "󰟖" } else { "🎬" };
        format!(" {} {} ({}) ", movie_icon, app.list_title, list.len())
    };

    // Left Panel: The List
    let block = Block::default()
        .title(Span::styled(title_text, Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let inner_list = block.inner(chunks[0]);
    f.render_widget(block, chunks[0]);

    if list.is_empty() {
        let hint = if is_kitty { "\n  󰍉 No results yet. Press 󰘳 Enter on a category or use / to search." } else { "\n  No results yet. Press Enter on a category or use / to search." };
        f.render_widget(
            Paragraph::new(hint)
                .style(Style::default().fg(theme.muted))
                .alignment(Alignment::Left),
            inner_list,
        );
    } else {
        // Column header
        let header_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner_list);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{:<4} ", "#"), Style::default().fg(theme.muted)),
                Span::styled(format!("{:<25}", "Title"), Style::default().fg(theme.muted)),
                Span::styled(format!("{:<6}", "Year"), Style::default().fg(theme.muted)),
                Span::styled(format!("{:<8}", "Rating"), Style::default().fg(theme.muted)),
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
                let rating_icon = if is_kitty { "󰓎" } else { "★" };
                let rating = r
                    .rating
                    .map(|rt| format!("{}{:.1}", rating_icon, rt))
                    .unwrap_or_else(|| "─".into());
                let title = truncate(&r.title, 24);

                let num_str = format!("{:>3}. ", abs_i + 1);

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::styled(num_str, Style::default().fg(theme.accent).bold()),
                        Span::styled(
                            format!("{:<25}", title),
                            Style::default().fg(theme.bg).bg(theme.accent).bold(),
                        ),
                        Span::styled(
                            format!("{:<6}", year),
                            Style::default().fg(theme.bg).bg(theme.accent),
                        ),
                        Span::styled(
                            format!("{:<8}", rating),
                            Style::default().fg(theme.bg).bg(theme.accent),
                        ),
                    ]))
                } else {
                    let title_color = if abs_i.is_multiple_of(2) { theme.text } else { theme.muted };
                    ListItem::new(Line::from(vec![
                        Span::styled(num_str, Style::default().fg(theme.muted)),
                        Span::styled(format!("{:<25}", title), Style::default().fg(title_color)),
                        Span::styled(format!("{:<6}", year), Style::default().fg(theme.muted)),
                        Span::styled(
                            format!("{:<8}", rating),
                            Style::default().fg(theme.accent),
                        ),
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
                .style(Style::default().fg(theme.border)),
            chunks[0].inner(Margin { horizontal: 0, vertical: 1 }),
            &mut scroll_state,
        );
    }

    // Right Panel: The Preview
    draw_movie_preview(f, app, chunks[1], theme);
}

fn draw_movie_preview(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let result = app.selected_result();
    let is_kitty = crate::kitty::is_kitty();
    let preview_icon = if is_kitty { "󰟖" } else { "🎬" };

    let block = Block::default()
        .title(Span::styled(format!(" {} Quick Preview ", preview_icon), Style::default().fg(theme.accent2).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(res) = result {
        // Layout: Top = Metadata, Bottom = Poster
        let preview_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(7), Constraint::Min(0)])
            .split(inner);

        // Metadata
        let rating_icon = if is_kitty { "󰓎" } else { "★" };
        let rating = res.rating.map(|r| format!("{} {:.1}/10", rating_icon, r)).unwrap_or_else(|| "N/A".into());
        let year = res.year.map(|y| y.to_string()).unwrap_or_else(|| "Unknown".into());

        let lines = vec![
            Line::from(vec![
                Span::styled("  Title:    ", Style::default().fg(theme.muted)),
                Span::styled(&res.title, Style::default().fg(theme.text).bold()),
            ]),
            Line::from(vec![
                Span::styled("  Year:     ", Style::default().fg(theme.muted)),
                Span::styled(year, Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("  Rating:   ", Style::default().fg(theme.muted)),
                Span::styled(rating, Style::default().fg(theme.accent).bold()),
            ]),
            Line::from(vec![
                Span::styled("  Type:     ", Style::default().fg(theme.muted)),
                Span::styled(res.content_type.to_string(), Style::default().fg(theme.purple)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ Enter ] View full detail & streams", Style::default().fg(theme.accent2).add_modifier(Modifier::ITALIC)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), preview_chunks[0]);

        // Poster
        if app.config.ui.kitty_images && area.width > 30 {
            let poster_area = centered_rect(60, 80, preview_chunks[1]);
            let poster_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border));
            let inner_poster = poster_block.inner(poster_area);
            f.render_widget(poster_block, poster_area);

            if let Some(bytes) = app.poster_cache.get(&res.id) {
                let mut drawn = app.kitty_image_drawn.borrow_mut();
                if drawn.as_ref() != Some(&res.id) {
                    let _ = crate::kitty::clear_images();
                    let _ = crossterm::queue!(
                        std::io::stdout(),
                        crossterm::cursor::MoveTo(inner_poster.x, inner_poster.y)
                    );
                    let _ = crate::kitty::display_image_bytes(bytes, inner_poster.width as u32, inner_poster.height as u32);
                    *drawn = Some(res.id.clone());
                }
            } else {
                let hint = if is_kitty { "\n\n  󰋩\n Loading..." } else { "\n\n  🖼️\n Loading..." };
                f.render_widget(
                    Paragraph::new(hint).alignment(Alignment::Center).style(Style::default().fg(theme.muted)),
                    inner_poster,
                );
            }
        }
    } else {
        f.render_widget(
            Paragraph::new("Select a title to see details").alignment(Alignment::Center).style(Style::default().fg(theme.muted)),
            inner,
        );
    }
}

// ─── Movie detail ─────────────────────────────────────────────────────────────

fn draw_movie_detail(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let Some(movie) = &app.current_movie else {
        f.render_widget(
            Paragraph::new("Loading…").style(Style::default().fg(theme.muted)),
            area,
        );
        return;
    };

    let is_kitty = crate::kitty::is_kitty();

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
    let movie_icon = if is_kitty { "󰟖" } else { "🎬" };
    let block = Block::default()
        .title(Span::styled(
            format!(" {} {} ", movie_icon, movie.title),
            Style::default().fg(theme.accent).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(info_area);
    f.render_widget(block, info_area);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // Title + year
    lines.push(Line::from(vec![
        Span::styled(&movie.title, Style::default().fg(theme.accent).bold().add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled(
            movie.year.map(|y| format!("({})", y)).unwrap_or_default(),
            Style::default().fg(theme.muted),
        ),
    ]));

    if let Some(tl) = &movie.tagline {
        lines.push(Line::from(Span::styled(
            format!("\"{}\"", tl),
            Style::default().fg(theme.accent2).add_modifier(Modifier::ITALIC),
        )));
    }

    lines.push(Line::from(""));

    // Rating row
    let rating_icon = if is_kitty { "󰓎" } else { "★" };
    let rating_str = movie
        .rating
        .map(|r| format!("{} {:.1}/10", rating_icon, r))
        .unwrap_or_else(|| "No rating".into());
    let votes_str = movie
        .votes
        .map(|v| format!(" ({} votes)", fmt_votes(v)))
        .unwrap_or_default();
    
    let mut rating_line = vec![
        Span::styled("  Rating:  ", Style::default().fg(theme.muted)),
        Span::styled(rating_str, Style::default().fg(theme.accent).bold()),
        Span::styled(votes_str, Style::default().fg(theme.muted)),
    ];

    let check_icon = if is_kitty { "󰖟" } else { "PlayIMDb:" };
    rating_line.push(Span::styled(format!("    {} ", check_icon), Style::default().fg(theme.muted)));
    match &app.playimdb_status {
        super::app::PlayImdbStatus::Unknown => {
            rating_line.push(Span::styled("❓ Unknown", Style::default().fg(theme.muted)));
        }
        super::app::PlayImdbStatus::Checking => {
            rating_line.push(Span::styled("⏳ Checking...", Style::default().fg(theme.accent2).bold()));
        }
        super::app::PlayImdbStatus::Available => {
            let ok_icon = if is_kitty { "󰄬 Available" } else { "🟢 Available" };
            rating_line.push(Span::styled(ok_icon, Style::default().fg(theme.green).bold()));
        }
        super::app::PlayImdbStatus::NotAvailable => {
            let err_icon = if is_kitty { "󰅖 Not Found" } else { "🔴 Not Found" };
            rating_line.push(Span::styled(err_icon, Style::default().fg(theme.red).bold()));
        }
        super::app::PlayImdbStatus::Error(e) => {
            rating_line.push(Span::styled(format!("⚠️ Error ({})", e), Style::default().fg(theme.red)));
        }
    }
    
    lines.push(Line::from(rating_line));

    // Type + runtime
    let runtime_str = movie
        .runtime
        .map(|r| format!("{} min", r))
        .unwrap_or_else(|| "–".into());
    lines.push(Line::from(vec![
        Span::styled("  Type:    ", Style::default().fg(theme.muted)),
        Span::styled(movie.content_type.to_string(), Style::default().fg(theme.purple)),
        Span::styled("   Runtime: ", Style::default().fg(theme.muted)),
        Span::styled(runtime_str, Style::default().fg(theme.text)),
    ]));

    // Genres
    if !movie.genres.is_empty() {
        let sep_sym = if is_kitty { "󰇙" } else { "·" };
        let genres: Vec<Span> = movie
            .genres
            .iter()
            .flat_map(|g| {
                vec![
                    Span::styled(g, Style::default().fg(theme.accent2)),
                    Span::styled(format!(" {} ", sep_sym), Style::default().fg(theme.muted)),
                ]
            })
            .collect();
        let mut genre_line = vec![Span::styled("  Genres:  ", Style::default().fg(theme.muted))];
        genre_line.extend(genres);
        lines.push(Line::from(genre_line));
    }

    // Director
    if !movie.director.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Director: ", Style::default().fg(theme.muted)),
            Span::styled(movie.director.join(", "), Style::default().fg(theme.text)),
        ]));
    }

    // Cast
    if !movie.cast.is_empty() {
        let cast_list = movie.cast.iter().take(6).cloned().collect::<Vec<_>>().join(", ");
        lines.push(Line::from(vec![
            Span::styled("  Cast:    ", Style::default().fg(theme.muted)),
            Span::styled(cast_list, Style::default().fg(theme.text)),
        ]));
    }

    // Language / country
    if !movie.language.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Language: ", Style::default().fg(theme.muted)),
            Span::styled(movie.language.join(", "), Style::default().fg(theme.text)),
        ]));
    }

    if let Some(rd) = &movie.release_date {
        lines.push(Line::from(vec![
            Span::styled("  Released: ", Style::default().fg(theme.muted)),
            Span::styled(rd, Style::default().fg(theme.text)),
        ]));
    }

    lines.push(Line::from(""));
    let plot_icon = if is_kitty { "󰦨" } else { "" };
    lines.push(Line::from(Span::styled(
        format!("  {} Plot:", plot_icon),
        Style::default().fg(theme.muted).bold(),
    )));

    if let Some(plot) = &movie.plot {
        let max_width = (inner.width as usize).saturating_sub(4);
        for line in wrap_text(plot, max_width) {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme.text),
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
                    Span::styled(format!("[{}]", k), Style::default().fg(theme.muted)),
                    Span::styled(" ", Style::default()),
                ]
            })
            .collect();
        let mut kw_line = vec![Span::styled("  Tags:   ", Style::default().fg(theme.muted))];
        kw_line.extend(kw);
        lines.push(Line::from(kw_line));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  IMDB:   ", Style::default().fg(theme.muted)),
        Span::styled(&movie.imdb_url, Style::default().fg(theme.accent2)),
    ]));

    lines.push(Line::from(""));
    let play_btn = if is_kitty { "󰐊" } else { "Enter / w" };
    lines.push(Line::from(vec![
        Span::styled(
            format!("  [ {} ]  Fetch streams from playimdb.com", play_btn),
            Style::default().fg(theme.green).bold(),
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
                .style(Style::default().fg(theme.border)),
            info_area.inner(Margin { horizontal: 0, vertical: 1 }),
            &mut sb_state,
        );
    }

    // Poster (Kitty) placeholder — actual image is written to stdout directly
    if let Some(poster_area) = poster_area_opt {
        let poster_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.surface));
        let inner_poster = poster_block.inner(poster_area);
        f.render_widget(poster_block, poster_area);
        let poster_hint = if is_kitty { "\n\n\n  󰋩\n  Poster" } else { "\n\n\n  🖼️\n  Poster" };
        f.render_widget(
            Paragraph::new(poster_hint)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.muted)),
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

fn draw_stream_select(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let Some(info) = &app.stream_info else {
        return;
    };

    let is_kitty = crate::kitty::is_kitty();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    // Header info
    let stream_icon = if is_kitty { "󰖟" } else { "🌐" };
    let header = Block::default()
        .title(Span::styled(format!(" {} Stream Sources ", stream_icon), Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.surface));
    let header_inner = header.inner(chunks[0]);
    f.render_widget(header, chunks[0]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Source: ", Style::default().fg(theme.muted)),
            Span::styled(&info.stream_url, Style::default().fg(theme.accent2)),
        ])),
        header_inner,
    );

    // Options list
    let nav_hint = if is_kitty { " 󰁝󰁅 Navigate  󰘳 Enter=Play  󰇚 D=Download  󰈆 Esc=Back " } else { " ↑↓ Navigate  Enter=Play  D=Download  Esc=Back " };
    let block = Block::default()
        .title(Span::styled(
            nav_hint,
            Style::default().fg(theme.muted),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let mut items: Vec<ListItem> = Vec::new();

    // Quality options
    if !info.qualities.is_empty() {
        let label = if is_kitty { "  󰇙 Direct Streams 󰇙" } else { "  — Direct Streams —" };
        items.push(ListItem::new(Line::from(Span::styled(
            label,
            Style::default().fg(theme.muted).add_modifier(Modifier::ITALIC),
        ))));
        for (i, q) in info.qualities.iter().enumerate() {
            let is_selected = i == app.selected_quality;
            let size_icon = if is_kitty { "󰠭" } else { "📦" };
            let size_str = q
                .size_bytes
                .map(|s| format!("  {} {}", size_icon, format_size(s)))
                .unwrap_or_default();
            let play_icon = if is_kitty { "󰐊" } else { "▶" };
            let content = format!(
                "  {}  {:20}  [{}]{}",
                play_icon, q.label, q.format, size_str
            );
            let style = if is_selected {
                Style::default().fg(theme.bg).bg(theme.accent).bold()
            } else {
                Style::default().fg(theme.text)
            };
            items.push(ListItem::new(content).style(style));
        }
    }

    // Torrent links
    if !info.torrent_links.is_empty() {
        let label = if is_kitty { "  󰇙 Torrent / Magnet 󰇙" } else { "  — Torrent / Magnet —" };
        items.push(ListItem::new(Line::from(Span::styled(
            label,
            Style::default().fg(theme.muted).add_modifier(Modifier::ITALIC),
        ))));
        for (i, t) in info.torrent_links.iter().enumerate() {
            let abs_i = info.qualities.len() + i;
            let is_selected = abs_i == app.selected_quality;
            let size_icon = if is_kitty { "󰠭" } else { "📦" };
            let seed_icon = if is_kitty { "󰓠" } else { "🌱" };
            let size_str = t
                .size_bytes
                .map(|s| format!("  {} {}", size_icon, format_size(s)))
                .unwrap_or_default();
            let seeds = t
                .seeders
                .map(|s| format!("  {}{}", seed_icon, s))
                .unwrap_or_default();
            let mag_icon = if is_kitty { "󰈡" } else { "🧲" };
            let content = format!("  {}  {}{}{}", mag_icon, truncate(&t.label, 30), size_str, seeds);
            let style = if is_selected {
                Style::default().fg(theme.bg).bg(theme.green).bold()
            } else {
                Style::default().fg(theme.green)
            };
            items.push(ListItem::new(content).style(style));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  No streams found. Press Esc to go back.",
            Style::default().fg(theme.red),
        ))));
    }

    let mut state = ListState::default();
    state.select(Some(app.selected_quality));
    f.render_stateful_widget(List::new(items).block(block), chunks[1], &mut state);
}

// ─── Download progress ────────────────────────────────────────────────────────

fn draw_download_progress(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let Some(dp) = &app.download_progress else {
        return;
    };

    let is_kitty = crate::kitty::is_kitty();
    let dl_icon = if is_kitty { "󰇚" } else { "⬇️" };
    let block = Block::default()
        .title(Span::styled(format!(" {} Downloading ", dl_icon), Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.surface));

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
            .style(Style::default().fg(theme.text)),
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
            .gauge_style(Style::default().fg(theme.accent).bg(theme.surface2))
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
        .style(Style::default().fg(theme.muted)),
        chunks[2],
    );
}

// ─── Help screen ──────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let is_kitty = crate::kitty::is_kitty();
    let help_icon = if is_kitty { "󰋖" } else { "❓" };
    let block = Block::default()
        .title(Span::styled(
            format!(" {} Watchie Help ", help_icon),
            Style::default().fg(theme.accent).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.surface));

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
            ("o", "Open standalone stream in browser"),
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
            Style::default().fg(theme.accent).bold(),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(40)),
            Style::default().fg(theme.border),
        )));
        for (key, desc) in keys {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:18}", key), Style::default().fg(theme.accent2).bold()),
                Span::styled(desc.to_string(), Style::default().fg(theme.text)),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  Config file: ", Style::default().fg(theme.muted)),
        Span::styled(
            crate::config::Config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".into()),
            Style::default().fg(theme.accent2),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Download dir: ", Style::default().fg(theme.muted)),
        Span::styled(
            app.config.download_dir.display().to_string(),
            Style::default().fg(theme.accent2),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Player: ", Style::default().fg(theme.muted)),
        Span::styled(&app.config.player.command, Style::default().fg(theme.accent2)),
    ]));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ─── Footer ───────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Dynamic constraints based on terminal width
    let constraints = if area.width >= 120 {
        vec![
            Constraint::Min(40),
            Constraint::Length(30),
            Constraint::Length(45),
        ]
    } else if area.width >= 80 {
        vec![
            Constraint::Min(30),
            Constraint::Length(22),
            Constraint::Length(30),
        ]
    } else {
        vec![
            Constraint::Min(20),
            Constraint::Length(0),
            Constraint::Length(25),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    // 1. Dynamic context-sensitive key bindings
    let is_kitty = crate::kitty::is_kitty();
    let help_keys = match app.screen {
        Screen::Home => if is_kitty { "󰘳 Browse • 󰍉 Search • 󰉋 Categories • 󰈆 Quit" } else { "Enter: Browse • /: Search • c: Categories • q: Quit" },
        Screen::CategoryList => if is_kitty { "󰘳 Select • 󰈆 Back • 󰈆 Quit" } else { "Enter: Select • Esc/b: Back • q: Quit" },
        Screen::MovieList => if is_kitty { "󰘳 Detail • 󰐊 Play • 󰇚 Download • 󰉋 Categories • 󰍉 Search • 󰈆 Quit" } else { "Enter: Detail • p: Play • d: Download • c: Categories • /: Search • q: Quit" },
        Screen::MovieDetail => if is_kitty { "󰖟 Streams • 󰈆 Back • 󰋩 Toggle • 󰖟 Browser • 󰈆 Quit" } else { "w: Streams • Esc/b: Back • i: Image Toggle • o: Browser • q: Quit" },
        Screen::SeasonList => if is_kitty { "󰘳 Episodes • 󰈆 Back • 󰈆 Quit" } else { "Enter: Episodes • Esc/b: Back • q: Quit" },
        Screen::EpisodeList => if is_kitty { "󰘳 Streams • 󰐊 Play • 󰇚 Download • 󰈆 Back • 󰖟 Browser • 󰈆 Quit" } else { "Enter: Streams • p: Play • d: Download • Esc/b: Back • o: Browser • q: Quit" },
        Screen::StreamSelect => if is_kitty { "󰘳 Play • 󰇚 Download • 󰈆 Back • 󰖟 Browser • 󰈆 Quit" } else { "Enter: Play • D: Download • Esc/b: Back • o: Browser • q: Quit" },
        Screen::DownloadProgress => if is_kitty { "󰈆 Back • 󰈆 Quit" } else { "Esc/b: Back • q: Quit" },
        Screen::Search => if is_kitty { "Type query • 󰘳 Search • 󰈆 Cancel" } else { "Type query • Enter: Search • Esc: Cancel" },
        Screen::Help => if is_kitty { "󰈆 Back • 󰈆 Quit" } else { "Esc/b: Back • q: Quit" },
        Screen::Setup => if is_kitty { "Type TMDB Key • 󰘳 Confirm • 󰈆 Quit" } else { "Type TMDB Key • Enter: Confirm • q: Quit" },
    };

    // Status message overrides help keys if active
    let (msg, style) = if let Some(s) = &app.status_msg {
        let color = match app.status_style {
            StatusStyle::Info => theme.accent2,
            StatusStyle::Success => theme.green,
            StatusStyle::Error => theme.red,
        };
        (s.as_str(), Style::default().fg(color))
    } else {
        (help_keys, Style::default().fg(theme.muted))
    };

    f.render_widget(
        Paragraph::new(Span::styled(msg, style)).alignment(Alignment::Left),
        chunks[0],
    );

    // 2. Middle Section (Context/Selection Status) - hidden on very narrow screens
    if chunks.len() > 2 && chunks[1].width > 0 {
        let context_txt = match app.screen {
            Screen::MovieList => {
                let total = app.movie_list.len();
                let current = if total > 0 { app.selected_movie + 1 } else { 0 };
                let movie_icon = if is_kitty { "󰟖" } else { "🎬" };
                format!("{} {}: {}/{}", movie_icon, app.list_title, current, total)
            }
            Screen::MovieDetail => {
                if let Some(ref m) = app.current_movie {
                    let movie_icon = if is_kitty { "󰟖" } else { "🎥" };
                    format!("{} Detail: {}", movie_icon, m.title)
                } else {
                    let movie_icon = if is_kitty { "󰟖" } else { "🎥" };
                    format!("{} Movie Detail", movie_icon)
                }
            }
            Screen::SeasonList => {
                let total = app.season_list.len();
                let current = if total > 0 { app.selected_season + 1 } else { 0 };
                let tv_icon = if is_kitty { "󰏤" } else { "📺" };
                format!("{} Seasons: {}/{}", tv_icon, current, total)
            }
            Screen::EpisodeList => {
                let total = app.episode_list.len();
                let current = if total > 0 { app.selected_episode + 1 } else { 0 };
                let tv_icon = if is_kitty { "󰏤" } else { "📺" };
                format!("{} Episodes: {}/{}", tv_icon, current, total)
            }
            Screen::StreamSelect => {
                let total = app.stream_info.as_ref().map(|s| s.qualities.len() + s.torrent_links.len()).unwrap_or(0);
                let current = if total > 0 { app.selected_quality + 1 } else { 0 };
                let stream_icon = if is_kitty { "󰖟" } else { "🌐" };
                format!("{} Streams: {}/{}", stream_icon, current, total)
            }
            Screen::CategoryList => {
                let total = app.categories.len();
                let current = if total > 0 { app.selected_category + 1 } else { 0 };
                let cat_icon = if is_kitty { "󰉋" } else { "📁" };
                format!("{} Categories: {}/{}", cat_icon, current, total)
            }
            Screen::DownloadProgress => {
                if let Some(ref dp) = app.download_progress {
                    let dl_icon = if is_kitty { "󰇚" } else { "⬇️" };
                    format!("{} {}", dl_icon, dp.filename)
                } else {
                    let dl_icon = if is_kitty { "󰇚" } else { "⬇️" };
                    format!("{} Downloading", dl_icon)
                }
            }
            Screen::Home => if is_kitty { "󰋜 Home".to_string() } else { "🏠 Home".to_string() },
            Screen::Search => {
                let search_icon = if is_kitty { "󰍉" } else { "🔍" };
                format!("{} Search: \"{}\"", search_icon, app.search_query)
            },
            Screen::Setup => if is_kitty { "󰒓 Setup".to_string() } else { "⚙️ Setup".to_string() },
            Screen::Help => if is_kitty { "󰋖 Help".to_string() } else { "❓ Help".to_string() },
        };

        f.render_widget(
            Paragraph::new(Span::styled(context_txt, Style::default().fg(theme.accent2))).alignment(Alignment::Center),
            chunks[1],
        );
    }

    // 3. Right Section (Config & System Info)
    let right_idx = if chunks.len() > 2 { 2 } else { 1 };
    
    let player = &app.config.player.command;
    let download_dir_str = app.config.download_dir.to_string_lossy();
    let display_dir = if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if download_dir_str.starts_with(&*home_str) {
            download_dir_str.replacen(&*home_str, "~", 1)
        } else {
            download_dir_str.into_owned()
        }
    } else {
        download_dir_str.into_owned()
    };

    let play_icon = if is_kitty { "󰐊 " } else { "🚀 " };
    let dir_icon = if is_kitty { "󰉋 " } else { "📁 " };

    let config_status = if chunks[right_idx].width >= 35 {
        format!("{}{}{} • {}{}  ", play_icon, player, if is_kitty { "" } else { "" }, dir_icon, display_dir)
    } else {
        format!("{}{}{}  ", play_icon, player, if is_kitty { "" } else { "" })
    };

    let mut right_spans = vec![
        Span::styled(config_status, Style::default().fg(theme.muted)),
    ];

    match &app.loading {
        LoadingState::Loading(_) => {
            let load_icon = if is_kitty { "󰑐" } else { "⟳" };
            right_spans.push(Span::styled(format!("{} Loading…", load_icon), Style::default().fg(theme.accent).bold()));
        }
        LoadingState::Error(msg) => {
            let err_icon = if is_kitty { "󰅖" } else { "✕" };
            right_spans.push(Span::styled(format!("{} Error: {}", err_icon, msg), Style::default().fg(theme.red).bold()));
        }
        LoadingState::Idle => {
            if app.config.ui.kitty_images && chunks[right_idx].width >= 40 {
                let img_icon = if is_kitty { "󰋩  " } else { "🖼️  " };
                right_spans.push(Span::styled(img_icon, Style::default().fg(theme.muted)));
            }
            right_spans.push(Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(theme.muted)));
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
        chunks[right_idx],
    );
}

// ─── Loading overlay ──────────────────────────────────────────────────────────

fn draw_loading_overlay(f: &mut Frame, area: Rect, msg: &str, theme: &Theme) {
    let popup = centered_rect(50, 20, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(popup);
    f.render_widget(block, popup);
    
    let is_kitty = crate::kitty::is_kitty();
    let load_icon = if is_kitty { "󰑐" } else { "⟳" };

    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(load_icon, Style::default().fg(theme.accent).bold())),
            Line::from(""),
            Line::from(Span::styled(msg, Style::default().fg(theme.text))),
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

fn draw_setup(f: &mut Frame, _app: &App, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent2))
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let is_kitty = crate::kitty::is_kitty();
    let setup_icon = if is_kitty { "󰒓" } else { "⚙️" };
    let key_icon = if is_kitty { "󰌆" } else { "🔑" };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("   {} Welcome to ", setup_icon), Style::default().fg(theme.text)),
            Span::styled("watchie", Style::default().fg(theme.accent).bold()),
            Span::styled("!", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from("   To browse, search, and view movies, a free TMDB API key is required."),
        Line::from("   IMDB is heavily protected, so watchie uses TMDB to fetch movie metadata."),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("   {} How to get a key (100% Free & Instant):", key_icon), Style::default().fg(theme.text).bold())
        ]),
        Line::from("   1. Create a free account at https://www.themoviedb.org"),
        Line::from("   2. Go to your Account Settings -> API"),
        Line::from("   3. Create an API Key (choose 'Developer' option)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("   How to configure watchie:", Style::default().fg(theme.text).bold())
        ]),
        Line::from(vec![
            Span::styled("   - Option A: ", Style::default().fg(theme.muted)),
            Span::styled("Run: ", Style::default().fg(theme.text)),
            Span::styled("watchie config set-tmdb-key <YOUR_KEY>", Style::default().fg(theme.accent)),
        ]),
        Line::from(vec![
            Span::styled("   - Option B: ", Style::default().fg(theme.muted)),
            Span::styled("Set env var: ", Style::default().fg(theme.text)),
            Span::styled("export TMDB_API_KEY=<YOUR_KEY>", Style::default().fg(theme.accent)),
        ]),
        Line::from(vec![
            Span::styled("   - Option C: ", Style::default().fg(theme.muted)),
            Span::styled("Directly edit: ", Style::default().fg(theme.text)),
            Span::styled("~/.config/watchie/config.toml", Style::default().fg(theme.accent2)),
        ]),
        Line::from(""),
        Line::from("   Press 'q' or 'Ctrl+C' to exit watchie."),
    ];

    f.render_widget(Paragraph::new(text), inner);
}

// ─── TV Season List screen ───────────────────────────────────────────────────

fn draw_season_list(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let Some(movie) = &app.current_movie else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let is_kitty = crate::kitty::is_kitty();
    let date_icon = if is_kitty { "󰃭" } else { "📅" };

    // Left: Seasons List
    let block = Block::default()
        .title(Span::styled(format!(" {} Seasons ", date_icon), Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let items: Vec<ListItem> = app
        .season_list
        .iter()
        .enumerate()
        .map(|(i, season)| {
            let is_selected = i == app.selected_season;
            let prefix = if is_selected { if is_kitty { "󰁔 " } else { "▶ " } } else { "  " };
            let style = if is_selected {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD).bg(theme.surface2)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(format!("{}{}", prefix, season.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_season));
    f.render_stateful_widget(List::new(items).block(block), chunks[0], &mut state);

    // Right: Selected Season Details
    let info_icon = if is_kitty { "󰋖" } else { "ℹ️" };
    let details_block = Block::default()
        .title(Span::styled(format!(" {} Season Details ", info_icon), Style::default().fg(theme.accent2).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));
    
    let inner_area = details_block.inner(chunks[1]);
    f.render_widget(details_block, chunks[1]);

    if let Some(season) = app.season_list.get(app.selected_season) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Show:         ", Style::default().fg(theme.muted)),
                Span::styled(&movie.title, Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Title:        ", Style::default().fg(theme.muted)),
                Span::styled(&season.name, Style::default().fg(theme.text).bold()),
            ]),
            Line::from(vec![
                Span::styled("Season Num:   ", Style::default().fg(theme.muted)),
                Span::styled(season.season_number.to_string(), Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Episodes:     ", Style::default().fg(theme.muted)),
                Span::styled(season.episode_count.to_string(), Style::default().fg(theme.accent2).bold()),
            ]),
        ];

        if let Some(ref air_date) = season.air_date {
            lines.push(Line::from(vec![
                Span::styled("Air Date:     ", Style::default().fg(theme.muted)),
                Span::styled(air_date, Style::default().fg(theme.text)),
            ]));
        }

        lines.push(Line::from(""));
        let ent_icon = if is_kitty { "󰘳" } else { "Enter" };
        let esc_icon = if is_kitty { "󰈆" } else { "Esc" };
        lines.push(Line::from(Span::styled(format!("Press [{}] to browse episodes", ent_icon), Style::default().fg(theme.green).bold())));
        lines.push(Line::from(Span::styled(format!("Press [{}] to go back to series detail", esc_icon), Style::default().fg(theme.muted))));

        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            inner_area,
        );
    }
}

// ─── TV Episode List screen ──────────────────────────────────────────────────

fn draw_episode_list(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
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

    let is_kitty = crate::kitty::is_kitty();
    let tv_icon = if is_kitty { "󰏤" } else { "📺" };

    // Left: Episodes List
    let title_str = format!(" {} {} - Episodes ", tv_icon, season.name);
    let block = Block::default()
        .title(Span::styled(title_str, Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));

    let items: Vec<ListItem> = app
        .episode_list
        .iter()
        .enumerate()
        .map(|(i, ep)| {
            let is_selected = i == app.selected_episode;
            let prefix = if is_selected { if is_kitty { "󰁔 " } else { "▶ " } } else { "  " };
            let style = if is_selected {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD).bg(theme.surface2)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(format!("{}E{:02}: {}", prefix, ep.episode_number, ep.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_episode));
    f.render_stateful_widget(List::new(items).block(block), chunks[0], &mut state);

    // Right: Selected Episode details
    let info_icon = if is_kitty { "󰋖" } else { "ℹ️" };
    let details_block = Block::default()
        .title(Span::styled(format!(" {} Episode Details ", info_icon), Style::default().fg(theme.accent2).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface));
    
    let inner_area = details_block.inner(chunks[1]);
    f.render_widget(details_block, chunks[1]);

    if let Some(episode) = app.episode_list.get(app.selected_episode) {
        let rating_icon = if is_kitty { "󰓎" } else { "★" };
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Show:         ", Style::default().fg(theme.muted)),
                Span::styled(&movie.title, Style::default().fg(theme.text)),
            ]),
            Line::from(vec![
                Span::styled("Season / Ep:  ", Style::default().fg(theme.muted)),
                Span::styled(format!("Season {}, Episode {}", episode.season_number, episode.episode_number), Style::default().fg(theme.text).bold()),
            ]),
            Line::from(vec![
                Span::styled("Title:        ", Style::default().fg(theme.muted)),
                Span::styled(&episode.name, Style::default().fg(theme.accent2).bold()),
            ]),
        ];

        if let Some(ref air) = episode.air_date {
            lines.push(Line::from(vec![
                Span::styled("Air Date:     ", Style::default().fg(theme.muted)),
                Span::styled(air, Style::default().fg(theme.text)),
            ]));
        }

        if let Some(rating) = episode.vote_average {
            lines.push(Line::from(vec![
                Span::styled("Rating:       ", Style::default().fg(theme.muted)),
                Span::styled(format!("{} {:.1}/10", rating_icon, rating), Style::default().fg(theme.accent).bold()),
            ]));
        }

        if let Some(runtime) = episode.runtime {
            lines.push(Line::from(vec![
                Span::styled("Runtime:      ", Style::default().fg(theme.muted)),
                Span::styled(format!("{} min", runtime), Style::default().fg(theme.text)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Overview:", Style::default().fg(theme.muted).bold())));

        if let Some(ref overview) = episode.overview {
            let max_w = inner_area.width.saturating_sub(4) as usize;
            for line in wrap_text(overview, max_w) {
                lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme.text))));
            }
        } else {
            lines.push(Line::from(Span::styled("  No overview available.", Style::default().fg(theme.muted))));
        }

        lines.push(Line::from(""));
        let ent_icon = if is_kitty { "󰘳 / 󰐊" } else { "Enter / p" };
        let d_icon = if is_kitty { "󰇚" } else { "d" };
        let esc_icon = if is_kitty { "󰈆" } else { "Esc" };
        lines.push(Line::from(Span::styled(format!("Press [{}] to Play Episode", ent_icon), Style::default().fg(theme.green).bold())));
        lines.push(Line::from(Span::styled(format!("Press [{}] to Download Episode", d_icon), Style::default().fg(theme.accent2).bold())));
        lines.push(Line::from(Span::styled(format!("Press [{}] to go back to season list", esc_icon), Style::default().fg(theme.muted))));

        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            inner_area,
        );
    }
}
