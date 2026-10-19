SERVE_PORT ?= 8080

.PHONY: serve check test build-native build-wasm build-wasm-webgpu verify-web package package-from-dist browser-test shots ci clean

serve: ## build + serve with auto-reload (waits for the port, prints the URL)
	SERVE_PORT=$(SERVE_PORT) tools/serve.sh

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
	tools/setup-wasm-opt.sh
	cd crates/game-app && env -u NO_COLOR trunk build --release

verify-web: ## validate the production web bundle and Pages-compatible URLs
	tools/verify-web-dist.sh

build-wasm-webgpu: ## WebGPU-capable wasm bundle (falls back to WebGL2)
	tools/setup-wasm-opt.sh
	cd crates/game-app && env -u NO_COLOR trunk build --release --features webgpu --dist dist-webgpu

package: build-wasm package-from-dist ## shippable zip of the release wasm build

package-from-dist: verify-web ## package the already-verified production bundle
	rm -rf release && mkdir -p release
	cp -r crates/game-app/dist release/fun-to-drive-web
	cd release && zip -qr fun-to-drive-web.zip fun-to-drive-web
	@echo "release/fun-to-drive-web.zip ready"

browser-test: ## audible browser endurance test across the first music transition
	tools/browser-ci.sh

shots: ## Playwright screenshots against a running server (make serve)
	cd tools/screenshots && node shot.mjs --url http://localhost:$(SERVE_PORT)

ci: ## everything CI runs
	tools/ci.sh

clean:
	cargo clean
	rm -rf crates/game-app/dist
