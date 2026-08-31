# pacseek

Fast **AUR + Official Repo** search for Arch / Artix Linux — written in Rust.

One binary to search both:
- **Official repos** via `libalpm` (reads `/var/lib/pacman/sync/*.db`, no network)
- **AUR** via `https://aur.archlinux.org/rpc/v5` (async `reqwest`)

Inspired by `paru` / `yay` but focused on **search only** (simple, safe, fast).

## Features
- Unified search `pacseek <query>` (repo first, AUR second)
- `--source aur|repo|all` (default `all`)
- `--by name|name-desc|maintainer|depends...` for AUR
- `--limit N`, `--regex`, `--installed-only`, `--bottom-up` (AUR first), `--json`, `--no-color`
- Colored output, votes/popularity for AUR, `[installed]` marker for repo
- Fallback to `pacman -Ss` if `libalpm` fails
- Tokio async (AUR + repo in parallel), tracing logs with `-v/-vv`

## Install

### Build from source
```bash
git clone <repo>
cd pacseek
cargo build --release
sudo install -Dm755 target/release/pacseek /usr/local/bin/pacseek
```

Requirements: `pacman` 5.1+ (`libalpm 16`), `pacman-contrib` for `pacman-conf` binary, `rust 1.85+`.

### Artix/Arch quick
```bash
cargo install --path .
# then sync DB
sudo pacman -Sy
```

## Usage
```bash
pacseek firefox
pacseek firefox --limit 10 --source aur
pacseek neovim --by maintainer --limit 20
pacseek "linux.*headers" --regex
pacseek code --installed-only
pacseek rust --json | jq
pacseek firefox --bottom-up   # AUR first
pacseek --help
```

## Project Layout
```
src/
  main.rs        # thin entry, clap -> tokio
  lib.rs
  cli.rs         # clap derive 4.5
  model.rs       # unified Package struct
  search/
    aur.rs       # AUR RPC v5
    repo.rs      # libalpm via alpm.rs 5.0 + pacmanconf
  output.rs      # colored + json
  error.rs
```

## Best Practices Used
- `2024 edition`, `clap derive` + `ValueEnum`, `anyhow` + `thiserror`, `tokio` full, `serde` JSON
- `alpm` crate (safe bindings) + `pacmanconf` parsing `/etc/pacman.conf`
- `reqwest` rustls, `tracing` env-filter, `colored`, `regex`
- Release profile `opt-level="z" lto=true strip=true` (~small binary)
- `cargo fmt`, `cargo clippy`, `cargo audit` ready

## Troubleshooting
- **No repo results**: run `sudo pacman -Sy` to refresh sync DBs (`/var/lib/pacman/sync`)
- **alpm init failed**: fallback uses `pacman -Ss`, check `pacman-conf` exists
- **AUR timeout**: network or AUR downtime; try again or `--source repo`

## License
MIT
