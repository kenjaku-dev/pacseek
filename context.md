# pacseek — Project Context

## Overview
Fast **AUR + Official Repo** search + TUI install for Arch/Artix, Rust, at `/home/achraf/Projects/pacseek` (`pacseek` binary, `mit`).
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
- `rust 1.85+` (MSRV 1.88 for `ratatui 0.30`), `edition 2024`, `tokio full`, `reqwest 0.12` (json+rustls+blocking via OnceLock), `alpm 5` + `pacmanconf 3`, `raur 8`, `clap 4.5` derive, `serde`/`serde_json`, `anyhow`/`thiserror`, `colored 2`/`owo-colors`, `indicatif 0.17`, `tracing 0.1`, `regex 1`, `url 2`
- TUI: `ratatui 0.30` `crossterm 0.29` `tui-input 0.15` `color-eyre 0.6` `dirs 5` `nix 0.29` `unicode-width 0.2` `signal-hook 0.3`
- Profile `release` `opt-level=z lto codegen-units=1 strip panic=unwind` (4.9M), `dev` `opt-level 0`

## Architecture
```
src/
  main.rs       # IsTerminal, --tui/--no-tui/--json branching, tokio::join! parallel repo+aur, spinner unwrap_or_else
  lib.rs        # pub mod cli/install/model/output/search/tui
  cli.rs        # Clap derive, query:Option<String>, Source (All/Aur/Repo), AurBy (name/name-desc...), limit 50, regex, installed_only, bottom_up, verbose, no_color, no_tui, tui
  model.rs      # Package {name, version, description, repo, arch, url, installed, votes/popularity/out_of_date/maintainer/num_votes/last_modified} PartialEq
  search/
    aur.rs      # RPC v5 async + blocking (OnceLock Client), NaN-safe sort (filter finite), regex client-side
    repo.rs     # alpm via OnceLock Config (get_cached_config), search_repo_with_config, fallback pacman -Ss (wrapped desc, indented lines)
  output.rs     # colored + json, votes/popularity, [installed]
  tui/
    mod.rs      # run/run_with_cli try_init/try_restore best-effort (disable_raw_mode+LeaveAlternateScreen+Show+DisableMouseCapture), suspend_and_run flush+drain EventStream, both-fail aggregation
    app.rs      # Input (tui-input), ListState None on empty, Focus Search/List, Popup Info/Confirm/Message, limit/source/aur_by/use_regex/installed_only/no_color, mpsc channel non-blocking trigger_search (next_search_id dedup), parallel repo+aur via thread::spawn, signal-hook SIGTERM/SIGINT/SIGHUP AtomicBool, poll_search_results, do_install suspend
    ui.rs       # Frame 60x13 floor, Layout vertical [3,Min(8),1,1] borderless status/help (<20% chrome), Search width-aware tail (UnicodeWidthStr, char_indices.rev), List virtualized highlight REVERSED when NO_COLOR, status/help contextual (Search: Enter/Esc/Ctrl+C, List: q:quit), Clear hole-punch, centered_rect manual math (no Layout alloc)
  install/
    repo.rs     # sudo pacman -S --needed (no --noconfirm by default)
    aur.rs      # sanitize PathBuf::file_name, /tmp fallback 0o700, git clone/pull --ff-only, bat/cat PKGBUILD, namcap+shellcheck gated, makepkg -si (PACSEEK_NOCONFIRM=1 for --noconfirm), nix::geteuid root check
  error.rs
tests/
  integration_search.rs # help/version/repo/json
  tui_snapshot.rs       # TestBackend 80x24, 50x8 too-small, info popup
  aur_nan.rs            # NaN sort, unicode truncate
```

## Skills Used
- `~/.config/opencode/skill/ratatui` (mte90, 0.30.1) — widgets, TestBackend, Layout cache, event, panic hook before init
- `~/.config/opencode/skill/tui-design` — alt-screen, non-blocking, NO_COLOR, 60x13 floor, clutter <1 border, keyboard-reachable, suspend/resume clear+flush+drain, "tui-design" workflow
- `~/.config/opencode/skill/aur-guides` — aur-rpc, aur-makepkg, aur-audit (namcap, shellcheck, PKGBUILD)
- Link: `.opencode/skill/*` + `.opencode/skills/*` → `~/.config/opencode/skill/*`, `opencode.json` skills.paths

## Build & Test
```bash
sudo pacman -Sy
cargo fmt; cargo clippy -- -W clippy::unwrap_used; cargo test # 11/11 (lib 2+4, aur_nan 2, tui_snapshot 3, integration 4)
cargo run -- firefox --no-tui --limit 2 --no-color
cargo run -- --tui             # TTY: type firefox → Enter, ↑↓, Enter→confirm Y, i→info, /→search, q→quit
NO_COLOR=1 cargo run -- --tui # plain
cargo build --release # 4.9M
cargo install --path . --force # ~/.cargo/bin/pacseek
```

## Flows
- **CLI:** `pacseek firefox` → `spawn_blocking search_repo` + `search_aur` `tokio::join!` → `print_packages` repo first
- **TUI:** `pacseek` or `pacseek --tui firefox` → `App::new_with_cli` (limit/source/by/regex/installed_only/no_color) → `trigger_search` mpsc+thread parallel → `poll_search_results` dedup `search_id` → `List` → `Enter` → `Confirm` → `suspend_and_run` → `install_*` → `last_query.clear()` → `trigger_search` refresh

## Known Gaps (Phases D/E/F done, F remaining if wanted)
- **D** polish done: floor 60x13, help contextual, centered_rect manual, flush/drain
- **E** hardening done: votes alias kept, makepkg default ask, /tmp 0o700 + sanitize, shellcheck, suspend both-fail
- Future: `cargo audit`/`deny`, `hyperfine`, `git tag v0.2.0`, `rstest` 120x40

## Repo
- `master` 4 commits: `7d1ebd9` search, `772e410` TUI, `68205d6` Phase-A, `cc0c62d` Phase-B, `321b77d` Phase-C, `39523b3` Phase-D, current `phase-E` (uncommitted until verify)
- `.gitignore` `/target`, `.opencode/skill` symlinks to global
