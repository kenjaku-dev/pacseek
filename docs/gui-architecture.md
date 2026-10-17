# pakmap GUI — architecture & IPC contract

Phase AB record (2026-10-17, base `main @ 961baaa`). Decision locked: **Tauri 2.12 shell
over the existing `pakmap` core library** — React 19 + TypeScript + Tailwind 4 frontend,
WebKitGTK system webview, single binary, zero logic duplication.

Visual companion: [`gui-architecture.html`](./gui-architecture.html) (self-contained dark
diagram; PNG/PDF export via the ⋯ menu).

## Spike numbers (decision gate)

| Metric | Result | Gate |
|---|---|---|
| Release binary | **4.1 MB** (`target/release/pakmap-gui`) | ≤ 10 MB budget from the sprint — pass |
| `cargo build --release -p pakmap-gui` (cold) | 4m 01s | acceptable (deps cached after first build) |
| Smoke run `PAKMAP_GUI_SMOKE_MS=4000` | exit 0, no panic output | pass (auto-quit hook) |
| webkit2gtk-4.1 | 2.54.1 present (pacman) | pass |
| Frontend `tsc --noEmit && vite build` | 31 modules, ~1.4 s, 71 KB gzip JS | pass |

Decision recorded: **Tauri confirmed** (no egui fallback needed). Startup/interaction
budgets (cold start ≤ 1 s interactive; repo leg paints first) are enforced in AF with
real measurements — the smoke harness proves the auto-quit path, not wall-clock budgets.

## Scope (frozen via parity matrix — anything not listed is v0.9)

### In scope (v0.8.0)

- **FR-1 Search parity:** staged streaming search (repo → AUR → flatpak) with the TUI's
  generation-guard semantics; filters: source, aur_by, regex, installed_only,
  orphans_only, limit.
- **FR-2 Inspection:** detail pane — description, version, repo, installed/update badge,
  AUR popularity, flatpak app id.
- **FR-3 Operations:** install / remove / upgrade / clean with pkexec escalation
  (polkit dialog), user-scope flatpak ops, honest failure states.
- **FR-4 Updates view** (repo + AUR), **FR-5 news feed**, **FR-6 orphans + related/
  diff actions**, **FR-7 config + theme reuse** (`config.toml` / `theme.toml` shared
  with CLI/TUI as single source of truth).

### Non-functional requirements

- **NFR-1** Cold start ≤ 1 s to interactive on release; first painted results = repo leg.
- **NFR-2** AUR warm pool fired at app launch (inherits the ~320 ms sprint guarantee).
- **NFR-3** No `unwrap`/`expect` in the GUI crate (same P5 lint contract as core).
- **NFR-4** CI gates: `cargo fmt/clippy` on the workspace, `pnpm -C gui typecheck/build`,
  166-test CLI/TUI contract stays green at every phase.
- **NFR-5** GUI never runs as root; pacman ops via pkexec, flatpak ops user-scope.

### Explicit out of scope (v0.8.0)

Flathub/Flatpak packaging (AUR PKGBUILD only, AE/AF), Windows/macOS, mobile, system tray,
daemon/service mode, i18n, auto-update, per-window multi-search, plugin surface.

### Parity matrix (TUI surface → GUI surface → phase)

| TUI surface | GUI equivalent | Phase |
|---|---|---|
| Search input (fuzzy, debounced) | search field + filter bar | AC |
| Staged per-source results (Repo/Aur/Flatpak) | per-leg stream groups, live totals | AC |
| Detail view | detail pane | AC |
| Install / remove (keybind flows) | row actions + confirm dialog + progress | AD |
| Update view | Updates tab | AD |
| Clean | Clean section | AD |
| News | News tab | AD |
| Orphans / related | sidebar tools | AD |
| Status bar / stats | status strip | AE polish |
| Themes | token-mapped light/dark/system | AE |

## Workspace & process layout

```text
pakmap/                        # workspace root; default-members = ["."] (CLI/TUI gates untouched)
├── src/                       # pakmap lib + CLI + TUI (frozen contract, 166 tests)
├── gui/
│   ├── package.json           # pnpm@12.14.0, vite 7, tauri CLI 2.12 lockstep
│   └── src-tauri/             # pakmap-gui crate (workspace member)
│       ├── src/lib.rs         # commands + event surface (this contract)
│       ├── tauri.conf.json    # identifier io.github.kenjaku-dev.pakmap, bundle off (AF)
│       └── capabilities/      # least-privilege IPC allowlist (core:default today)
└── .github/workflows/ci.yml   # path-filtered gui job (archlinux container)
```

`pakmap-gui` is a path-dependency consumer of `pakmap` (the lib). The CLI/TUI binaries
are untouched; `default-members = ["."]` keeps every existing bare `cargo` invocation
(root crate only) CI-compatible. MSRV is workspace-pinned to **1.90** (Tauri 2.12 policy).
Tauri crate, CLI, and JS API stay pinned in lockstep on 2.12.x; upgrades re-evaluated
after v0.8.0.

## IPC contract (single source of truth for AC/AD work)

Commands are `#[tauri::command]` fn's in `gui/src-tauri/src/lib.rs`, stateless over the
lib (no config duplication — `Config::load()`/lib calls own the truth). **Rust note:
commands must not be `pub fn`** — on rustc ≥ 1.99 the command macro's generated
`pub use {__cmd__*, …}` collides with `#[macro_export]` for public commands (E0255);
keep commands crate-private.

### Shipped (AB spike → AC search MVP; AD ops below)

| Command | Signature | Returns |
|---|---|---|
| `greet` | `{ name: string }` | `string` (IPC liveness probe) |
| `get_config_summary` | — | `ConfigSummary` (incl. `debounce_ms` clamped 50..=5000) |
| `search` | `{ query, filters, onEvent: Channel<SearchEvent> }` | `u64` generation; staged events stream on the channel |
| `cancel_search` | — | `u64` (new generation; in-flight sends drop) |
| `package_info` | `{ name }` | full `PackageDto` (installed rows: deps/reason via localdb) |

`filters` (`SearchFilters`) = `{ source: "all"|"repo"|"aur"|"flatpak", aur_by,
use_regex, installed_only, orphans_only, limit }`. Backend semantics mirror the TUI
orchestrator exactly: `installed_only`/`orphans_only` become the LOCAL path
(`search_local` + `list_orphans` intersection, enriched orphan badges) carried by a
payload `done` event; otherwise three parallel legs join sequentially, each sent the
moment it completes behind the `active_search` generation guard, then an empty `done`.
A one-shot AUR warm (HEAD) overlaps the repo leg.

`ConfigSummary` = `{ app_version, limit, source, timeout_secs, flatpak_enabled,
custom_aur_rpc, debounce_ms }` — resolved-effective config (user file, fail-open
defaults).

Frontend guard model: the hook binds a monotonic client identity into the per-call
event sink, so IPC ordering races cannot mix runs; the backend's own generation guard
prevents wasted sends for superseded searches. Vitest fixtures mirror the TUI's four
staged-flow tests (accumulate→swap, superseded discard, carried Done, fetch reset).

### Shipped (AD — operations)

| Command | Request | Response |
|---|---|---|
| `run_op` | `{ op: OpRequest, onEvent: Channel<OpMessage> }` | `u64` op_id; progress/done stream on the channel |
| `cancel_operation` | `{ opId }` | `bool` — flag is honored at phase boundaries only |
| `drop_operation` | `{ opId }` | frees the cancel-flag registry entry |
| `get_config_flags` | — | `{ readonly, flatpak_enabled }` (actions disable up-front) |
| `get_updates` | — | `UpdatesResult` (both legs degrade independently) |
| `get_news` | — | `NewsItem[]` (RSS subset, `needs_action` flag) |

`OpRequest` is a tagged union on the wire: `{ "kind": "install_repo", "args":
{ "name" } }`, kinds `install_repo` / `install_flatpak` / `install_aur` / `remove` /
`remove_flatpak` / `remove_orphans` / `upgrade` / `refresh` / `clean { keep }`.

Screen layout of the surface: AUR installs are `clone (bounded) → build (makepkg,
unprivileged, cwd=clone dir) → pkexec pacman -U`; the clone dir matches the
TUI/diff module (`diff::clone_dir_for`); built *.pkg.tar.{zst,xz,gz} artifacts are
globbed after makepkg. Repo ops are single-phase; flatpak ops run user-scope
unprivileged (`--noninteractive`). Post-op, the AUR update memo invalidates
(phase-Z audit rule: stale memos lie).

Replacement of the AB "planned" list: `updates` → `get_updates`; the `list_installed`
command was folded into the AC search local path (installed-only queries already hit
localdb); `stats` remains AE polish. Mutating ops split into `run_op(kind,…)` +
events instead of per-kind commands so one cancel-semantics implementation covers
all nine kinds.

### Streaming events (one channel per operation)

Search mirrors the TUI's `SearchStage::{Repo, Aur, Flatpak, Done}` and reuses the same
`active_search` generation guard — superseded searches drop out incrementally, first leg
paints immediately:

```jsonc
// search://stage
{ "generation": 7, "stage": "repo",  "packages": [PackageDto…] }  // then "aur", "flatpak"
{ "generation": 7, "stage": "done",  "packages": [] }             // leg complete
```

Operations (AD) emit `{ kind: "progress"|"done" }` messages on one channel per op
(`indicatif` stays CLI-only; `percent` was dropped — pacman piped output carries no
reliable percentages, phases+lines are the honest signal):

```jsonc
// op messages
{ "kind": "progress", "op_id": 12, "phase": "install", "line": "installing htop-3.4" }
{ "kind": "done",     "op_id": 12, "ok": true, "exit_code": 0, "error": null }
{ "kind": "done",     "op_id": 13, "ok": false, "exit_code": 255,
  "error": { "code": "auth_dismissed", "message": "…", "hint": "…" } }
```

Frontend ops model: a single active operation (`opsReducer`) — progress lines
accumulate into a 100-line journal ring, `done` is terminal, messages for foreign
op ids and post-terminal stragglers are dropped, and dismissing the banner calls
`drop_operation`. One op runs at a time; the Cancel button stays honest with the
copy "cancels before the next phase — the running transaction is never killed
mid-flight".

### AD failure matrix (stderr/exit → `{ code, message, hint }`)

Classified from the bounded stderr tail in `ops.rs` (`classify_op_failure`); keep in
sync with the table when adding codes:

| code | trigger | the card tells the user |
|---|---|---|
| `pkexec_missing` | pkexec not on PATH (pre-flight) | install polkit + a polkit agent |
| `auth_dismissed` | pkexec "disapproved/dismissed/not authorized" | retry and answer the polkit dialog |
| `db_locked` | pacman "database is locked" | wait for the running pacman / stale db.lck |
| `network_failure` | sync/download failures, timeouts | check mirrors and retry |
| `unresolvable_deps` | "could not satisfy dependencies" | resolve the listed deps first |
| `package_not_found` | "target not found" | spelling/repo coverage |
| `readonly_mode` | `behavior.readonly = true` guard | how to turn readonly off |
| `paccache_missing` | paccache not installed | install pacman-contrib |
| `flatpak_missing` | flatpak not installed (pre-flight) | install flatpak or disable the leg |
| `aur_clone_failed` | git clone/pull failure (bounded 300s) | connectivity to aur.archlinux.org |
| `aur_build_empty` | makepkg produced no artifact | read the build journal (bad checksum/PKGBUILD) |
| `invalid_name` | empty/traversal package name | reject banner (guard, pre-flight) |
| `op_canceled` | cancel honored at a phase boundary | next-phase only, by design |
| `op_failed` | anything else | last stderr line + exit code |

### Error envelope

All command errors reject with `{ code, message, hint? }` (`GuiError`). Strings are
mapped from the CLI's `error.rs` hints — one source of truth, no GUI-specific copies.
Seeded codes (AB): `empty_query`, `repo_search_failed`; AC adds `aur_search_failed`,
`flatpak_search_failed`, `config_unreadable`; AD adds the privilege/ops matrix
(`pkexec_missing`, `auth_dismissed`, `db_locked`, `flatpak_missing`, …) — each maps to
an actionable error card with the CLI's hint text.

### Frontend DTO

`PackageDto` (AC): every `Package` field the detail pane needs — `{ name, version,
description, repo, arch, url, installed, votes, popularity, out_of_date, maintainer,
last_modified, depends, optdepends, required_by, optional_for, reason, orphan }`;
list badges (`OOD`/`ORPHAN`/`UNMAINTAINED`) keep `Package::badges()` semantics.
camelCase? — no: Tauri serializes field names as-is (snake_case), the TS types mirror
the Rust structs 1:1.

### Privilege & cancellation model

- GUI process stays unprivileged; pacman ops spawn `pkexec pacman` (polkit owns the
  auth UI); flatpak ops run user-scope. Missing pkexec ⇒ actionable error, never a
  root GUI.
- GUI divergence from the TUI (documented, deliberate): pacman/paccache in the GUI
  run **always non-interactive** (`--noconfirm`) because there is no TTY — the polkit
  dialog replaces pacman's own confirm prompt. AUR builds run makepkg against the
  cloned PKGBUILD (`-srf --noconfirm`); the live PKGBUILD review stays a TUI surface
  (the GUI copies the link instead); AUR dependency resolution inherits pacman -U's
  failure mode (missing AUR deps surface as `unresolvable_deps`).
- Cancellation: searches cancelable anytime (generation guard makes it free); pacman
  transactions never killed mid-transaction — op cancel applies between phases only
  (`OpsState` cancel flags are read at clone→build→install boundaries), surfaced in
  the UI copy, and the terminal event says so (`op_canceled`).
- Arch news guard: intervention items surface as a News-tab banner and per-item
  `[ACTION REQUIRED]` badges; the GUI does not gate the upgrade op on a live news
  fetch (divergence: the TUI prompts; the banner is the discoverable path).
- Links (news, package URLs) copy to the clipboard in AD; a real `tauri-plugin-opener`
  lands with AF packaging (capability addition).

## Testing strategy (cross-phase)

| Layer | Tooling | Guards |
|---|---|---|
| Core lib | existing 164 tests | frozen contract, untouched |
| Tauri commands | Rust unit tests in `gui/src-tauri` (8: DTO full-field map, stage-name mirror, fail-open parse + AD: failure-matrix classification, keyof wire tagged unions, name guard, makepkg-artifact glob) | this contract |
| Stream reducer | vitest (15: 5 search staged protocol + 10 ops) | staged protocol incl. superseded generations (mirrors the 4 TUI staged tests); ops: terminal-once semantics, ring cap, wrong-id guard, canceling copy |
| E2E smoke | Playwright dev-browser battery (`pnpm e2e`), 25 specs | search: empty → streaming → done → detail, keyboard-complete, generation guard, error envelope, no-results, overlay below 1020px; AD: tablist semantics, updates both-legs render, confirm → progress → done → dismiss, cancel → `op_canceled`, classified failure card, news intervention banner; axe scans: empty/done/detail/error/dialog + updates/news/confirm-dialog/failed-banner states; local gate in AC/AD (CI wiring pending container browser deps), revisit in AF |
| Accessibility | axe-core scans (`e2e/a11y.spec.ts`) over 5 key states — empty/done/detail/error/modal | zero violations (wcag 2.1 A/AA); violations fail the battery |

## CI integration

Path filter `gui/**` gates a `gui` job (archlinux container: webkit2gtk-4.1, librsvg,
node 22 + pnpm): frontend install/build (frozen lockfile), `cargo clippy -p pakmap-gui
--locked --all-targets -- -D warnings`, `cargo check -p pakmap-gui --locked`.
`cargo fmt --all` runs in the rust job and covers the GUI crate's Rust files. The
`ci-ok` aggregate job requires all triggered jobs green.
