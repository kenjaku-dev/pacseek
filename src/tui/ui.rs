use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::app::{App, Focus, Popup};

// tui-design skill: calm, predictable, fast — single border depth, semantic tokens, NO_COLOR aware
pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Responsive floor: 60x24 minimum per tui-design visual-patterns
    if area.width < 60 || area.height < 10 {
        let msg = Paragraph::new("Terminal too small — need ≥60×10")
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().title(" pacseek "));
        f.render_widget(msg, area);
        return;
    }

    // Layout: vertical [search 3][results min(0)][status 1][help 1]
    // Clutter audit: only results and search have borders, status/help are borderless to keep chrome <20% cells
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Min(8),    // results
            Constraint::Length(1), // status
            Constraint::Length(1), // help
        ])
        .split(area);

    draw_search(f, app, chunks[0]);
    draw_results(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);
    draw_help(f, app, chunks[3]);

    // Popups on top with Clear hole-punch
    match &app.popup {
        Popup::Info(pkg) => draw_info_popup(f, pkg, area, app),
        Popup::Confirm(pkg) => draw_confirm_popup(f, pkg, area, app),
        Popup::Message(msg) => draw_message_popup(f, msg, area, app),
        Popup::None => {}
    }
}

fn draw_search(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.focus == Focus::Search {
        " Search (Enter to search, Esc to list) "
    } else {
        " Search (press / to focus) "
    };
    let style = if app.no_color {
        Style::default()
    } else if app.focus == Focus::Search {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_str = app.input.value();
    let width = area.width.saturating_sub(4) as usize; // borders + padding
    // width-aware tail truncation, char-boundary safe per rust-common-pitfalls
    let input_width = input_str.width();
    let display = if input_width > width && width > 3 {
        let target = width.saturating_sub(3);
        let mut w = 0usize;
        let mut start_idx = input_str.len();
        // Walk from end, collect width until target — char-boundary safe
        for (idx, ch) in input_str.char_indices().rev() {
            let cw = ch.width().unwrap_or(0);
            if w + cw > target {
                break;
            }
            w += cw;
            start_idx = idx;
        }
        // start_idx is already at char boundary from char_indices
        format!("...{}", &input_str[start_idx..])
    } else {
        input_str.to_string()
    };

    // Cursor position for tui-input (visual width, not byte index)
    let cursor_pos = app.input.visual_cursor();

    let paragraph = Paragraph::new(display)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .title(title)
                .border_type(ratatui::widgets::BorderType::Rounded),
        );

    f.render_widget(paragraph, area);

    // Show cursor when focused
    if app.focus == Focus::Search {
        // tui-input gives visual cursor, but we place native cursor for terminal
        let x = area.x + 1 + (cursor_pos as u16).min(area.width.saturating_sub(3));
        let y = area.y + 1;
        f.set_cursor_position((x, y));
    }
}

fn draw_results(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(
        " Results {} {} ",
        if app.is_loading { "⏳" } else { "" },
        if app.packages.is_empty() {
            "".into()
        } else {
            format!(
                "({}/{})",
                app.list_state.selected().map(|i| i + 1).unwrap_or(0),
                app.packages.len()
            )
        }
    );

    if app.packages.is_empty() {
        let text = if app.is_loading {
            "Searching..."
        } else if app.input.value().is_empty() {
            "Type a package name above and press Enter — e.g. firefox"
        } else {
            "No packages found. Try another query or check filters."
        };
        let p = Paragraph::new(text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .title(title),
            );
        f.render_widget(p, area);
        return;
    }

    let use_color = !app.no_color;
    let items: Vec<ListItem> = app
        .packages
        .iter()
        .map(|pkg| {
            let repo_style = if !use_color {
                Style::default().add_modifier(Modifier::BOLD)
            } else if pkg.repo == "aur" {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                match pkg.repo.as_str() {
                    "core" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    "extra" => Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    "multilib" => Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                }
            };
            let installed = if pkg.installed {
                Span::styled(
                    " [installed]",
                    if use_color {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::BOLD)
                    },
                )
            } else {
                Span::raw("")
            };
            let aur_extra = if pkg.repo == "aur" {
                let votes = pkg.votes.unwrap_or(0);
                let pop = pkg.popularity.unwrap_or(0.0);
                let ood = if pkg.out_of_date.is_some() {
                    " [out-of-date]"
                } else {
                    ""
                };
                Span::styled(
                    format!(" (+{} {:.2}){}", votes, pop, ood),
                    if use_color {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                )
            } else {
                Span::raw("")
            };

            let first = Line::from(vec![
                Span::styled(format!("{}/{}", pkg.repo, pkg.name), repo_style),
                Span::styled(
                    format!(" {}", pkg.version),
                    if use_color {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default()
                    },
                ),
                aur_extra,
                installed,
            ]);

            let second = Line::from(vec![Span::styled(
                format!("  {}", pkg.description.as_deref().unwrap_or("-")),
                if use_color {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                },
            )]);

            ListItem::new(vec![first, second])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(title),
        )
        .highlight_style(if use_color {
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::REVERSED)
        })
        .highlight_symbol("▸ ");

    // ListState is stateful per ecosystem-rust.md
    let mut state = app.list_state;
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let style = if app.no_color {
        Style::default()
    } else if app.is_loading {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let line = Line::from(vec![Span::styled(app.status.clone(), style)]);
    let p = Paragraph::new(line);
    f.render_widget(p, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    // Discoverability per tui-design: contextual hints, not hidden — fixed C1 lie: Search no longer shows q:quit
    let help = if app.focus == Focus::Search {
        " Enter:search  Esc:list  Ctrl+C:quit  (type to filter)"
    } else {
        " ↑↓/j k:navigate  Enter:install  i:info  /:search  q:quit "
    };
    let p = Paragraph::new(help)
        .style(if app.no_color {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .block(Block::default());
    f.render_widget(p, area);
}

fn draw_info_popup(f: &mut Frame, pkg: &crate::model::Package, area: Rect, app: &App) {
    let popup_area = centered_rect(70, 60, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} / {} ", pkg.repo, pkg.name))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(if app.no_color {
            Style::default()
        } else {
            Style::default().fg(Color::Yellow)
        })
        .style(if app.no_color {
            Style::default()
        } else {
            Style::default().bg(Color::Black)
        });

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let text = vec![
        Line::from(vec![
            Span::styled(
                "Version: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(pkg.version.clone()),
            Span::raw(format!("  Arch: {}", pkg.arch.as_deref().unwrap_or("-"))),
        ]),
        Line::from(vec![
            Span::styled(
                "Repo: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(pkg.repo.clone()),
            Span::raw(if pkg.installed { "  [installed]" } else { "" }),
        ]),
        Line::from(vec![
            Span::styled(
                "URL: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(pkg.url.as_deref().unwrap_or("-")),
        ]),
        Line::from(vec![
            Span::styled(
                "Desc: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(pkg.description.as_deref().unwrap_or("-")),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "Votes: ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}", pkg.votes.unwrap_or(0))),
            Span::raw(format!(
                "  Popularity: {:.2}",
                pkg.popularity.unwrap_or(0.0)
            )),
            Span::raw(
                pkg.maintainer
                    .as_deref()
                    .map(|m| format!("  Maintainer: {}", m))
                    .unwrap_or_default(),
            ),
        ]),
        Line::from(vec![Span::styled(
            if pkg.out_of_date.is_some() {
                "[out-of-date]"
            } else {
                ""
            },
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::raw(""),
        Line::from(Span::styled(
            "Press Esc/q/Enter to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    f.render_widget(p, inner);
}

fn draw_confirm_popup(f: &mut Frame, pkg: &crate::model::Package, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 30, area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Confirm install ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(if app.no_color {
            Style::default()
        } else {
            Style::default().fg(Color::Yellow)
        })
        .style(if app.no_color {
            Style::default()
        } else {
            Style::default().bg(Color::Black)
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let text = vec![
        Line::from(vec![
            Span::raw("Install "),
            Span::styled(
                format!("{}/{} {}", pkg.repo, pkg.name, pkg.version),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ?"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "Y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("/Enter = yes  "),
            Span::styled(
                "N/Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" = cancel"),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            if pkg.repo == "aur" {
                "AUR PKGBUILD will be shown before build."
            } else {
                "Will run: sudo pacman -S <pkg> [needs password]"
            },
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let p = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn draw_message_popup(f: &mut Frame, msg: &str, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Message ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(if app.no_color {
            Style::default()
        } else {
            Style::default().fg(if msg.starts_with('✓') {
                Color::Green
            } else {
                Color::Red
            })
        })
        .style(if app.no_color {
            Style::default()
        } else {
            Style::default().bg(Color::Black)
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let p = Paragraph::new(msg.to_string())
        .style(Style::default().fg(Color::White))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    ratatui::layout::Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
