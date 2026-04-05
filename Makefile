.PHONY: build test fmt fmt-check clippy run smoke

build:
	cargo build

test:
	cargo test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

clippy:
	cargo clippy --quiet --all-targets -- -D warnings

run:
	cargo run -- $(ARGS)

smoke:
	cargo run --quiet -- --dump-config >/dev/null
