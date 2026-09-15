.PHONY: setup wasm dev server web test determinism build serve

setup: wasm
	cd web && npm install

# The tray, compiled for the browser into web/public/ebru.wasm.
wasm:
	rustup target add wasm32-unknown-unknown
	cd web && npm run wasm

server:
	cargo run --release -p ebru-server

web:
	cd web && npm run dev

# Both, stopped together with Ctrl+C.
dev:
	@echo "server  http://localhost:8740"
	@echo "web     http://localhost:5174"
	@$(MAKE) -j2 server web

test:
	cargo test --workspace --release
	cargo clippy --workspace --all-targets --release -- -D warnings
	cd web && npm run check

# Native and WebAssembly trays replay a recorded print bit for bit.
determinism: wasm
	cd web && node scripts/determinism.mjs

build: wasm
	cd web && npm run build
	cargo build --release -p ebru-server

# One process: the server hosting the built workshop.
serve: build
	EBRU_WEB="$(CURDIR)/web/dist" ./target/release/ebru
