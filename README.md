# pacseek

Fast **AUR + Official Repo** search **+ TUI install/remove** for Arch / Artix — Rust, `ratatui` + `libalpm`.

One binary for both `pacman -Ss` and AUR `RPC v5`, with your drawing's terminal: `firefox` bar → results → `Enter` install / `i` info — plus `Tab` remover mode.

```
 🔍 Search (install) │ 🗑 Installed (remove)
┌ Search (Enter to search, Tab remover) ──────┐
│ firefox█                                    │
└─────────────────────────────────────────────┘
┌ Results (1/10) ─────────────────────────────┐
│▸ extra/firefox 123.0-1                       │
│    Fast browser                             │
│  aur/firefox-bin 123.0-1 (+1200 5.2)        │
│    Binary Firefox                           │
└─────────────────────────────────────────────┘
 Found 10 (repo 5 aur 5)  Tab:remover ?:help
```

Built with **best skills**: `ratatui 0.30` (tui-design ecosystem-rust, immediate-mode, Layout cache) + `tui-design` (alternate screen, pane fallback, clutter audit, `NO_COLOR`) + `aur-guides` (RPC, makepkg, audit, never root).

## Features
- **One-shot CLI:** `pacseek <query>` repo first, aur after — parallel tokio, `libalpm` local DB, fallback `pacman -Ss`
- **TUI:** `pacseek --tui` or `pacseek` (no args, TTY) → tabs `Search` / `Installed` (`Tab` to switch) → search bar + virtualized `List` → `i` info / `?` help popup → `Enter` confirm → `sudo pacman -S` (repo) or `git clone + makepkg -si` (AUR, PKGBUILD preview, `namcap` if present, `~/.cache/pacseek`)
 - **Remover:** `Tab` → Installed mode (empty filter lists all, type to filter) → `Enter`/`d`/`x` remove → confirm popup (`sudo pacman -Rs`, `pacman -Qi` preview) → `pacseek --remove <pkg>` for scripts
 - Filters: `--source aur|repo|all`, `--by name|name-desc|maintainer...`, `--limit N`, `--regex`, `--installed-only`, `--bottom-up`, `--json`, `--no-color`, `--no-tui`, `--remove PKG`
 - **Config:** `~/.config/pacseek/config.toml` (or `./pacseek.toml` project-local) — edit colors, borders, layout, timeouts without recompile — `pacseek --init-config` to generate, `--show-config` to locate, `--config PATH` to override
 - Safety: `aur-guides` — HTTPS sources, `makepkg` never as root via `nix::geteuid`, `PKGBUILD` shown, `tui-design` lifecycle `try_restore/try_init` + panic hook `color_eyre` before `ratatui::init`

## Install
```bash
cd /home/achraf/Projects/pacseek
cargo build --release  # 4.4M stripped, opt-level=z lto
sudo install -Dm755 target/release/pacseek /usr/local/bin/pacseek
# or
cargo install --path . --force  # -> ~/.cargo/bin/pacseek (add to PATH)
sudo pacman -Sy  # refresh sync DB before first repo search
```

Deps: `pacman 5.1+ (libalpm 16)`, `pacman-contrib` (`pacman-conf`), `git`, `base-devel` for AUR builds, `rust 1.85+` (MSRV 1.88 for ratatui 0.30).

## Usage

**CLI (automation-friendly, no TUI):**
```bash
pacseek firefox --no-tui
pacseek firefox --limit 10 --source aur
pacseek neovim --by maintainer --limit 20
pacseek "linux.*headers" --regex
pacseek code --installed-only
pacseek rust --json | jq '.[].name'
pacseek firefox --bottom-up   # AUR first
```

**TUI (your drawing):**
```bash
pacseek --tui               # empty search, type firefox → Enter
pacseek --tui firefox       # prefilled query, immediate search
pacseek                     # no args + TTY => TUI (like original pacseek)
# Inside TUI: type query, Enter search, ↑↓/j k navigate, Enter install (confirm Y), i info, / search, Tab remover, ? help, q quit
# Tab → Installed mode: type to filter, Enter/d/x remove (confirm Y), Tab back
# Install needs sudo password; AUR shows PKGBUILD + namcap then makepkg -si; remove runs sudo pacman -Rs
# CLI shortcut: pacseek --remove <pkg>
```

**Config (ez edit):**
```bash
pacseek --init-config          # creates ~/.config/pacseek/config.toml
pacseek --show-config          # prints paths (XDG + project-local if exists)
cat ~/.config/pacseek/config.toml
# edit: colors, borders, layout, timeouts — then restart pacseek
# [theme] border_focused="cyan" repo_aur="magenta bold" no_color=false tab_selected="black on cyan bold"
# [tui] floor_width=60 floor_height=14 show_tabs=true border="rounded" highlight_symbol="▸ " debounce_ms=400
# [search] limit=50 source="all" timeout_secs=15
# [behavior] makepkg_noconfirm=false remove_flags="Rs" remove_noconfirm=false cache_dir="/tmp/pacseek"
# Project-local override: ./pacseek.toml (same format) wins over XDG
pacseek --config ./my.toml firefox --no-tui
```

**Non-TTY / scripts:** `pacseek` auto-detects TTY; `echo | pacseek --tui` gives friendly `requires a terminal`. Use `--json` or `--no-tui` for pipes.

## Project Layout

```
src/
  main.rs        # cli + tui branch, IsTerminal, tokio, Config::load + CLI merge
  lib.rs         # re-exports
  cli.rs         # clap 4.5 derive, Option query, --tui/--no-tui, --config/--init-config/--show-config
  config.rs      # XDG ~/.config/pacseek/config.toml + ./pacseek.toml, TOML, theme/layout/behavior (serde)
  model.rs       # Package (PartialEq), unified repo/aur
  search/
    aur.rs       # RPC v5 async + blocking (TUI), sorts popularity, config aur_rpc + timeout
    repo.rs      # alpm 5 + pacmanconf, fallback pacman -Ss
  output.rs      # colored + json (CLI)
  tui/
    mod.rs       # run() try_init/try_restore, run_with_config, suspend_and_run handoff + flush/drain
    app.rs       # Input (tui-input), ListState, Config stored, debounced/poll_ms from config, non-blocking mpsc
    ui.rs        # Layout vertical configurable, border_type + colors from theme, popups %, Clear hole-punch
  install/
    repo.rs      # sudo pacman -S --needed (aur-helpers)
    aur.rs       # git clone/pull, PKGBUILD bat/cat, makepkg -si, cache_dir + makepkg_noconfirm from config
  error.rs
  config.example.toml  # ship example
```

## Best Practices (skills)

- `ratatui` 0.30: `ratatui::try_init` before `color_eyre`, `Block::bordered().title`, `ListState` stateful, `TestBackend` snapshots at 80×24/60 cols, `unicode_width` via wrap, `Constraint::Min` responsive, `Clear` for popups
- `tui-design`: alternate screen, non-blocking disk/net (search via mpsc+thread parallel), `NO_COLOR` honor, `too-small` 60×13 fallback (3+8+1+1), clutter audit (<1 border, Spacing::Overlap), keyboard-reachable (`j/k`, `/`, `Enter`, `i`, `q`, `Ctrl+C`), suspend/resume with `clear`+`flush`+drain
- `aur-guides`: ` Aurweb RPC` `https://aur.archlinux.org/rpc/v5/search/{q}?by=`, `makepkg -s`, `namcap`, `.SRCINFO`, HTTPS sources
- Release `opt-level="z" lto codegen-units=1 strip`, `cargo fmt/clippy/audit`

## Troubleshooting
- **No repo results:** `sudo pacman -Sy` (`/var/lib/pacman/sync/*.db`)
- **alpm init failed:** fallback `pacman -Ss`; check `pacman-conf` exists
- **AUR timeout:** retry or `--source repo`; TUI shows `No results`
- **TUI not starting:** `pacseek --tui` needs TTY; in headless use `--no-tui`; panic restores via `color_eyre` hook
- **AUR install as root:** blocked — run as user, `sudo` for pacman only

## Skills used
- `~/.config/opencode/skill/ratatui/SKILL.md` (mte90, 0.30.1)
- `~/.config/opencode/skill/tui-design/` + `references/ecosystem-rust.md`
- `~/.config/opencode/skill/aur-guides/` (`aur-rpc`, `aur-makepkg`)

## License
MIT
