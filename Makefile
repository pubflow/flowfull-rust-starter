.PHONY: fmt check test clippy doc verify run

fmt:
	cargo fmt --all -- --check

check:
	cargo check

test:
	cargo test

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --all-features --no-deps

verify: fmt check test clippy doc

run:
	cargo run
