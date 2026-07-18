# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A course-schedule generator / study-path planner for Université Laval, commissioned by the director of the *baccalauréat en génie des eaux* (GEX) as a paid mandate.
The whole domain is French: keep domain vocabulary in French (`cours`, `cheminement`, `préalables`, `matière`, `session`, `jalon`, `pavillon`) in prose, documentation, and user-facing UI copy.

**Code is English.** All identifiers (variables, functions, types), error messages, and data keys (JSON) are in English — as the test fixtures already do: `title`/`credits`/`prerequisites`/`mandatory`/`rules`, never `titre`/`crédits`/`préalables`/`obligatoires`/`regles`. French domain terms belong in prose and displayed text, not in code or serialized keys.

**Current state: pre-implementation.**
No source code yet — only planning documents and parser test cases.

## Documents

- `docs/project_plan.md` — **the standalone source of truth**: functionality, constraints, implementation, weekly jalons, revised decisions, open questions. Read it before writing code.
- `docs/next_steps.md` — the immediate task list (test-first scraper/parser).
- `docs/conception/` — the design history (original conception documents, deliverable plan, initial ADR, request emails). Consult only for extra detail (full grammar specs, worked JSON examples, spike results, rejected-alternative reasoning); where it contradicts `docs/project_plan.md`, the plan wins.
- `tests/fixtures/test_cases/` — expected parser outputs: `classes/*.json` (course pages) and `programs/*.json` (program pages). Shared across crates; see ADR `2026-07-structure-des-tests-et-fixtures`.

## Decision records (ADR) — required practice

Every decision taken from now on is documented in its own file under `docs/conception/adr/`:

- One decision per file, kebab-case name (e.g. `2026-07-throttle-10-req-s.md`).
- Keep it short: context, the decision, alternatives rejected and why.
- When a decision changes the plan, update `docs/project_plan.md` *and* add the ADR — the plan carries the **what**, the ADR preserves the **why**.
- When you (Claude) participate in a decision with the user, write the ADR as part of the same change; do not let decisions live only in conversation.

## Stack and architecture (decided)

Rust throughout, one Cargo workspace — details and reasoning in `docs/project_plan.md`:

- **`core`** (library) — all domain logic, zero IO/async; compiles to native (scraper, tests) and WASM (UI).
- **`scraper`** (native async binary) — fetch + parse ULaval pages into JSON snapshots; fetching and parsing strictly separated, parser tested against frozen HTML fixtures.
- **`ui`** (WASM binary) — Dioxus 0.7, client-side rendered. **Whenever Dioxus code is written or understood, first read `.claude/dioxus.md`** (Dioxus 0.7 API reference): 0.7 changed every API — `cx`, `Scope`, and `use_state` are gone; use `use_signal`, `#[component]`, `rsx!`, `Routable`, `use_resource`.

To run tests with coverage, use `make test` (wraps `cargo +nightly llvm-cov`).

Fully static, serverless: no backend, no database; snapshots produced by a CI cron job (never in-app scraping); the solver runs in the browser; user state in `localStorage`; schedule sharing via URL.
Load-bearing invariants (constraints, not preferences) are in `docs/project_plan.md` § Contraintes — notably: **all business logic in the pure `core` crate, none in the view**; **never drop unrecognized input silently**; **atomic snapshot replacement**.

## Domain quick facts

- **Session naming & founding hypothesis**: files are named season+year (`a2026` = Automne 2026, `h####` = Hiver, `e####` = Été). A future session with no published schedule reuses the most recent snapshot of the *same season* — so keep one snapshot per season, never blindly overwritten.
- **Data files**: `data/cours/{session}.json` (per-course: `code`, `title`, `credits`, `cycle`, subject, `prerequisites` raw + parsed tree, contributing programs, `equivalents`, sections with NRC/type/slots/building/linked sections) and `data/programmes.json` (`credits_required`, `mandatory`, `rules`, `concentrations`, `profiles`, plus the hand-encoded `cheminement_type`).
- **`matière` = course-code prefix** (`GCI-`, `GEX-`); filtering by subject filters the catalogue URLs, no facet needed.
- **Program mapping comes from course pages** ("Cette activité est contributoire dans :"), not program pages; only programs whose rules are needed get their page scraped.
- **`cheminement_type` (A1→H8 organigramme) is hand-encoded**, GEX only — no machine-readable source exists.
- **Two parser grammars** (full specs in `docs/conception/`): program rules ("Règle N – \<contrainte\> parmi :" → `{type: course|credits, …}` with `subgroups`) and préalables (ET/OU source text → trees keyed `all`/`any` + `program_credits`). Anything outside a grammar is kept as raw text (`{"raw": "…"}`) and surfaced, never ignored.
- **Scraping**: pages are server-rendered, plain GET, no headless browser (2026-07-02 spike); ~10 000 requests for the full catalogue, throttled to ~10 requests/second (~20 min); resume on error; existing snapshots keep being served until the atomic `rename`.

## Milestones (jalons)

Ten weekly demonstrable jalons (`docs/project_plan.md` § Versions et jalons hebdomadaires), ~10 h/week ≈ one per week, grouped into three end-to-end usable versions (ADR `2026-07-decoupage-en-versions-v0-v1-v2`):
- **v0 (MVP)**, weeks 1–3: enter course codes for a session → schedule auto-built, obvious conflicts highlighted, add/remove courses, credit total shown.
- **v1**, weeks 4–6: pick courses from a list (search, filters, full catalogue + CI cron), program courses presented by rules and profils.
- **v2**, weeks 7–10: the full bac — sessions fill automatically (organigramme, rules coverage, generation under constraints) and stay editable; preferences + URL sharing as final polish.

## Other instructions

- When writing comments, don't prefix them with `ponytail: `
- Don't hesitate to delegate to a cheaper model when it makes sense
- Never use while loops
