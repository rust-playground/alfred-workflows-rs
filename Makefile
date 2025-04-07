lint:
	cargo fmt --all -- --check

test:
	cargo test

build:
	cargo build

release:
	cargo build --release --target aarch64-apple-darwin
	cargo build --release --target x86_64-apple-darwin

.PHONY: lint test build release