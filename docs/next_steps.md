# Plan

Test-first parser implementation — step 1 of the build order in `docs/project_plan.md`.
The expected outputs in `tests/fixtures/test_cases/` are done; this plan makes them pass.
Rhythm for each page type: freeze the real HTML → write the failing integration test → implement the parser until it matches the expected JSON → pin edge cases with inline unit tests.

- [x] Build e2e test cases for the parser
    - [x] All courses page (listing)
    - [x] Course page
    - [x] Program page
- [ ] Freeze HTML fixtures
    - [ ] Fetch (one-off `curl`, honest user agent) the real page behind each test case into `tests/fixtures/html/classes/*.html` and `tests/fixtures/html/programs/*.html`, same basename as the expected JSON
    - [ ] Freeze the three GEX facet listing pages into `tests/fixtures/html/listing/gex-page-{0,1,2}.html` (50 courses, 2 courses, empty page — ADR `2026-07-listing-teste-sur-html-gele`)
    - [ ] ADR: HTML fixture location and refresh policy
    - Verify: every `test_cases/classes/*.json` and `test_cases/programs/*.json` has a same-named `.html` source, and the three listing pages are present
- [ ] Parser skeleton in `scraper`
    - [ ] Dependencies: `scraper` (HTML parsing), `thiserror` (library-side errors; `anyhow` stays at the binary frontier)
    - [ ] Module layout: `parse/` with `listing.rs`, `prerequisites.rs`, `course.rs`, `program.rs`, and a shared error type that carries the offending raw text (an anomaly is data, never a panic)
    - Verify: `cargo check` passes with the empty module tree
- [ ] Listing parser (`parse/listing.rs`)
    - [ ] One page of HTML → `{code, title, url}` entries + `total_results`
    - [ ] Termination signal: 0 entries **with** the `total-resultats` element present; 0 entries **without** it = markup drift = error, not end of results
    - [ ] Malformed entry → raw error line, never silently dropped
    - Verify: integration test parses the three frozen listing pages, merges, sorts and dedups, and compares with `test_cases/listing/gex.json`; unit tests on inline snippets pin what the frozen pages never exercise (empty page without `total-resultats` = drift, malformed entry)
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
- [ ] Next, out of parser scope (completes jalon 1): fetch module (throttled ~10 req/s, resume), CLI wiring, snapshot `data/cours/a2026.json` for the GEX matières
