.PHONY: static test wasm docs ui ui-data ui-calc

static: ui-data ui-calc
	cargo fmt --all
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	# the JS boundary exists only under wasm32: nothing above ever sees it
	cargo clippy -p ulaval-scheduler-wasm \
		--target wasm32-unknown-unknown -- -D warnings
	# the browser glue of the ui and its worker is wasm32-only too
	cargo clippy -p ulaval-scheduler-ui \
		--target wasm32-unknown-unknown -- -D warnings
	cargo clippy -p ulaval-scheduler-ui-calculations \
		--target wasm32-unknown-unknown -- -D warnings

test: ui-data ui-calc
	cargo +nightly llvm-cov --ignore-filename-regex \
		'(lib\.rs|/mod\.rs|/main\.rs)$$|crates/ui/src/components/'

wasm:
	wasm-pack build crates/wasm --target web

# dx serve only serves asset!() files: the snapshots must live under the
# crate's assets/ (ADR 2026-07-donnees-servies-en-assets-du-harnais)
ui-data:
	mkdir -p crates/ui/assets/data/programmes
	cp data/cours.json crates/ui/assets/data/cours.json
	# meta.json appears at the next scrape; until then an explicit unknown
	# (never a guessed date — ADR 2026-08-meta-json-provenance-du-snapshot)
	if [ -f data/meta.json ]; then \
		cp data/meta.json crates/ui/assets/data/meta.json; \
	else \
		echo '{"scraped_at":null}' > crates/ui/assets/data/meta.json; \
	fi
	cp data/programmes/*.json crates/ui/assets/data/programmes/

# the worker's wasm module, dropped into the ui's assets so dx serves it —
# the ui's asset!() needs the files at every build, hence the make dep
crates/ui/assets/calc/calc.js: crates/ui-calculations/Cargo.toml \
		$(wildcard crates/ui-calculations/src/*.rs)
	wasm-pack build crates/ui-calculations --target web --no-typescript \
		--out-dir ../ui/assets/calc --out-name calc

ui-calc: crates/ui/assets/calc/calc.js

ui: ui-data ui-calc
	BUILD_HASH=$$(git rev-parse --short HEAD) \
		dx serve --package ulaval-scheduler-ui --port 8000

docs:
	mdbook build docs/livre
