# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A course-schedule generator / study-path planner for Université Laval, commissioned by the director of the *baccalauréat en génie des eaux* (GEX) as a paid mandate.
Current state: first delivered version; only preferences (jalon 10) remain.

- **Domain is French**: keep domain vocabulary in French (`cours`, `cheminement`, `préalables`, `session`) in prose, docs, and UI copy.
- **Code is English**: identifiers, error messages, and JSON keys (`title`/`credits`/`prerequisites`, never `titre`/`crédits`/`préalables`).

## Source of truth

`docs/conception/adr/` — one decision per file. **Read the ADRs relevant to an area before changing it**; where anything else contradicts an ADR, the ADR wins.

Every new decision gets its own ADR (kebab-case, short: context, decision, alternatives rejected). When you participate in a decision with the user, write the ADR as part of the same change — never leave decisions in conversation only.

Other docs: `docs/conception/` (history, grammar specs — background only), `docs/livre/` (the published mdBook, French, `make docs`), `tests/fixtures/test_cases/` (expected parser/solver outputs, shared across crates).

## Architecture

Rust throughout, one Cargo workspace. Fully static, serverless: no backend, snapshots produced by a CI cron job (never in-app scraping), solver runs in the browser, user state in `localStorage`, sharing via URL.

- **`core`** — all domain logic, zero IO/async; compiles native and WASM. The ULaval parser lives here behind the default-on `parser` feature.
- **`scraper`** — native async binary; fetching and parsing strictly separated, parser tested against frozen HTML fixtures.
- **`wasm`** — the frontier crate: one orchestration over `core`. The live surface is the Dioxus worker's (`init_snapshot` + `handle_message`, JSON strings); the eight-function JavaScript surface is **frozen** and never a design constraint. Pure Rust functions tested natively; `#[wasm_bindgen]` glue in `boundary.rs` only.
- **`ui`** — Dioxus 0.7 WASM binary. **Before writing or reading Dioxus code, read `.claude/dioxus.md`** — 0.7 changed every API. Scenario tests in `crates/ui/tests/scenarios/` replay the UI's orchestration natively.

Load-bearing invariants (constraints, not preferences):

- **All business logic in the pure `core` crate, none in the view.**
- **Never drop unrecognized input silently** — anything outside a grammar is kept raw and surfaced.
- **Atomic snapshot replacement** (write to tmp, then rename).

## Commands

- `make static` — lint (includes the wasm target); treat warnings as errors.
- `make test` — tests with coverage, fails under 100 %. If it reports <100 % on a fully-tested file, it's the double-compilation effect: see ADR `2026-07-couverture-par-instanciation-le-plus-petit-ecart`.
- `make e2e` — Playwright browser suite (specs in French, `tests/e2e/`), the only executable specification of the interface. Serves `_ui/public`, never `dx serve`.
- `make ui` / `make ui-build` — serve / build the deployed site (`_ui/public`).
- `make wasm` / `make ui-calc` — npm package / same crate into the ui's assets (need `wasm-pack`).
- The makefile is the single definition of what CI verifies; `ci.yml` only calls make targets.

## Domain quick facts

- Sessions are season+year (`a2026`, `h####`, `e####`). A future session with no published schedule reuses the most recent offering of the same season, per course.
- Data: `data/cours.json` (every course, sorted by code; `options: null` = offered but schedule unpublished, distinct from `[]`), `data/cours.manuel.json` (hand-maintained, scraper never writes it), `data/programmes/{code}-{semester}.json` (one file per program and semester vintage; `code` is the official répertoire code, e.g. `B-GEX`).
- Scope authority is the cycle read on the page, not the `8xxx` URL filter; préuniversitaire `0xxx` courses are in scope.
- Program mapping comes from course pages (« contributoire dans »), not program pages.
- Two parser grammars (specs in `docs/conception/`): program rules and préalables. A grammar change reaches `data/cours.json` via `ulaval-scraper reparse` — no network, no re-scrape.
- Scraping: server-rendered pages, plain GET, ~10 req/s throttle, resume on error.

## Working conventions

- Claude writes all the code — production, tests, fixtures, ADRs — and runs the verification loop; Antoine directs and reviews.
- Never use while loops; avoid `expect` in production code.
- Don't prefix comments with `ponytail: `.
- Don't hesitate to delegate to a cheaper model when it makes sense.
- Use `agent-browser` instead of Claude-in-Chrome.
- `../grille-de-cheminement-interactive` is **not** a compatibility target.
