# pacseek — Project Context

## Overview
Fast **AUR + Official Repo + Flatpak** search + TUI install/remove/upgrade/clean for Arch/Artix, Rust `0.6.0`, at `/home/achraf/projects/pacseek` (`pacseek` binary, `MIT`).
One binary: `libalpm` local DB (no network) + AUR `https://aur.archlinux.org/rpc/v5` + `flatpak --json` + `ratatui` TUI matching drawing: top `firefox` search bar → results list → `Enter` install / `i` info+deps, `Tab` cycles Search→Installed→Updates, `c` cleans cache, `U` upgrades all (pacman+flatpak).

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
- `rust 1.85+` (MSRV 1.88 for `ratatui 0.30`), `edition 2024`, `tokio` (`rt-multi-thread` + `macros`), `reqwest 0.12` (json+rustls+blocking via OnceLock), `alpm 5` + `pacmanconf 3`, `clap 4.5` derive, `serde`/`serde_json`/`toml 0.8`, `anyhow`/`thiserror`, `colored 2`, `indicatif 0.17`, `tracing 0.1`, `regex 1`, `url 2`
- TUI: `ratatui 0.30` `crossterm 0.29` `tui-input 0.15` `color-eyre 0.6` `dirs 5` `nix 0.29` `unicode-width 0.2` `signal-hook 0.3`
- Config: `~/.config/pacseek/config.toml` + `./pacseek.toml` project-local, `[search]/[tui]/[theme]/[behavior]` via `crate::config::Config` (serde TOML, defaults < file < env < CLI)
- Profile `release` `opt-level=z lto codegen-units=1 strip panic=unwind` **5.4M** `target/release/pacseek`, `dev` `opt-level 0`

## Architecture
```
src/
  main.rs       # IsTerminal, --tui/--no-tui/--json, Config::load + CLI merge (defaults < file < env < CLI), tokio::join! parallel repo+aur, spinner from config
  lib.rs        # pub mod cli/config/install/model/output/search/tui
  cli.rs        # Clap derive, query:Option<String>, Source (All/Aur/Repo), AurBy (name/name-desc...), limit 50, regex, installed_only, bottom_up, verbose, no_color, no_tui, tui, --config/--init-config/--show-config
  config.rs     # Config {search,tui,theme,behavior} serde TOML, XDG ~/.config/pacseek/config.toml + ./pacseek.toml, example_toml(), parse_style/border_type, aur_rpc/cache_dir helpers, 9 tests
  model.rs      # Package {name, version, description, repo, arch, url, installed, votes/popularity/out_of_date/maintainer/num_votes/last_modified} PartialEq
  search/
    aur.rs      # RPC v5 async + blocking, config aur_rpc + timeout (OnceLock vs per-call), NaN-safe sort (filter finite), regex client-side
    repo.rs     # alpm via OnceLock Config (get_cached_config), search_repo_with_config, fallback pacman -Ss (wrapped desc, indented lines)
  sources/      # 0.6.0 Backend trait seam (id/available/list/search/install/remove/updates + cfg)
    mod.rs      # SourceBackend trait with fail-open defaults; repo/aur keep old paths, flatpak implements
    flatpak.rs  # has_flatpak/remote_exists (which + remote-list), list/search --json parsers, remote-info updates + 5min memo, install/remove/update/unused runners (no sudo), app-id validator, command-args builders
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
  integration_search.rs # help/version/repo/json/regex-recall/updates/orphans/stats/readonly/clean (14)
  tui_snapshot.rs       # TestBackend 80x24, 50x8 too-small, info/confirm/help/upgrade/clean popups, tabs, badges (16)
  aur_nan.rs            # NaN sort, unicode truncate (2)
  lib (src/)            # model 2 + config 10 + remove 2 + tui::app 28 + aur 2 + repo-fallback 2 + ui 1 + confirm 3 + updates 3 + orphans 2 + news 4 + diff 5 + stats 2 + upgrade 1 + clean 5 = 72 — total 104 + doc 0
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
cargo test # 142/142 (lib 106, aur_nan 2, integration 17, tui_snapshot 17) + doc 0
pacseek --init-config          # → ~/.config/pacseek/config.toml (edit colors/layout/timeouts)
pacseek --show-config          # prints XDG + project-local paths
cargo run -- firefox --no-tui --limit 2 --no-color
cargo run -- --tui             # TTY: type firefox → Enter, ↑↓, Enter→confirm Y, i→info+deps, Tab→Installed (o orphans, c clean), Tab→Updates (U upgrade-all pacman+flatpak), q→quit
cargo run -- firefox --source flatpak --limit 3
cargo run -- --updates --json | jq .
cargo run -- --orphans --json | jq .
cargo run -- --stats            # incl. Flatpak apps/updates
cargo run -- --clean --dry-run # preview reclaimable cache (paccache -k2) + flatpak note
cargo run -- --config ./my.toml firefox --no-tui
NO_COLOR=1 cargo run -- --tui # plain
cargo build --release # 5.4M stripped
cargo install --path . --force # ~/.cargo/bin/pacseek 0.6.0
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
- **F** release: fmt/clippy -D warnings, 5.4M, `v0.2.0` `31719f5` tag `v0.2.0` (historical)
- **G** config: XDG + project-local TOML, [search]/[tui]/[theme]/[behavior], parse_style/border, --config/--init-config/--show-config, wired tui/ui + aur/install — done
- **H** 0.3.0: remover + refresh + 47 tests, 5.1M release (historical)
- **I** `0.4.0`: repo-regex recall, --remove/--refresh confirm, no-expect AUR clients, worker cancel, draw throttle, popup_help, confirm module + tests
- **J** orphan-spin fix: crossterm poll spins inside its fd-read loop on EIO (dead terminal) — no timeout, signals swallowed by EINTR-continue; parked watchdog probes stdio (/proc/self/fd re-open + zero-write) every 5s and exits 0 — verified e2e via pty harness (was 91% CPU forever, now self-exits rc 0)
- **K** `0.4.1`: orphan-spin watchdog fix — tag `v0.4.1`
- **L** `0.5.0`: maintenance+safety — Updates tab + `--updates/--upgrade` (vercmp + AUR info, news guard), orphans (`-Qdt`, `o` filter, `--orphans`), stats (`--stats`), PKGBUILD diff preview, badges `[OOD]/[ORPHAN]/[UNMAINTAINED]` + deps in info, readonly mode — 85 tests, 5.3M
- **M** `0.5.1`: cache cleaner + paper cuts — `--clean [--keep N] [--dry-run]` (paccache `-d` preview parse, `sudo -r` run) + TUI `c` + ConfirmClean popup, real RFC2822 news dates, inline upgrade warning (no two-`U` dance), AUR memo 5min + F5 invalidate, recursive cache sizes, fetch-before-diff (15s bound), BackTab reverse, `U` scoped to Updates, condensed help popup — 104 tests, 5.3M
- **N** `0.6.0`: flatpak backend in 5 phases — P1 `sources/` seam + detect/list (`--json` parsers, `flatpak_enabled/remote` config), P2 `Source::Flatpak` search fan-out (CLI tokio + TUI workers, `print_packages` 3-slice, `Found (repo aur flatpak)`), P3 install/remove without sudo (app-id validator, TUI routing by `repo`, CLI `--remove` auto-detect, `flatpak_noconfirm`), P4 updates+upgrade (`remote-info` version check + 5min memo, `--updates/--stats/--upgrade` incl. flatpak, Updates-tab worker, installed-row → `flatpak update`), P5 `--clean` also drops unused runtimes (CLI both-stages + TUI popup flag) — 131 tests, 5.4M
- **O** perf P1 (measure): `src/timing.rs` BackendTimer (warn >2000ms, info otherwise) on all 5 fan-outs (CLI search/updates/stats, TUI search/updates workers); fixed 2 real bugs found by instrumenting — logs wrote to stdout (broke `--json` pipes) now stderr, and `--updates/--stats` exited before tracing init (TUI branch moved before init so alt-screen stays silent); budgets recorded — search repo 145ms/aur 507ms/flatpak 3490ms cold (long pole), updates repo 326ms/aur 627ms/flatpak 1380ms, stats local 1398ms — 135 tests
- **P** perf P2 (bounds): `src/process.rs` bounded runner (`output_bounded`/`status_inherit_bounded`/`status_null_bounded`, try_wait poll + kill/reap, QUICK 15s/NETWORK 30s/CLONE 300s); migrated paccache preview, pacman -Q/-Qi/-Ss/-Qdt, git pull/clone, git diff/fetch, all 3 flatpak probes (run_flatpak/info/remote-info collapsed onto it); interactive mutate runs (sudo/makepkg/flatpak install) stay unbounded by design (user present, Ctrl-C); 6 runner tests incl. sleep-timeout reap — 141 tests
- **Q** current perf P3 (parallel): TUI Updates worker runs repo/AUR/flatpak on 3 threads (mirror of search fan-out, in-thread timings + panic guards); `remote-info` loop → 4-way scoped pool; `check_flatpak_updates_for/cached_for` take a pre-fetched list so `--stats` spawns `flatpak list` once; CLI `--updates` repo joins the tokio fan-out, `--stats` runs all 4 probes concurrently; measured — `--updates` wall 4.0s→1.7s, `--stats` wall 4.0s→1.6s — 142 tests

## Repo
- `main` (synced with `origin/main`); history includes `7d1ebd9` search, `772e410` TUI, `68205d6` A, `cc0c62d` B, `321b77d` C, `39523b3` D, `bc15b9f` E, `31719f5` v0.2.0 (older commits; `main` now also carries landing + 0.3.0 work)
- no local `git tag` (historical `v0.2.0` tag not present locally)
- `.gitignore` `/target`, `.opencode/skill` symlinks to global
- `task.md` removed per request, replaced by this `context.md`

