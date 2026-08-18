.PHONY: static lint test wasm docs ui ui-build ui-data ui-calc

static:
	cargo fmt --all
	$(MAKE) lint

# the lint set, defined once: `make static` formats then calls it, the CI
# only checks the formatting and calls it too — neither can drift from the
# other (ADR 2026-08-makefile-definition-unique-de-la-ci)
lint: ui-data ui-calc
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	# the browser boundary exists only under wasm32: nothing above ever
	# sees it, and one crate now carries both surfaces
	cargo clippy -p ulaval-scheduler-wasm \
		--target wasm32-unknown-unknown -- -D warnings
	# the browser glue of the ui is wasm32-only too
	cargo clippy -p ulaval-scheduler-ui \
		--target wasm32-unknown-unknown -- -D warnings

# the threshold is the CI's: a local run must fail exactly where the push
# would (build.rs is generated plumbing, not testable code)
test: ui-data ui-calc
	cargo +nightly llvm-cov --ignore-filename-regex \
		'(lib\.rs|/mod\.rs|/main\.rs)$$|crates/ui/src/components/|build\.rs$$' \
		--fail-under-lines 100

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
# the ui's asset!() needs the files at every build, hence the make dep. It is
# the very package `make wasm` publishes: one crate, two surfaces (ADR
# 2026-08-fusion-des-crates-wasm-et-ui-calculations)
crates/ui/assets/calc/calc.js: crates/wasm/Cargo.toml \
		$(wildcard crates/wasm/src/*.rs)
	wasm-pack build crates/wasm --target web --no-typescript \
		--out-dir ../ui/assets/calc --out-name calc

ui-calc: crates/ui/assets/calc/calc.js

ui: ui-data ui-calc
	BUILD_HASH=$$(git rev-parse --short HEAD) \
		dx serve --package ulaval-scheduler-ui --port 8000

# the production bundle: Pages serves the project site under a sub-path, and
# `asset!()` emits absolute URLs, so the base path has to be baked in. dx
# nests the web bundle under `public/`: _ui/public is the site root, and the
# service worker sits beside the index there — its scope is the directory it
# is served from (ADR 2026-08-interface-publiee-a-la-racine-de-pages)
ui-build: ui-data ui-calc
	BUILD_HASH=$$(git rev-parse --short HEAD) \
		dx bundle --release --platform web \
			--package ulaval-scheduler-ui \
			--base-path ulaval-generateur-horaire --out-dir _ui
	cp crates/ui/assets/sw.js _ui/public/sw.js

docs:
	mdbook build docs/livre
