# Plan

Test-first parser implementation — step 1 of the build order in `docs/project_plan.md`.
The expected outputs in `tests/fixtures/test_cases/` are done; this plan makes them pass.
Rhythm for each page type: freeze the real HTML → write the failing integration test → implement the parser until it matches the expected JSON → pin edge cases with inline unit tests.

- [x] Build e2e test cases for the parser
    - [x] All courses page (catalogue)
    - [x] Course page
    - [x] Program page
- [x] Freeze HTML fixtures
    - [x] Fetch (one-off `curl`, honest user agent) the real page behind each test case into `tests/fixtures/html/classes/*.html` and `tests/fixtures/html/programs/*.html`, same basename as the expected JSON
    - [x] Freeze the catalogue pages into `tests/fixtures/test_cases/catalogue/`: `gex_{0,1,2}.html` (50 courses, 2 courses, 0 courses — ADR `2026-07-catalogue-teste-sur-html-gele`) and `all_last.html` (« Aucun résultat » variant — ADR `2026-07-page-aucun-resultat-et-total-optionnel`)
    - Verify: every `test_cases/classes/*.json` and `test_cases/programs/*.json` has a same-named `.html` source, and the two catalogue pages are present
- [x] Parser skeleton in `scraper`
    - [x] Dependencies: `scraper` (HTML parsing), `thiserror` (library-side errors; `anyhow` stays at the binary frontier)
    - [x] Module layout: `parse/` with `catalogue.rs`, `prerequisites.rs`, `course.rs`, `program.rs`, and a shared error type that carries the offending raw text (an anomaly is data, never a panic)
    - Verify: `cargo check` passes with the empty module tree
- [x] Catalogue parser (`parse/catalogue.rs`)
    - [x] One page of HTML → `{code, title, url}` entries + `total_results: Option<usize>` (`None` on the « Aucun résultat » variant)
    - [x] Termination signal: 0 entries **with** proof of page shape (`total-resultats` element **or** texte « Aucun résultat ») = end of results; neither = markup drift = error (ADR `2026-07-page-aucun-resultat-et-total-optionnel`)
    - [x] Malformed entry → raw error line, never silently dropped
    - Verify: integration test parses the four frozen catalogue pages, merges, sorts and dedups, and compares with `test_cases/catalogue/gex.json`; unit tests on inline snippets pin what the frozen pages never exercise (0 entries with neither marker = drift, malformed entry)
- [ ] Fetch module, up to complete catalogues (`fetch.rs`)
    - [x] `Fetcher`: async client (honest user agent, 30 s timeout), shared throttle ~10 req/s (`fetch(&self)`, mutexed clock), bounded retries (3; transport, 5xx, 429), `Retry-After` honored (seconds + HTTP-date, 5 min cap, bumps the shared clock) — ADR `2026-07-conception-du-fetcher`
    - [x] Everything testable: pure `should_retry` / `parse_retry_after` unit-tested, `wait_for_slot` on tokio's paused clock, full HTTP behavior (200, 503 + `Retry-After` then 200, permanent 404, retries exhausted) against `wiremock`
    - [x] Two-step pagination per matière URL: page 0 gives total + page size → remaining pages fan out under the shared throttle; arithmetic reconciliation guarantees completeness (merged count == advertised total, per-page totals agree, hard page cap) — ADR `2026-07-pagination-du-catalogue-par-comptage`; the computed page count is an upper bound, trailing « Aucun résultat » pages tolerated when empty — ADR `2026-07-tolerance-des-pages-aucun-resultat-du-fan-out`; any mismatch = error that stops the run, never a silent truncation
    - [x] Merge, sort, dedup entries across pages and matières (same shape as `test_cases/catalogue/gex.json`)
    - [x] Full catalogue = the union of the matière facets: the site's index caps any query at 10 000 results, so page 0 provides the facet directory and one partition per matière fans out under the one shared throttle (bracketed `matieres%5B<id>%5D=<id>` form only — the flat form is silently ignored by the site); the banner total is ignored — the widget omitting 11 real courses is a ULaval bug the scraper doesn't work around — ADR `2026-07-partition-du-catalogue-par-matiere`, `2026-07-le-catalogue-est-lunion-des-facettes`
    - [x] CLI wiring: a `catalogue` subcommand that writes the merged catalogue JSON (`anyhow` at the binary frontier) — clap 4 derive, exit code 2 on usage errors, `catalogue_errors.log` beside the artifact (ADR `2026-07-cli-dans-la-lib-et-style-derreurs`, `2026-07-adoption-de-clap`); optional `--output-dir`/`--url` flags with defaults `data` + the production URL, no scoped mode (ADR `2026-07-scraper-plein-catalogue-seulement`)
    - Verify: `make test` green (unit + wiremock); run live on the full catalogue and spot-check the unique course count (~10 224, the facet union)
- [ ] Préalables grammar (`parse/prerequisites.rs` — pure function, raw text → `PrereqTree`, no HTML)
    - [ ] ET/OU with parentheses → `all`/`any` trees; « Crédits exigés : N » → `program_credits`
    - [ ] Out-of-grammar text → kept raw-only and surfaced as an anomaly — requires deciding how `core::Prerequisites` represents "raw without tree" (type change + ADR at this step)
    - Verify: unit tests per grammar rule, plus rejection cases that fall back to raw
- [ ] Course page parser (`parse/course.rs`)
    - [ ] Page HTML → `core::Course`: code, title, credits, cycle, prerequisites (raw + tree via the grammar), equivalents, seasons → components (lecture/laboratory) → sections (NRC, section, mode) → slots (day, start, end)
    - Verify: integration test parses each frozen `classes/*.html` and compares `serde_json::Value` with the matching `test_cases/classes/*.json`
- [ ] Program page parser (`parse/program.rs`)
    - [ ] Page HTML → `core::Program`: mandatory, rules (« Règle N — contrainte parmi : » → `count` vs `min`/`max` credits, list vs reference vs keyword courses), concentrations, profiles
    - [ ] Unrecognized rule text → `Raw` variant, surfaced, never ignored
    - Verify: integration test against the three `test_cases/programs/*.json`
- [ ] Create unit tests for parser
    - [ ] Fill the gaps the valid fixtures never exercise: malformed times, unknown component types, unrecognized modes, missing elements
    - Verify: `make test` green; parse modules covered
- [ ] Next, out of parser scope (completes jalon 1): extend fetch to course pages (resume on error), snapshot `data/cours/a2026.json` for the GEX matières
