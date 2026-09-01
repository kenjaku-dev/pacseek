use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::{border_type_from_str, parse_style};

use super::app::{App, Focus, Popup};

// tui-design skill: calm, predictable, fast — single border depth, semantic tokens, NO_COLOR aware
pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Responsive floor: configurable via [tui] floor_width/floor_height per tui-design responsive + layout Min
    let fw = app.config.tui.floor_width.max(40);
    let fh = app.config.tui.floor_height.max(10);
    if area.width < fw || area.height < fh {
        let msg = Paragraph::new(format!("Terminal too small — need ≥{}×{}", fw, fh))
            .style(if app.no_color {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                parse_style(&app.config.theme.border_error)
            })
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().title(" pacseek "));
        f.render_widget(msg, area);
        return;
    }

    // Layout: vertical configurable via config (defaults 3 / Min(8) /1 /1) — clutter audit keeps chrome <20%
    let ls = app.config.tui.layout_search.max(2);
    let lm = app.config.tui.layout_results_min.max(4);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ls), // search bar
            Constraint::Min(lm),    // results
            Constraint::Length(1),  // status
            Constraint::Length(1),  // help
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
        parse_style(&app.config.theme.border_focused)
    } else {
        parse_style(&app.config.theme.border_unfocused)
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
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.text)
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .title(title)
                .border_type(border_type_from_str(&app.config.tui.border)),
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
            .style(if app.no_color {
                Style::default()
            } else {
                parse_style(&app.config.theme.text_dim)
            })
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(border_type_from_str(&app.config.tui.border))
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
                parse_style(&app.config.theme.repo_aur)
            } else {
                match pkg.repo.as_str() {
                    "core" => parse_style(&app.config.theme.repo_core),
                    "extra" => parse_style(&app.config.theme.repo_extra),
                    "multilib" => parse_style(&app.config.theme.repo_multilib),
                    _ => parse_style(&app.config.theme.repo_other),
                }
            };
            let installed = if pkg.installed {
                Span::styled(
                    " [installed]",
                    if use_color {
                        parse_style(&app.config.theme.installed)
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
                        parse_style(&app.config.theme.votes)
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
                        parse_style(&app.config.theme.version)
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
                    parse_style(&app.config.theme.text_dim)
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
                .border_type(border_type_from_str(&app.config.tui.border))
                .title(title),
        )
        .highlight_style(if use_color {
            let bg = crate::config::parse_color(&app.config.theme.highlight_bg);
            let fg_style = parse_style(&app.config.theme.highlight_fg);
            let mut st = Style::default().bg(bg);
            if let Some(c) = fg_style.fg {
                st = st.fg(c);
            }
            st.add_modifier(fg_style.add_modifier)
        } else {
            Style::default().add_modifier(Modifier::REVERSED)
        })
        .highlight_symbol(
            app.config
                .tui
                .highlight_symbol
                .clone()
                .unwrap_or_else(|| "▸ ".into()),
        );

    // ListState is stateful per ecosystem-rust.md
    let mut state = app.list_state;
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let style = if app.no_color {
        Style::default()
    } else if app.is_loading {
        parse_style(&app.config.theme.status_loading)
    } else {
        parse_style(&app.config.theme.status_idle)
    };
    let line = Line::from(vec![Span::styled(app.status.clone(), style)]);
    let p = Paragraph::new(line);
    f.render_widget(p, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help = if app.focus == Focus::Search {
        " Enter:search  Esc:list  Ctrl+C:quit  (type to filter)"
    } else {
        " ↑↓/j k:navigate  Enter:install  i:info  /:search  q:quit "
    };
    let p = Paragraph::new(help)
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.help)
        })
        .block(Block::default());
    f.render_widget(p, area);
}

fn draw_info_popup(f: &mut Frame, pkg: &crate::model::Package, area: Rect, app: &App) {
    let sz = app.config.tui.popup_info.unwrap_or([70, 60]);
    let popup_area = centered_rect(sz[0], sz[1], area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} / {} ", pkg.repo, pkg.name))
        .borders(Borders::ALL)
        .border_type(border_type_from_str(&app.config.tui.border))
        .border_style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.popup_title)
        })
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.popup_bg)
        });

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let text = vec![
        Line::from(vec![
            Span::styled(
                "Version: ",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.status_loading).add_modifier(Modifier::BOLD)
                },
            ),
            Span::raw(pkg.version.clone()),
            Span::raw(format!("  Arch: {}", pkg.arch.as_deref().unwrap_or("-"))),
        ]),
        Line::from(vec![
            Span::styled(
                "Repo: ",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.status_loading).add_modifier(Modifier::BOLD)
                },
            ),
            Span::raw(pkg.repo.clone()),
            Span::raw(if pkg.installed { "  [installed]" } else { "" }),
        ]),
        Line::from(vec![
            Span::styled(
                "URL: ",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.status_loading).add_modifier(Modifier::BOLD)
                },
            ),
            Span::raw(pkg.url.as_deref().unwrap_or("-")),
        ]),
        Line::from(vec![
            Span::styled(
                "Desc: ",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.status_loading).add_modifier(Modifier::BOLD)
                },
            ),
            Span::raw(pkg.description.as_deref().unwrap_or("-")),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "Votes: ",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.votes).add_modifier(Modifier::BOLD)
                },
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
            if app.no_color {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                parse_style(&app.config.theme.out_of_date)
            },
        )]),
        Line::raw(""),
        Line::from(Span::styled(
            "Press Esc/q/Enter to close",
            if app.no_color {
                Style::default()
            } else {
                parse_style(&app.config.theme.text_dim)
            },
        )),
    ];

    let p = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.text)
        });
    f.render_widget(p, inner);
}

fn draw_confirm_popup(f: &mut Frame, pkg: &crate::model::Package, area: Rect, app: &App) {
    let sz = app.config.tui.popup_confirm.unwrap_or([60, 30]);
    let popup_area = centered_rect(sz[0], sz[1], area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Confirm install ")
        .borders(Borders::ALL)
        .border_type(border_type_from_str(&app.config.tui.border))
        .border_style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.popup_title)
        })
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.popup_bg)
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let text = vec![
        Line::from(vec![
            Span::raw("Install "),
            Span::styled(
                format!("{}/{} {}", pkg.repo, pkg.name, pkg.version),
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.installed)
                },
            ),
            Span::raw(" ?"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "Y",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.installed)
                },
            ),
            Span::raw("/Enter = yes  "),
            Span::styled(
                "N/Esc",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    parse_style(&app.config.theme.out_of_date)
                },
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
            if app.no_color {
                Style::default()
            } else {
                parse_style(&app.config.theme.text_dim)
            },
        )),
    ];
    let p = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn draw_message_popup(f: &mut Frame, msg: &str, area: Rect, app: &App) {
    let sz = app.config.tui.popup_message.unwrap_or([60, 20]);
    let popup_area = centered_rect(sz[0], sz[1], area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Message ")
        .borders(Borders::ALL)
        .border_type(border_type_from_str(&app.config.tui.border))
        .border_style(if app.no_color {
            Style::default()
        } else if msg.starts_with('✓') {
            parse_style(&app.config.theme.installed)
        } else {
            parse_style(&app.config.theme.out_of_date)
        })
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.popup_bg)
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let p = Paragraph::new(msg.to_string())
        .style(if app.no_color {
            Style::default()
        } else {
            parse_style(&app.config.theme.text)
        })
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Phase D5: cache-free, no Layout alloc — manual centered math (vs 2 Layout splits before)
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let popup_y = r.y + (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(popup_x, popup_y, popup_width, popup_height)
}
