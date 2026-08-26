.PHONY: static lint test wasm docs ui ui-build ui-data ui-calc

static:
	rm -f *.profraw
	cargo fmt --all
	$(MAKE) lint

lint: ui-data ui-calc
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo clippy -p ulaval-scheduler-wasm \
		--target wasm32-unknown-unknown -- -D warnings
	cargo clippy -p ulaval-scheduler-ui \
		--target wasm32-unknown-unknown -- -D warnings

test: ui-data ui-calc
	cargo +nightly llvm-cov --ignore-filename-regex \
		'(lib\.rs|/mod\.rs|/main\.rs)$$|crates/ui/src/components/|build\.rs$$' \
		--fail-under-lines 100

wasm:
	wasm-pack build crates/wasm --target web

ui-data:
	rm -rf crates/ui/assets/data/programmes
	mkdir -p crates/ui/assets/data/programmes
	cp data/cours.json crates/ui/assets/data/cours.json
	cp data/cours.manuel.json crates/ui/assets/data/cours.manuel.json
	if [ -f data/meta.json ]; then \
		cp data/meta.json crates/ui/assets/data/meta.json; \
	else \
		echo '{"scraped_at":null}' > crates/ui/assets/data/meta.json; \
	fi
	cp data/programmes/*.json crates/ui/assets/data/programmes/

crates/ui/assets/calc/calc.js: crates/wasm/Cargo.toml \
		$(wildcard crates/wasm/src/*.rs) \
		$(wildcard crates/core/src/*.rs)
	wasm-pack build crates/wasm --target web --no-typescript \
		--out-dir ../ui/assets/calc --out-name calc

ui-calc: crates/ui/assets/calc/calc.js

ui: ui-data ui-calc
	BUILD_HASH=$$(git rev-parse --short HEAD) \
	DATA_HASH=$${DATA_HASH:-$$(git log -1 --format=%h -- data/)} \
		dx serve --package ulaval-scheduler-ui --port 8000

ui-build: ui-data ui-calc
	BUILD_HASH=$$(git rev-parse --short HEAD) \
	DATA_HASH=$${DATA_HASH:-$$(git log -1 --format=%h -- data/)} \
		dx bundle --release --platform web \
			--package ulaval-scheduler-ui \
			--base-path ulaval-generateur-horaire --out-dir _ui
	cp crates/ui/assets/sw.js _ui/public/sw.js

docs:
	mdbook build docs/livre
