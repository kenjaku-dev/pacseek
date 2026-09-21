use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::app::{App, Focus, Mode, Popup};

// tui-design skill: calm, predictable, fast — single border depth, semantic tokens, NO_COLOR aware
// v0.3.0: Tabs bar (Search/Installed) + ThemeStyles cache (no per-frame parse_style).
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
                app.styles.border_error
            })
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().title(" pacseek "));
        f.render_widget(msg, area);
        return;
    }

    let show_tabs = app.config.tui.show_tabs;
    // Layout: tabs(1) + search(3) + results(Min 8) + status(1) + help(1)
    let ls = app.config.tui.layout_search.max(2);
    let lm = app.config.tui.layout_results_min.max(4);
    if show_tabs {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // tabs
                Constraint::Length(ls), // search bar
                Constraint::Min(lm),    // results
                Constraint::Length(1),  // status
                Constraint::Length(1),  // help
            ])
            .split(area);
        draw_tabs(f, app, chunks[0]);
        draw_search(f, app, chunks[1]);
        draw_results(f, app, chunks[2]);
        draw_status(f, app, chunks[3]);
        draw_help(f, app, chunks[4]);
    } else {
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
    }

    // Popups on top with Clear hole-punch
    match &app.popup {
        Popup::Info(pkg) => draw_info_popup(f, pkg, area, app),
        Popup::Confirm(pkg) => draw_confirm_popup(f, pkg, area, app),
        Popup::ConfirmRemove(pkg) => draw_confirm_remove_popup(f, pkg, area, app),
        Popup::ConfirmRefresh => draw_confirm_refresh_popup(f, area, app),
        Popup::Help => draw_help_popup(f, area, app),
        Popup::Message(msg) => draw_message_popup(f, msg, area, app),
        Popup::None => {}
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let use_color = !app.no_color;
    let titles: Vec<Line> = if area.width < 50 {
        // Narrow ASCII fallback — no emoji, short labels
        vec![Line::from("[1]Search"), Line::from("[2]Installed")]
    } else {
        vec![
            Line::from("🔍 Search (install)"),
            Line::from("🗑 Installed (remove)"),
        ]
    };
    let tabs = Tabs::new(titles)
        .select(app.mode.tab_index())
        .style(if use_color {
            app.styles.tab_normal
        } else {
            Style::default()
        })
        .highlight_style(if use_color {
            app.styles.tab_selected
        } else {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        })
        .divider(if area.width < 50 { "|" } else { " │ " });
    // Hint on the right is rendered via status/help; keep tabs single-line, no border (clutter audit).
    f.render_widget(tabs, area);
    let _ = Mode::Search; // keep import used in narrow builds
}

fn draw_search(f: &mut Frame, app: &App, area: Rect) {
    let title = match (app.mode, app.focus) {
        (Mode::Search, Focus::Search) => " Search (Enter to search, Esc to list, Tab remover) ",
        (Mode::Search, Focus::List) => " Search (press / to focus, Tab remover) ",
        (Mode::Installed, Focus::Search) => " Filter installed (Enter to filter, Tab back) ",
        (Mode::Installed, Focus::List) => " Filter installed (press / to focus, Tab back) ",
    };
    let style = if app.no_color {
        Style::default()
    } else if app.focus == Focus::Search {
        app.styles.border_focused
    } else {
        app.styles.border_unfocused
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
            app.styles.text
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .title(title)
                .border_type(app.border_type),
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
    let mode_tag = match app.mode {
        Mode::Search => "",
        Mode::Installed => "installed ",
    };
    let title = format!(
        " Results {}{} {} ",
        mode_tag,
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
            match app.mode {
                Mode::Search => "Searching...",
                Mode::Installed => "Listing installed...",
            }
        } else if app.input.value().trim().is_empty() {
            match app.mode {
                Mode::Search => "Type a package name above and press Enter — e.g. firefox",
                Mode::Installed => {
                    "Installed mode — type to filter, empty shows all (Tab: back to search)"
                }
            }
        } else {
            match app.mode {
                Mode::Search => "No packages found. Try another query or check filters.",
                Mode::Installed => "No installed packages match. Clear filter or press Tab.",
            }
        };
        let p = Paragraph::new(text)
            .style(if app.no_color {
                Style::default()
            } else {
                app.styles.text_dim
            })
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(app.border_type)
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
                app.styles.repo_aur
            } else {
                match pkg.repo.as_str() {
                    "core" => app.styles.repo_core,
                    "extra" => app.styles.repo_extra,
                    "multilib" => app.styles.repo_multilib,
                    "local" => app.styles.repo_local,
                    _ => app.styles.repo_other,
                }
            };
            // In Installed mode the [installed] tag is noise (all are installed) — hide it.
            let show_installed_tag = pkg.installed && app.mode == Mode::Search;
            let installed = if show_installed_tag {
                Span::styled(
                    " [installed]",
                    if use_color {
                        app.styles.installed
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
                        app.styles.votes
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
                        app.styles.version
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
                    app.styles.text_dim
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
                .border_type(app.border_type)
                .title(title),
        )
        .highlight_style(if use_color {
            app.styles.highlight
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
        app.styles.status_loading
    } else {
        app.styles.status_idle
    };
    let line = Line::from(vec![Span::styled(app.status.clone(), style)]);
    let p = Paragraph::new(line);
    f.render_widget(p, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help = match (app.mode, app.focus) {
        (Mode::Search, Focus::Search) => {
            " Enter:search  Esc:list  Tab:remover  ?:help  Ctrl+C:quit "
        }
        (Mode::Search, Focus::List) => {
            " ↑↓/j k:move  Enter:install  i:info  r:refresh  /:search  Tab:remover  ?:help  q:quit "
        }
        (Mode::Installed, Focus::Search) => {
            " Enter:filter  Esc:list  Tab:search  ?:help  Ctrl+C:quit "
        }
        (Mode::Installed, Focus::List) => {
            " ↑↓/j k:move  Enter/d:remove  i:info  r:refresh  /:filter  Tab:search  ?:help  q:quit "
        }
    };
    let p = Paragraph::new(help)
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.help
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
        .border_type(app.border_type)
        .border_style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_title
        })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_bg
        });

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let bold = |s: Style| {
        if app.no_color {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            s.add_modifier(Modifier::BOLD)
        }
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Version: ", bold(app.styles.status_loading)),
            Span::raw(pkg.version.clone()),
            Span::raw(format!("  Arch: {}", pkg.arch.as_deref().unwrap_or("-"))),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", bold(app.styles.status_loading)),
            Span::raw(pkg.repo.clone()),
            Span::raw(if pkg.installed { "  [installed]" } else { "" }),
        ]),
        Line::from(vec![
            Span::styled("URL: ", bold(app.styles.status_loading)),
            Span::raw(pkg.url.as_deref().unwrap_or("-")),
        ]),
        Line::from(vec![
            Span::styled("Desc: ", bold(app.styles.status_loading)),
            Span::raw(pkg.description.as_deref().unwrap_or("-")),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Votes: ", bold(app.styles.votes)),
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
                app.styles.out_of_date
            },
        )]),
        Line::raw(""),
        Line::from(Span::styled(
            "Press Esc/q/Enter to close",
            if app.no_color {
                Style::default()
            } else {
                app.styles.text_dim
            },
        )),
    ];

    let p = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.text
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
        .border_type(app.border_type)
        .border_style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_title
        })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_bg
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
                    app.styles.installed
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
                    app.styles.installed
                },
            ),
            Span::raw("/Enter = yes  "),
            Span::styled(
                "N/Esc",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    app.styles.out_of_date
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
                app.styles.text_dim
            },
        )),
    ];
    let p = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn draw_confirm_remove_popup(f: &mut Frame, pkg: &crate::model::Package, area: Rect, app: &App) {
    let sz = app.config.tui.popup_confirm.unwrap_or([60, 30]);
    let popup_area = centered_rect(sz[0], sz[1], area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Confirm remove ")
        .borders(Borders::ALL)
        .border_type(app.border_type)
        .border_style(if app.no_color {
            Style::default()
        } else {
            app.styles.out_of_date
        })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_bg
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let flag = app.config.remove_flag_arg();
    let text = vec![
        Line::from(vec![
            Span::raw("Remove "),
            Span::styled(
                format!("{}/{} {}", pkg.repo, pkg.name, pkg.version),
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    app.styles.out_of_date
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
                    app.styles.installed
                },
            ),
            Span::raw("/Enter = yes  "),
            Span::styled(
                "N/Esc",
                if app.no_color {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    app.styles.out_of_date
                },
            ),
            Span::raw(" = cancel"),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            format!(
                "Will run: sudo pacman {} {} [needs password]",
                flag, pkg.name
            ),
            if app.no_color {
                Style::default()
            } else {
                app.styles.text_dim
            },
        )),
        Line::from(Span::styled(
            "Check Required By in info (i) before removing libs.",
            if app.no_color {
                Style::default()
            } else {
                app.styles.text_dim
            },
        )),
    ];
    let p = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn draw_help_popup(f: &mut Frame, area: Rect, app: &App) {
    let sz: [u16; 2] = [70, 60];
    let popup_area = centered_rect(sz[0], sz[1], area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Help (?) ")
        .borders(Borders::ALL)
        .border_type(app.border_type)
        .border_style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_title
        })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_bg
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let lines = vec![
        Line::from("Tab / Shift+Tab : switch Search <-> Installed"),
        Line::from(""),
        Line::from("Search mode:"),
        Line::from("  type + Enter  : search repo+AUR"),
        Line::from("  ↑↓ / j k      : navigate"),
        Line::from("  Enter         : install selected"),
        Line::from("  i             : package info"),
        Line::from("  r / F5        : refresh sync DBs (confirm)"),
        Line::from(""),
        Line::from("Installed mode:"),
        Line::from("  type + Enter  : filter installed"),
        Line::from("  ↑↓ / j k      : navigate"),
        Line::from("  Enter / d / x : remove selected"),
        Line::from("  i             : package info"),
        Line::from("  r / F5        : refresh sync DBs (confirm)"),
        Line::from(""),
        Line::from("Global: / focus search, Esc focus toggle, ? help, q quit, r refresh"),
        Line::from(""),
        Line::from("Press Esc/q/Enter/? to close"),
    ];
    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.text
        });
    f.render_widget(p, inner);
}

fn draw_message_popup(f: &mut Frame, msg: &str, area: Rect, app: &App) {
    let sz = app.config.tui.popup_message.unwrap_or([60, 20]);
    let popup_area = centered_rect(sz[0], sz[1], area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Message ")
        .borders(Borders::ALL)
        .border_type(app.border_type)
        .border_style(if app.no_color {
            Style::default()
        } else if msg.starts_with('✓') {
            app.styles.installed
        } else {
            app.styles.out_of_date
        })
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.popup_bg
        });
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    let p = Paragraph::new(msg.to_string())
        .style(if app.no_color {
            Style::default()
        } else {
            app.styles.text
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
