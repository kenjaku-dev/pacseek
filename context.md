# pacseek — Project Context

## Overview
Fast **AUR + Official Repo** search + TUI install for Arch/Artix, Rust `0.2.0`, at `/home/achraf/Projects/pacseek` (`pacseek` binary, `MIT`).
One binary: `libalpm` local DB (no network) + AUR `https://aur.archlinux.org/rpc/v5` + `ratatui` TUI matching drawing: top `firefox` search bar → results list → `Enter` install / `i` info.

```
┌ Search (Enter to search) ─────────────────┐
│ firefox█                                  │
└───────────────────────────────────────────┘
┌ Results (1/10) ───────────────────────────┐
│▸ extra/firefox 123.0-1                     │
│    Fast browser                           │
└───────────────────────────────────────────┘
 Found 10 (repo 5 aur 5)   ↑↓/j k  Enter:install  i:info  /:search q:quit
```

## Tech Stack
- `rust 1.85+` (MSRV 1.88 for `ratatui 0.30`), `edition 2024`, `tokio full`, `reqwest 0.12` (json+rustls+blocking via OnceLock), `alpm 5` + `pacmanconf 3`, `raur 8`, `clap 4.5` derive, `serde`/`serde_json`/`toml 0.8`, `anyhow`/`thiserror`, `colored 2`, `indicatif 0.17`, `tracing 0.1`, `regex 1`, `url 2`
- TUI: `ratatui 0.30` `crossterm 0.29` `tui-input 0.15` `color-eyre 0.6` `dirs 5` `nix 0.29` `unicode-width 0.2` `signal-hook 0.3`
- Config: `~/.config/pacseek/config.toml` + `./pacseek.toml` project-local, `[search]/[tui]/[theme]/[behavior]` via `crate::config::Config` (serde TOML, defaults < file < env < CLI)
- Profile `release` `opt-level=z lto codegen-units=1 strip panic=unwind` **5.6M** `target/release/pacseek`, `dev` `opt-level 0`

## Architecture
```
src/
  main.rs       # IsTerminal, --tui/--no-tui/--json, Config::load + CLI merge (defaults < file < env < CLI), tokio::join! parallel repo+aur, spinner from config
  lib.rs        # pub mod cli/config/install/model/output/search/tui
  cli.rs        # Clap derive, query:Option<String>, Source (All/Aur/Repo), AurBy (name/name-desc...), limit 50, regex, installed_only, bottom_up, verbose, no_color, no_tui, tui, --config/--init-config/--show-config
  config.rs     # Config {search,tui,theme,behavior} serde TOML, XDG ~/.config/pacseek/config.toml + ./pacseek.toml, example_toml(), parse_style/border_type, aur_rpc/cache_dir helpers, 4 tests
  model.rs      # Package {name, version, description, repo, arch, url, installed, votes/popularity/out_of_date/maintainer/num_votes/last_modified} PartialEq
  search/
    aur.rs      # RPC v5 async + blocking, config aur_rpc + timeout (OnceLock vs per-call), NaN-safe sort (filter finite), regex client-side
    repo.rs     # alpm via OnceLock Config (get_cached_config), search_repo_with_config, fallback pacman -Ss (wrapped desc, indented lines)
  output.rs     # colored + json, votes/popularity, [installed]
  tui/
    mod.rs      # run/run_with_cli/run_with_config try_init/try_restore best-effort, suspend_and_run flush+drain EventStream, both-fail aggregation
    app.rs      # Config stored, Input (tui-input), ListState None on empty, Focus Search/List, Popup Info/Confirm/Message, limit/source/aur_by/use_regex/installed_only/no_color, mpsc channel non-blocking trigger_search, poll_ms/debounce_ms from config, signal-hook
    ui.rs       # Frame floor configurable, Layout vertical configurable, border_type + colors from theme (parse_style), popups % from config, highlight_symbol, Clear hole-punch
  install/
    repo.rs     # sudo pacman -S --needed (no --noconfirm by default)
    aur.rs      # config cache_dir + makepkg_noconfirm, sanitize PathBuf::file_name, /tmp fallback 0o700, git clone/pull --ff-only, bat/cat PKGBUILD, namcap+shellcheck gated, makepkg -si, nix::geteuid root check
  error.rs
  config.example.toml # example ship
tests/
  integration_search.rs # help/version/repo/json (4)
  tui_snapshot.rs       # TestBackend 80x24, 50x8 too-small, info popup (3)
  aur_nan.rs            # NaN sort, unicode truncate (2)
  tui::app::tests       # trigger_search non-blocking, q in Search/List, filters, ListState (6) + config::tests (4) — total 15+2 doc
```

## Skills Used
- `~/.config/opencode/skill/ratatui` (mte90, 0.30.1) — widgets, TestBackend, Layout cache, event, panic hook before init
- `~/.config/opencode/skill/tui-design` — alt-screen, non-blocking, NO_COLOR, 60x13 floor, clutter <1 border, keyboard-reachable, suspend/resume clear+flush+drain
- `~/.config/opencode/skill/aur-guides` — aur-rpc, aur-makepkg, aur-audit (namcap, shellcheck, PKGBUILD)
- Link: `.opencode/skill/*` + `.opencode/skills/*` → `~/.config/opencode/skill/*`, `opencode.json` skills.paths

## Build & Test
```bash
sudo pacman -Sy
cargo fmt --check; cargo clippy -- -D warnings -- -W clippy::unwrap_used # 0 warnings
cargo test # 15/15 (lib 10, aur_nan 2, integration 4, tui_snapshot 3) + doc 0
pacseek --init-config          # → ~/.config/pacseek/config.toml (edit colors/layout/timeouts)
pacseek --show-config          # prints XDG + project-local paths
cargo run -- firefox --no-tui --limit 2 --no-color
cargo run -- --tui             # TTY: type firefox → Enter, ↑↓, Enter→confirm Y, i→info, /→search, q→quit
cargo run -- --config ./my.toml firefox --no-tui
NO_COLOR=1 cargo run -- --tui # plain
cargo build --release # 5.6M stripped
cargo install --path . --force # ~/.cargo/bin/pacseek 0.2.0
```

## Flows
- **CLI:** `pacseek firefox` → `spawn_blocking search_repo` + `search_aur` `tokio::join!` → `print_packages` repo first
- **TUI:** `pacseek` or `pacseek --tui firefox` → `App::new_with_cli` (limit/source/by/regex/installed_only/no_color) → `trigger_search` mpsc+thread parallel → `poll_search_results` dedup `search_id` → `List` → `Enter` → `Confirm` → `suspend_and_run` → `install_*` → `last_query.clear()` → `trigger_search` refresh

## Phases — All Done
- **A** stability: NaN-safe, unicode-width, unwrap_or_else, panic unwind, signal-hook, try_restore — `68205d6`
- **B** perf: mpsc+thread parallel, OnceLock client, cached Config, tokio::join! — `cc0c62d`
- **C** bugs: Esc dedup, ListState None, CLI filters piped, NO_COLOR, pacman -Ss wrapped — `321b77d`
- **D** polish: floor 60x13, help contextual, centered_rect manual, suspend flush/drain — `39523b3`
- **E** hardening: votes alias, makepkg ask, /tmp 0o700 sanitize, shellcheck — `bc15b9f`
- **F** release: fmt/clippy -D warnings, 5.4M, `v0.2.0` `31719f5` tag `v0.2.0`
- **G** config: XDG + project-local TOML, [search]/[tui]/[theme]/[behavior], parse_style/border, --config/--init-config/--show-config, wired tui/ui + aur/install — `pending`

## Repo
- `master` 8 commits: `7d1ebd9` search, `772e410` TUI, `68205d6` A, `cc0c62d` B, `321b77d` C, `39523b3` D, `bc15b9f` E, `31719f5` v0.2.0
- `git tag v0.2.0` annotated
- `.gitignore` `/target`, `.opencode/skill` symlinks to global
- `task.md` removed per request, replaced by this `context.md`
