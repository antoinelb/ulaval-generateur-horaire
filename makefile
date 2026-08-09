.PHONY: static test wasm docs

static:
	cargo fmt --all
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	# the JS boundary exists only under wasm32: nothing above ever sees it
	cargo clippy -p ulaval-scheduler-wasm \
		--target wasm32-unknown-unknown -- -D warnings

test:
	cargo +nightly llvm-cov --ignore-filename-regex '(lib\.rs|/mod\.rs|/main\.rs)$$'

wasm:
	wasm-pack build crates/wasm --target web

docs:
	mdbook build docs/livre
