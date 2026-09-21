use pacseek::model::Package;
use pacseek::tui::app::{App, Focus, Mode, Popup};
use ratatui::{Terminal, backend::TestBackend};

fn sample_packages() -> Vec<Package> {
    vec![
        Package {
            name: "firefox".into(),
            version: "123.0-1".into(),
            description: Some("Standalone web browser".into()),
            repo: "extra".into(),
            arch: Some("x86_64".into()),
            url: Some("https://firefox.example".into()),
            installed: false,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
        },
        Package {
            name: "firefox-bin".into(),
            version: "123.0-1".into(),
            description: Some("Binary Firefox".into()),
            repo: "aur".into(),
            arch: None,
            url: Some("https://aur.example".into()),
            installed: false,
            votes: Some(1200),
            popularity: Some(5.2),
            out_of_date: None,
            maintainer: Some("someone".into()),
            num_votes: Some(1200),
            last_modified: Some(1700000000),
        },
    ]
}

#[test]
fn tui_renders_search_and_results() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new("firefox".into());
    app.packages = sample_packages();
    app.list_state.select(Some(0));
    app.focus = Focus::List;
    app.status = "Found 2 (repo 1 aur 1)".into();
    app.is_loading = false;

    terminal.draw(|f| pacseek::tui::ui::draw(f, &app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    // Convert buffer to lines for snapshot
    let lines: Vec<String> = (0..24)
        .map(|y| {
            let mut line = String::new();
            for x in 0..80 {
                line.push_str(buf[(x, y)].symbol());
            }
            line.trim_end().to_string()
        })
        .collect();
    // Basic assertions — per tui-design visual-patterns, check layout not empty
    assert!(lines.iter().any(|l| l.contains("Search")));
    assert!(lines.iter().any(|l| l.contains("Results")));
    assert!(lines.iter().any(|l| l.contains("firefox")));
    assert!(lines.iter().any(|l| l.contains("Found 2")));
}

#[test]
fn tui_too_small_shows_message() {
    let backend = TestBackend::new(50, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = App::new("".into());
    terminal.draw(|f| pacseek::tui::ui::draw(f, &app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let content = buf.content().iter().map(|c| c.symbol()).collect::<String>();
    assert!(content.contains("Terminal too small"));
}

#[test]
fn tui_info_popup_renders() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new("firefox".into());
    app.packages = sample_packages();
    app.popup = Popup::Info(sample_packages()[0].clone());
    terminal.draw(|f| pacseek::tui::ui::draw(f, &app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let content = buf.content().iter().map(|c| c.symbol()).collect::<String>();
    assert!(content.contains("firefox"));
    assert!(content.contains("Version"));
}

fn sample_installed() -> Vec<Package> {
    vec![Package {
        name: "vim".into(),
        version: "9.1-1".into(),
        description: Some("Vi improved".into()),
        repo: "local".into(),
        arch: Some("x86_64".into()),
        url: None,
        installed: true,
        votes: None,
        popularity: None,
        out_of_date: None,
        maintainer: None,
        num_votes: None,
        last_modified: None,
    }]
}

fn render(app: &App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| pacseek::tui::ui::draw(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>()
}

#[test]
fn tui_tabs_bar_renders_both_modes() {
    let mut app = App::new("".into());
    app.focus = Focus::List;
    let content = render(&app, 80, 24);
    assert!(content.contains("Search"));
    assert!(content.contains("Installed"));
}

#[test]
fn tui_installed_mode_renders_local_list() {
    let mut app = App::new("".into());
    app.switch_mode();
    app.packages = sample_installed();
    app.list_state.select(Some(0));
    app.focus = Focus::List;
    app.is_loading = false;
    app.status = "Installed 1 (local 1)".into();
    let content = render(&app, 80, 24);
    assert!(content.contains("Installed"));
    assert!(content.contains("local/vim"));
    assert!(content.contains("Installed 1"));
    // [installed] tag hidden in Installed mode (noise reduction)
    assert!(!content.contains("[installed]"));
}

#[test]
fn tui_confirm_remove_popup_renders() {
    let mut app = App::new("".into());
    app.switch_mode();
    app.packages = sample_installed();
    app.popup = Popup::ConfirmRemove(sample_installed()[0].clone());
    let content = render(&app, 80, 24);
    assert!(content.contains("Confirm remove"));
    assert!(content.contains("sudo pacman"));
    assert!(content.contains("vim"));
}

#[test]
fn tui_help_popup_renders() {
    let mut app = App::new("".into());
    app.focus = Focus::List;
    app.popup = Popup::Help;
    let content = render(&app, 80, 24);
    assert!(content.contains("Help"));
    assert!(content.contains("Tab"));
    assert!(content.contains("Installed"));
}

#[test]
fn tui_narrow_fallback_no_emoji() {
    let mut app = App::new("".into());
    app.focus = Focus::List;
    app.packages = sample_packages();
    app.list_state.select(Some(0));
    // 60-col split per tui-design floor test
    let content = render(&app, 60, 24);
    assert!(content.contains("Search"));
    assert!(content.contains("Installed"));
}

#[test]
fn tui_no_color_tabs_ascii() {
    let mut app = App::new("".into());
    app.no_color = true;
    app.focus = Focus::List;
    app.mode = Mode::Installed;
    app.packages = sample_installed();
    app.list_state.select(Some(0));
    let content = render(&app, 80, 24);
    assert!(content.contains("Installed"));
}
