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

# Incomplete-area scenarios (@incomplete): intentionally red until their
# milestones land. Opt-in only — never part of the CI gate.
.PHONY: cucumber-incomplete
cucumber-incomplete:
	BUSBAR_INCOMPLETE=1 cargo test --test cucumber
