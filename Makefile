SERVE_PORT ?= 8080

.PHONY: serve check test build-native build-wasm shots ci clean

serve: ## build + serve with auto-reload, open in browser
	cd crates/game-app && trunk serve --port $(SERVE_PORT) --open

check: ## fmt + clippy + unit tests + native debug build (run before every commit)
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	cargo build -p game-app

test: ## unit tests only
	cargo test --workspace

build-native: ## native release binary
	cargo build --release -p game-app

build-wasm: ## wasm release bundle via trunk
	cd crates/game-app && trunk build --release

shots: ## Playwright screenshots against a running server (make serve)
	cd tools/screenshots && node shot.mjs --url http://localhost:$(SERVE_PORT)

ci: ## everything CI runs
	tools/ci.sh

clean:
	cargo clean
	rm -rf crates/game-app/dist
