# pacseek — butter Makefile
.PHONY: help build release install test fmt clippy clean run

help:  ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Debug build
	cargo build

release: ## Optimized release build (z + lto + strip)
	cargo build --release
	@ls -lh target/release/pacseek

install: release ## Install to ~/.cargo/bin and /usr/local/bin (if sudo)
	cargo install --path . --force
	@echo "Installed to ~/.cargo/bin/pacseek"
	@which pacseek && pacseek --version || true

test: ## Run all checks + live search smoke test
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test 2>&1 | head -n 50
	./target/debug/pacseek neovim --limit 2 --json | head -n 20
	./target/debug/pacseek firefox --source repo --limit 2

fmt: ## Format code
	cargo fmt

clippy: ## Lint
	cargo clippy

clean: ## Clean target
	cargo clean

run: ## Quick run, e.g. make run Q=firefox
	cargo run -- $(Q) --limit 5

# butter aliases
dev: fmt clippy build ## fmt+clippy+build in one go
update-db: ## Refresh pacman DB (needed for repo search)
	sudo pacman -Sy
