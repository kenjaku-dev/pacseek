# pacseek — Phase-by-Phase Task Plan

> Goal: `firefox` bar → results → `Enter` install / `i` info, stable, fast, on Arch/Artix.
> Stack: `ratatui 0.30 + crossterm 0.29 + tui-input 0.15` (tui-design, ratatui skills) + `aur-guides` (libalpm, RPC, makepkg)

Source: audit `src/tui/app.rs:281` blocking, `src/search/aur.rs:128` NaN panic, `Cargo.toml:50` panic=abort, `src/tui/ui.rs:65` byte slice, `src/search/aur.rs:154` client per keystroke, `src/search/repo.rs:13` alpm per search.

## How to use

Check one phase at a time. Do not start next until `Verification` passes. Run `cargo fmt; cargo clippy; cargo test` after each phase. Commit with `git commit -m "phase-X: ..."`.

## Phase 0 — Baseline (done)
- [x] `src/search/aur.rs:6` async+blocking RPC, `src/search/repo.rs:6` alpm, `src/tui/mod.rs:9` try_init, `src/install/*` sudo/makepkg
- [x] Tests: `cargo test` 7/7, `cargo run -- firefox --no-tui` 4, `pacseek --tui` TTY check `src/main.rs:19`
- Verify: `cargo test --test tui_snapshot` 3/3 `src/tui/ui.rs:12` floor

## Phase A — Stability (CRITICAL) — 1d — ✅ DONE 2026-09-01

**Skills:** `ratatui` panic safety, `tui-design` lifecycle, `rust-common-pitfalls`

- [x] **A1** Fix `unwrap` on `NaN` `src/search/aur.rs:128,191` → `partial_cmp(...).unwrap_or(Ordering::Equal)`; add `is_finite` guard. `src/search/aur.rs:126,192` now uses `filter(|v| v.is_finite()).unwrap_or(0.0)` + `unwrap_or(Equal)`. Verify: `tests/aur_nan.rs:1` `sort_with_nan_does_not_panic` passes.
- [x] **A2** Fix byte slice panic `src/tui/ui.rs:65` → `unicode_width::UnicodeWidthStr::width` + char-boundary walk; test with `fire🦀fox` CJK. `src/tui/ui.rs:64` now `UnicodeWidthChar` + `char_indices().rev()` + `floor_char_boundary` removed (MSRV 1.85). Added `unicode-width 0.2` `Cargo.toml:39`. Verify: `tests/aur_nan.rs:33` `unicode_truncate_no_panic` passes.
- [x] **A3** Fix `template(...).unwrap()` `src/main.rs:148` → `unwrap_or_else(|_| default_spinner())` `src/main.rs:144` safe fallback.
- [x] **A4** `Cargo.toml:50` panic=abort → changed release `panic="unwind"` with comment, keep stripped; dev stays unwind. `src/tui/mod.rs:12` `try_init/try_restore` best-effort.
- [x] **A5** Signal handling `src/tui/app.rs:79` only Ctrl+C → added `signal-hook 0.3` `Cargo.toml:40`, `Arc<AtomicBool>` flag `src/tui/app.rs:79` registers `SIGTERM/SIGINT/SIGHUP`, loop checks `term_flag.load` `src/tui/app.rs:96`, `src/tui/mod.rs:9` `try_restore` with manual `disable_raw_mode`+`LeaveAlternateScreen`+`Show` fallback + `suspend_and_run` child/reentry aggregation `src/tui/mod.rs:20`.
- Verify: `cargo clippy -- -W clippy::unwrap_used` 0 warnings, `cargo test` 9/9, `cargo run -- --tui` in non-TTY now friendly `requires a terminal` `src/main.rs:36` not panic, multibyte no panic, `cargo test --test aur_nan` 2/2.

## Phase B — Performance (blocking UI) — 0.5d — ✅ DONE 2026-09-01

**Skills:** `ratatui` Async with Tokio, `tokio`, `tracing`+`tui-logger`, `tokio` signal, `aur-guides` cache

- [x] **B1** Blocking `do_search` `src/tui/app.rs:281` → `mpsc::channel` + `thread::spawn` background worker, `event::poll(200ms)` loop, `poll_search_results` `src/tui/app.rs:144` dedup via `search_id`, parallel repo+aur via two `thread::spawn` join `src/tui/app.rs:384`, `is_loading` spinner before block. Verify: `tests` `trigger_search_is_non_blocking` <100ms `src/tui/app.rs:453` 2/2.
- [x] **B2** Reuse `reqwest::blocking::Client` single `OnceLock` `src/search/aur.rs:6,154` `AUR_BLOCKING_CLIENT` `blocking_client()` not per 400ms; `src/search/aur.rs:154` now `blocking_client()` vs `Client::builder` per call. Share with async `src/main.rs:76`.
- [x] **B3** Cache `Config::new()` `src/search/repo.rs:13` via `OnceLock<Config> CACHED_CONFIG` `src/search/repo.rs:8` `get_cached_config()` `src/search/repo.rs:12`, new `search_repo_with_config` `src/search/repo.rs:21`, `search_repo` delegates. `App` benefits via `search_repo` in background thread, invalidates only on new process.
- [x] **B4** Fix `futures::future::join` `src/main.rs:4` → `tokio::join!` `src/main.rs:157`, `spawn_blocking` `JoinError` handled `match res { Ok(v)=>v, Err(e)=>log vec![] }` `src/main.rs:102`, removed `futures` dep `Cargo.toml:37`. Verify: `cargo clippy` 0 warnings, `cargo test` 11/11.
- Verify: `cargo test` 11/11, `cargo run -- firefox --no-tui` still parallel (`tokio::join!`), TUI `trigger_search` <100ms, spinner visible at `src/tui/app.rs:364` `Searching for`, debounced 400ms `src/tui/app.rs:124` still, `is_loading` polled `src/tui/app.rs:144`.

## Phase C — Logic Bugs — 0.5d

**Skills:** `tui-design` interaction-patterns, `rust-common-pitfalls`

- [ ] **C1** Key clash `src/tui/app.rs:152,188,202` `q/i/j/k` before type-to-search `src/tui/app.rs:231` → make `q` quit only when `focus==List && popup==None && input.value().is_empty()`, or require `Ctrl+q`; same for `i`/`j/k` only when not typing. Test typing `query` doesn't quit.
- [ ] **C2** Popup swallow `src/tui/app.rs:170` `'/'` and `src/tui/app.rs:111` `handle_popup_event` returns true for any Popup → only swallow handled keys.
- [ ] **C3** `Esc` duplicate `src/tui/app.rs:160` vs `handle_popup_event:120` → remove unreachable branch, single source.
- [ ] **C4** `ListState(Some(0))` on empty `src/tui/app.rs:55` → init `None`, `do_search` `src/tui/app.rs:327` already fixes but test empty start `tests/tui_snapshot.rs` already covers.
- [ ] **C5** TUI ignores CLI filters `src/tui/app.rs:37` hardcodes `limit 50`, `by name-desc` → pipe `Cli.source/limit/regex/installed_only/by` into `App::new(initial, cli)` `src/main.rs:42`.
- [ ] **C6** `NO_COLOR` empty block `src/main.rs:32` → honor via `if supports_color` or `ratatui::style::Color::Reset`, test with `NO_COLOR=1 pacseek --tui`.
- [ ] **C7** `pacman -Ss` fallback brittle `src/search/repo.rs:188` → handle wrapped lines, `status.success` vs `no results` distinct.
- Verify: `cargo test --test tui_snapshot` add test `type 'q' in Search does not quit`, manual TUI `q` in List quits, `q` in Search types.

## Phase D — TUI Polish (visual-patterns, responsive) — 0.5d

**Skills:** `tui-design` visual-patterns, `ratatui` Layout, `insta` snapshots

- [ ] **D1** Clutter audit `src/tui/ui.rs:86,130` both Rounded borders → keep 1 border, `status/help` borderless fine, but align title colors to semantic tokens (cyan repo, yellow AUR already `src/tui/ui.rs:141`).
- [ ] **D2** Floor mismatch `src/tui/ui.rs:16` `60×10` vs layout need `13` `src/tui/ui.rs:28` → enforce `60×13` minimum, align `README.md:79` `60×10` to `60×13`, test 60×24,80×24,120×40 `rstest`.
- [ ] **D3** Help lies `src/tui/ui.rs:239` `q:quit` shown in Search but `q` types `src/tui/app.rs:152` → contextual help per `focus`, add `Ctrl+C` hint, `y/n` only in confirm `src/tui/ui.rs:368`.
- [ ] **D4** Suspend flush `src/tui/mod.rs:29` `terminal.clear()` → add `terminal.flush()`, drain `EventStream`, test `suspend_and_run` with `sudo -v` child + reentry both-fail case per skill.
- [ ] **D5** `centered_rect` `src/tui/ui.rs:417` 2 Layout alloc per popup/frame → cache or `Layout::init` once.
- Verify: `cargo insta test --accept` snapshots at 3 sizes pass, `Rat::TestBackend` `Buffer::with_lines` for selected row highlight per skill, no chrome >20%.

## Phase E — Install Hardening (aur-guides) — 0.5d

**Skills:** `aur-guides` aur-makepkg, aur-audit, aur-pkgbuild

- [ ] **E1** `makepkg --noconfirm` forced `src/install/aur.rs:106` vs repo `let pacman ask` `src/install/repo.rs:25` → make flag `--noconfirm` explicit, default ask.
- [ ] **E2** Cache race `src/install/aur.rs:22` `/tmp` fallback `0o700` + sanitize `name` `../` path-traversal `PathBuf::file_name`.
- [ ] **E3** `suspend_and_run` already handles restore, but add `terminal.clear()` + `draw` before install output `src/install/repo.rs:28` to avoid escape interleaving.
- [ ] **E4** `Package` duplicate `votes/num_votes` `src/model.rs:12,16` → deduplicate or document `src/search/aur.rs:57`.
- [ ] **E5** Add `namcap` audit already `src/install/aur.rs:78` but gate behind `which namcap`, add `shellcheck` for PKGBUILD per `aur-guides`.
- Verify: `makepkg -si` as user (not root) `nix::geteuid` `src/install/aur.rs:14` blocks root, `git pull --ff-only` not overwrite dirty, `cargo test` with tempfile clone.

## Phase F — Release & Docs — 0.2d

- [ ] `cargo fmt; cargo clippy -- -D warnings; cargo audit; cargo deny check`
- [ ] `cargo build --release` `target/release/pacseek` 4.9M → strip already, check `opt-level=z` `Cargo.toml:46`
- [ ] `Makefile` `make dev, make install, make update-db`
- [ ] `README.md` sync TUI help, `INSTALL.md` `sudo pacman -Sy` + `base-devel`
- [ ] `git tag v0.2.0` `git log --oneline`

## Verification checklist (each phase)

- [ ] `cargo fmt --check && cargo clippy && cargo test` 7/7
- [ ] `timeout 10 pacseek firefox --no-tui --limit 2` found, `pacseek --tui` in TTY shows `firefox` list, `i` info, `Enter` confirm, `q` quit
- [ ] `NO_COLOR=1`, `80×24`, `60×10` too-small, `Ctrl+C`/`kill -TERM` restores terminal (no raw leak)
- [ ] `cargo insta review` 3 snapshots, `hyperfine "pacseek firefox --no-tui"` vs `paru -Ss`

## Skills per phase

- A: `ratatui` panic safety, `tui-design` lifecycle, `rust-common-pitfalls`
- B: `ratatui` Async with Tokio, `tokio`, `tracing`/`tui-logger`
- C: `tui-design` interaction-patterns, `aur-guides` aur-rpc
- D: `tui-design` visual-patterns, `ratatui` TestBackend, `insta`
- E: `aur-guides` aur-makepkg/aur-audit, `nix`, `dirs`
- F: `clippy`, `audit`, `deny`

> Work one phase at a time, commit, then request next.
