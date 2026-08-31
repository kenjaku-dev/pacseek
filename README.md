# pacseek

Fast **AUR + Official Repo** search **+ TUI install** for Arch / Artix — Rust, `ratatui` + `libalpm`.

One binary for both `pacman -Ss` and AUR `RPC v5`, with your drawing's terminal: `firefox` bar → results → `Enter` install / `i` info.

```
┌ Search (Enter to search) ─────────────────┐
│ firefox█                                  │
└───────────────────────────────────────────┘
┌ Results (1/10) ───────────────────────────┐
│▸ extra/firefox 123.0-1                     │
│    Fast browser                           │
│  aur/firefox-bin 123.0-1 (+1200 5.2)      │
│    Binary Firefox                         │
└───────────────────────────────────────────┘
 Found 10 (repo 5 aur 5)   ↑↓/j k  Enter:install  i:info  /:search q:quit
```

Built with **best skills**: `ratatui 0.30` (tui-design ecosystem-rust, immediate-mode, Layout cache) + `tui-design` (alternate screen, pane fallback, clutter audit, `NO_COLOR`) + `aur-guides` (RPC, makepkg, audit, never root).

## Features
- **One-shot CLI:** `pacseek <query>` repo first, aur after — parallel tokio, `libalpm` local DB, fallback `pacman -Ss`
- **TUI:** `pacseek --tui` or `pacseek` (no args, TTY) → search bar + virtualized `List` → `i` info popup → `Enter` confirm → `sudo pacman -S` (repo) or `git clone + makepkg -si` (AUR, PKGBUILD preview, `namcap` if present, `~/.cache/pacseek`)
- Filters: `--source aur|repo|all`, `--by name|name-desc|maintainer...`, `--limit N`, `--regex`, `--installed-only`, `--bottom-up`, `--json`, `--no-color`, `--no-tui`
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
# Inside TUI: type query, Enter search, ↑↓/j k navigate, Enter install (confirm Y), i info, / search, q quit
# Install needs sudo password; AUR shows PKGBUILD + namcap then makepkg -si
```

**Non-TTY / scripts:** `pacseek` auto-detects TTY; `echo | pacseek --tui` gives friendly `requires a terminal`. Use `--json` or `--no-tui` for pipes.

## Project Layout

```
src/
  main.rs        # cli + tui branch, IsTerminal, tokio
  lib.rs         # re-exports
  cli.rs         # clap 4.5 derive, Option query, --tui/--no-tui
  model.rs       # Package (PartialEq), unified repo/aur
  search/
    aur.rs       # RPC v5 async + blocking (TUI), sorts popularity
    repo.rs      # alpm 5 + pacmanconf, fallback pacman -Ss
  output.rs      # colored + json (CLI)
  tui/
    mod.rs       # run() try_init/try_restore, suspend_and_run handoff
    app.rs       # Input (tui-input), ListState, debounced 400ms, Mode popup
    ui.rs        # Layout vertical [3,Min(8),1,1], Clear hole-punch, 60×10 floor
  install/
    repo.rs      # sudo pacman -S --needed (aur-helpers)
    aur.rs       # git clone/pull, PKGBUILD bat/cat, makepkg -si (aur-makepkg, never root)
  error.rs
```

## Best Practices (skills)

- `ratatui` 0.30: `ratatui::try_init` before `color_eyre`, `Block::bordered().title`, `ListState` stateful, `TestBackend` snapshots at 80×24/60 cols, `unicode_width` via wrap, `Constraint::Min` responsive, `Clear` for popups
- `tui-design`: alternate screen, non-blocking disk/net (search via blocking threads), `NO_COLOR` honor, `too-small` 60×10 fallback, clutter audit (<1 border), keyboard-reachable (`j/k`, `/`, `Enter`, `i`, `q`, `Ctrl+C`), suspend/resume with `clear`
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
