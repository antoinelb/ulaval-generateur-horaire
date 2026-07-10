static:
	cargo fmt --all
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo +nightly llvm-cov
