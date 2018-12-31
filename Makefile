lint:
	cargo fmt --all -- --check

test:
	cargo test

build:
	cargo build

release:
	cargo build --release

.PHONY: lint test build release