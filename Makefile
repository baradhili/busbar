# BusBar dev environment — see AGENTS.md and Design/esld-implementation.md
.PHONY: build test cucumber fmt fmt-check lint doc wasm clean

build:
	cargo build --workspace

test:
	cargo test --workspace

cucumber:
	cargo test --test cucumber

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

doc:
	cargo doc --workspace --no-deps

wasm:
	cargo build -p busbar-wasm --target wasm32-unknown-unknown

clean:
	cargo clean

# Phase 1 acceptance suites: intentionally red until M1/M3 land.
.PHONY: cucumber-phase1
cucumber-phase1:
	BUSBAR_PHASE1=1 cargo test --test cucumber
