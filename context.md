# pakmap — Project Context

## Overview
Fast **AUR + Official Repo + Flatpak** search + TUI install/remove/upgrade/clean for Arch/Artix, Rust `0.6.0`, at `/home/achraf/projects/pakmap` (`pakmap` binary, `MIT`).
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
- Config: `~/.config/pakmap/config.toml` + `./pakmap.toml` project-local, `[search]/[tui]/[theme]/[behavior]` via `crate::config::Config` (serde TOML, defaults < file < env < CLI)
- Profile `release` `opt-level=z lto codegen-units=1 strip panic=unwind` **5.4M** `target/release/pakmap`, `dev` `opt-level 0`

## Architecture
```
src/
  main.rs       # IsTerminal, --tui/--no-tui/--json, Config::load + CLI merge (defaults < file < env < CLI), tokio::join! parallel repo+aur, spinner from config
  lib.rs        # pub mod cli/config/install/model/output/search/tui
  cli.rs        # Clap derive, query:Option<String>, Source (All/Aur/Repo), AurBy (name/name-desc...), limit 50, regex, installed_only, bottom_up, verbose, no_color, no_tui, tui, --config/--init-config/--show-config
  config.rs     # Config {search,tui,theme,behavior} serde TOML, XDG ~/.config/pakmap/config.toml + ./pakmap.toml, example_toml(), parse_style/border_type, aur_rpc/cache_dir helpers, 9 tests
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
cargo test # 148/148 (lib 108, aur_nan 2, chaos 4, integration 17, tui_snapshot 17) + doc 0
pakmap --init-config          # → ~/.config/pakmap/config.toml (edit colors/layout/timeouts)
pakmap --show-config          # prints XDG + project-local paths
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
cargo install --path . --force # ~/.cargo/bin/pakmap 0.6.0
```

## Flows
- **CLI:** `pakmap firefox` → `spawn_blocking search_repo` + `search_aur` `tokio::join!` → `print_packages` repo first
- **TUI:** `pakmap` or `pakmap --tui firefox` → `App::new_with_cli` (limit/source/by/regex/installed_only/no_color) → `trigger_search` mpsc+thread parallel → `poll_search_results` dedup `search_id` → `List` → `Enter` → `Confirm` → `suspend_and_run` → `install_*` → `last_query.clear()` → `trigger_search` refresh

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
- **Q** perf P3 (parallel): TUI Updates worker runs repo/AUR/flatpak on 3 threads (mirror of search fan-out, in-thread timings + panic guards); `remote-info` loop → 4-way scoped pool; `check_flatpak_updates_for/cached_for` take a pre-fetched list so `--stats` spawns `flatpak list` once; CLI `--updates` repo joins the tokio fan-out, `--stats` runs all 4 probes concurrently; measured — `--updates` wall 4.0s→1.7s, `--stats` wall 4.0s→1.6s — 142 tests
- **R** perf P4 (cache): `list_installed_cached()` 60s inventory memo + `invalidate_flatpak_list_memo()` paired at all 6 updates-memo invalidate sites (refresh excluded — pacman -Sy never changes flatpak inventory); `with_config`/trait/`--stats` all serve the cached list; verified live that `flatpak install` on an installed app is a safe no-op (exit 0), so Search-row routing needs no change — 143 tests
- **S** perf P5 (chaos+gates): `tests/chaos_shims.rs` — fake flatpak/paccache on PATH under a mutex+restore guard (garbage/exit-1/hung remote-info all fail-open; slow test invalidates memos first so the bound is genuinely exercised, ~4s); caught a real flaw while writing it (shared updates memo served sibling tests); `[lints.clippy] unwrap_used/expect_used="deny"` in manifest + cfg_attr allow for lib tests + file allows for integration crates; CI now runs `--lib` and `--test chaos_shims`; measured delta — `--updates`/`--stats` walls track slowest backend not the sum (e.g. 3.2s vs 4.3s serial, 5.6s vs 8.9s serial on a loaded box; absolute ms vary with cache/network warmth) — 147 tests
- **T** CI fix: pure-Rust `process::has_tool()` PATH scan (+ test) replaces all 4 `Command::new("which")` probes (clean/flatpak/aur/diff) — the Arch CI container lacks a `which` binary, so every probe false-negatived and all chaos asserts failed there while green locally; proven by running the chaos binary under `env -i PATH=/tmp/minbin` (old spawn → exit 127, new scan → 4/4 pass); integration presence helpers now use the real probes — 148 tests
- **U** current CI green: unit step failed twice fast (~10s) after `--lib` first ran lib tests in CI; made the "offline" step honestly offline (`PAKMAP_SKIP_LIVE=1` + news-fetch guard) and split lib/snapshot steps with a diagnose step that surfaces failing test names as annotations; `81d3ccd` fully green (fmt/clippy/unit/integration/release) — whether the middle failure was live-flake or env, the step is now hermetic and self-diagnosing
- **V** landing audit (webdesign-review skill, 20 domains) + fixes: Rust side verified clean (fmt/clippy `-D warnings`/148 tests all passing); landing lint/typecheck/build green incl. `NEXT_PUBLIC_BASE_PATH=/pakmap` CI-parity build — fixes: WCAG 1.4.3 contrast drops on `text-muted-foreground/70`/`/80` informational text (Install command comments, TerminalDemo placeholder, Footer blurb); version single-source from `Cargo.toml` via generated `src/lib/version.ts` + `scripts/sync-version.mjs` (`predev`/`prebuild`, was hardcoded "0.3.0") — note: Next 16 `env` config is legacy and Turbopack inlines unset `NEXT_PUBLIC_*` as "", so committed generated module chosen over bundler magic; MSRV text `1.85+`→`1.88+` + `Flatpak` dep line; Flatpak feature card added (completes 3×3 features grid) + hero/footer copy now mention Flatpak (0.6.0 headline feature was missing); brand favicon `src/app/icon.svg` replaces default Next `favicon.ico`, boilerplate `public/*.svg` deleted; SEO: `metadataBase`+canonical+og:image (GitHub assets URL)+twitter card+`theme-color`/`colorScheme` via `viewport` export + `robots.ts`/`sitemap.ts` (need `dynamic="force-static"` under `output:export`); a11y: version badge sr-only markup plain span aria-label, `aria-current="location"` for section nav, CopyButton `role="status"` live region + failure feedback; `design-system/pakmap/MASTER.md` palette synced to shipped tokens; landing README rewritten from create-next-app boilerplate (was describing Geist/Vercel that do not apply) — left untouched: `docs/index.html` (owner-maintained legacy page, updated at rename commit — candidate for consolidation, needs owner decision)
- **W** perf sprint P1+P2 (skills: observability, back-of-the-envelope, caching, resilience-failure): baseline benchmarked (debug walls via `/tmp/opencode/bench.sh`: repo 384ms, aur 656ms, flatpak 3.4–4.5s, all 3.1s, updates 2.0s, stats 3.0s; `flatpak remotes` spawn only 36ms — the 3.4s IS flatpak loading its 49MB/11MB-gz appstream on every `flatpak search`, so fix = stop spawning) — **flatpak search now reads the local index directly**: `<install>/appstream/<remote>/<arch>/active/appstream.xml` (user XDG + system roots, dedupe by app-id, early-exit at limit, zero new deps: plain XML only, `.gz`-only layouts fall back to bounded `flatpak search`; fields parity — `<id>` (`.desktop` suffix stripped), version from first `<release version>`, summary/description entity-unescaped, keyword matching incl. case-fold, `url=flatpak:<remote>`); `PAKMAP_FLATPAK_APPSTREAM_ROOT` env seam isolates tests from host caches + serves chaos fixtures; 256MB cap treats oversized cache as absent (fail-open chain parse → spawn); Timer `flatpak-appstream` on the fast path; debugging story: parse looked hung in CI-style parallel runs — gdb backtrace showed `unescape_xml` spinning forever on a trailing entity (`…&amp;` cursor never advanced) — fixed + regression-tested; measured: flatpak wall 3.4–4.5s → **309–554ms** (5–8×, still carrying ~200–300ms debug overhead), all-sources 3.1s → **536ms**; updates/stats legs untouched (remote-info per app) — 160 tests, fmt/clippy clean
- **X** perf sprint P3 (AUR latency; `aur-guides` skill GONE — `.opencode/skill` symlinks dangle to deleted dirs, conventions carried by the code itself: RPC v5, `by=`, popularity sort): reqwest gains `gzip` (+~0.2MB, transfer halved on 15KB JSON rows); all 5 AUR-speaking client builds collapsed onto shared tuned builders `aur::client_builder`/`blocking_client_builder` (gzip decode, `connect_timeout` 5s — dead network fails in seconds not the whole budget, `tcp_nodelay`, UA) — 3 async sites in `main.rs` + blocking in `aur.rs` + blocking in `updates.rs`; one-shot per-process connection warm-up (`AUR_WARMED` flag): cheap HEAD at RPC origin (derived pure fn `aur_warm_origin`, config-aware incl. custom mirrors/ports, skipped under `PAKMAP_SKIP_LIVE`) fired detached — CLI: `tokio::spawn` warm before each tokio fan-out (search/updates/stats ×3), TUI: detached std thread at Search+Updates orchestrators (initialized blocking, moved onto own thread so it can't serialize the orchestrator) so the TLS+DNS handshake overlaps other legs and every later query rides the pooled connection; Timer `aur-warm` observability; measured: debug aur 990→**375ms**, all 536→**343ms**; release steady-state: aur 507→**~320ms** (target 350 hit), flatpak **147ms** (23× vs 3.4s), all **~490ms**, repo ~440ms
- **Y** perf sprint P4 (staged/streamed TUI results; `ratatui`/`tui-design` skills also dangling, snapshot suite kept as the design guardrail): search channel `(u64, Vec<Package>, String)` → `SearchMsg { search_id, stage, query, packages }` with `SearchStage::Repo/Aur/Flatpak/Done`; the search orchestrator sends each leg the moment its join completes instead of holding the UI for the slowest backend (sequential joins pin the arrival order to the old combined-list order repo→aur→flatpak; per-stage `active_search` generation guards so a superseded search drops out incrementally, not only at the end); UI-side `staged_packages`+`staged_search_id` scratch per generation — first arrival REPLACES the scratch (superseded leftovers never mix; regression-tested: a ghost stage from search A cannot leak into search B), later legs append, visible list + cursor stay on the last complete generation, interim status ticks `Searching for 'q' — N found so far...`; `Done` (empty payload for search mode) swaps the whole generation in atomically via existing `apply_results` (is_loading off, `Found N (repo X aur Y flatpak Z)` exact format unchanged, cache filled once at completion); installed/updates workers keep their single-message flow with the list carried inside `Done`; scratch cleared on mode switch + cache invalidation; measured: TUI first paint now lands at the repo leg (~150–400ms debug) instead of the max leg; total wall unchanged (same joins) — 166 tests (+4 staged-flow tests: accumulate→atomic swap incl. exact final status, superseded discard, carry-payload `Done`, ghost scratch replace; 17 snapshots untouched)
- **Z** perf sprint P5 (resilience/caching/observability audit + docs + release; caching/resilience-failure skills loaded at P2, observability at P1): memo invalidation audit — every mutation site paired correctly (CLI install/remove `main.rs:325/490`, TUI install/remove, `do_upgrade`, internal install/remove fns; refresh correctly excluded per phase R — `pacman -Sy` never changes flatpak inventory) and **caught 1 real miss**: TUI `c` clean with the unused-runtimes stage shrank inventory but never invalidated the 60s list memo — now invalidated on success only when the flatpak stage ran (CLI `--clean` exits before memo reuse); W/X/Y failure paths re-reviewed against the sprint design — appstream fail-open chain (missing/oversized/unparsable → bounded spawn), warm-up non-fatal + `PAKMAP_SKIP_LIVE`, staged sends `let _`-guarded, no unwrap/expect in prod paths; docs: README flatpak bullet (local index + fallback), Performance section (final release walls), troubleshooting `.gz`-only fallback note; release: v0.6.0 tag was already on origin (pre-rename 0.6.0) — version bumped to **0.7.0** and tagged as the first pakmap-named release (landing version auto-syncs from Cargo.toml) — 166 tests, fmt/clippy `-D warnings` clean, 5.5M release binary; final sprint walls (release, steady): repo ~440ms, aur ~320ms, flatpak ~147ms, all ~490ms, staged TUI first paint ~ repo leg

## Repo
- `main` (synced with `origin/main`); history includes `7d1ebd9` search, `772e410` TUI, `68205d6` A, `cc0c62d` B, `321b77d` C, `39523b3` D, `bc15b9f` E, `31719f5` v0.2.0 (older commits; `main` now also carries landing + 0.3.0 work)
- no local `git tag` (historical `v0.2.0` tag not present locally)
- `.gitignore` `/target`, `.opencode/skill` symlinks to global
- `task.md` removed per request, replaced by this `context.md`

