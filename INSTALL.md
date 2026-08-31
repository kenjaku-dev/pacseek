# Quick Install (ez & butter)

## 1. One-liner build & install
```bash
cd /home/achraf/Projects/pacseek
make install        # or: cargo install --path . --force
pacseek --help
```

## 2. Sync pacman DB first (required for repo results)
```bash
sudo pacman -Sy
```

## 3. Try it
```bash
pacseek firefox --limit 5
pacseek neovim --json | jq
pacseek yay --source aur --by name
pacseek "linux.*headers" --regex
pacseek rust --installed-only
```

## 4. Release binary
```bash
make release
ls -lh target/release/pacseek   # ~2-3 MB stripped
sudo install -Dm755 target/release/pacseek /usr/local/bin/pacseek
```

## Shell completions (optional butter)
```bash
# bash
pacseek --help | head   # helps discovery
# completions via clap_complete if you enable feature later:
# cargo run -- --generate-completion bash > /usr/share/bash-completion/completions/pacseek
```

## Update
```bash
cd /home/achraf/Projects/pacseek
git pull
cargo install --path . --force
```
