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

### Shipped in AB (spike surface)

| Command | Signature | Returns |
|---|---|---|
| `greet` | `{ name: string }` | `string` (IPC liveness probe) |
| `get_config_summary` | — | `ConfigSummary` |
| `search_once` | `{ query: string }` | `PackageDto[]` \ `GuiError` (repo leg only, limit 8, no network) |

`ConfigSummary` = `{ app_version, limit, source, timeout_secs, flatpak_enabled,
custom_aur_rpc }` — resolved-effective config (user file, fail-open defaults).

### Planned command surface (AC/AD)

| Command | Request | Response | Phase |
|---|---|---|---|
| `search` | `{ query, sources, aur_by, regex, installed_only, orphans_only, limit }` | kicks off staged stream (events below) | AC |
| `cancel_search` | `{ generation }` | ok | AC |
| `updates` | — | repo + AUR update lists | AC (lite badges) / AD |
| `list_installed` | `{ query, limit }` | installed packages (localdb fast path) | AC |
| `package_info` | `{ name, source }` | detail record (files/deps/votes) | AC |
| `stats` | — | status-bar counters | AE |
| `install` / `remove` / `upgrade` / `clean` | op request | op handle; progress via events | AD |
| `cancel_operation` | `{ op_id }` | ok (between phases only) | AD |

### Streaming events (one channel per operation)

Search mirrors the TUI's `SearchStage::{Repo, Aur, Flatpak, Done}` and reuses the same
`active_search` generation guard — superseded searches drop out incrementally, first leg
paints immediately:

```jsonc
// search://stage
{ "generation": 7, "stage": "repo",  "packages": [PackageDto…] }  // then "aur", "flatpak"
{ "generation": 7, "stage": "done",  "packages": [] }             // leg complete
```

Operations (AD) emit progress from the existing `process.rs`/`timing.rs` plumbing
(`indicatif` stays CLI-only):

```jsonc
// op://progress  { "op_id": 12, "phase": "install", "percent": 42, "line": "…" }
// op://done      { "op_id": 12, "ok": true, "exit_code": 0 }
```

### Error envelope

All command errors reject with `{ code, message, hint? }` (`GuiError`). Strings are
mapped from the CLI's `error.rs` hints — one source of truth, no GUI-specific copies.
Seeded codes (AB): `empty_query`, `repo_search_failed`; AC adds `aur_search_failed`,
`flatpak_search_failed`, `config_unreadable`; AD adds the privilege/ops matrix
(`pkexec_missing`, `auth_dismissed`, `db_locked`, `flatpak_missing`, …) — each maps to
an actionable error card with the CLI's hint text.

### Frontend DTO

`PackageDto` (AB): `{ name, version, description, repo, installed }` (`From<&Package>`);
AC extends with votes/maintainer/out-of-date/flatpak app id as the detail pane needs.
camelCase? — no: Tauri serializes field names as-is (snake_case), the TS types mirror
the Rust structs 1:1.

### Privilege & cancellation model

- GUI process stays unprivileged; pacman ops spawn `pkexec pacman` (polkit owns the
  auth UI); flatpak ops run user-scope. Missing pkexec ⇒ actionable error, never a
  root GUI.
- Cancellation: searches cancelable anytime (generation guard makes it free); pacman
  transactions never killed mid-transaction — op cancel applies between phases only
  (surfaced in UI copy, AD).

## Testing strategy (cross-phase)

| Layer | Tooling | Guards |
|---|---|---|
| Core lib | existing 166 tests | frozen contract, untouched |
| Tauri commands | Rust unit tests in `gui/src-tauri` (fixtures + `PAKMAP_SKIP_LIVE`) | this contract |
| Stream reducer + components | vitest + Testing Library | staged protocol incl. superseded generations (mirrors the 4 TUI staged tests) |
| E2E smoke | Playwright against the dev server | search → results render |
| Accessibility | axe-core in CI on key routes | no new violations |

## CI integration

Path filter `gui/**` gates a `gui` job (archlinux container: webkit2gtk-4.1, librsvg,
node 22 + pnpm): frontend install/build (frozen lockfile), `cargo clippy -p pakmap-gui
--locked --all-targets -- -D warnings`, `cargo check -p pakmap-gui --locked`.
`cargo fmt --all` runs in the rust job and covers the GUI crate's Rust files. The
`ci-ok` aggregate job requires all triggered jobs green.
