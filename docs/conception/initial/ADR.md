# ULaval Schedule Builder — Architecture and Decision Record

**Date:** July 2026
**Status:** Planning complete, implementation not started
**Purpose:** Automatic schedule creation and management for Université Laval courses, shared with the program director. Secondary purpose: serve as a vehicle for learning Rust in depth, building foundations reusable for a future coastal engineering digital twin platform (3D, web- or desktop-distributed).

---

## 1. Goals and constraints

The primary deliverable is a tool that takes the ULaval course catalogue and generates conflict-free schedules according to preferences. The developer knows some Rust and wants to learn it substantially better through this project; existing experience is in Python, JS, and Elm. The app must be usable by a non-technical stakeholder (the program director) without any manual data-update ritual. Performance is not a concern at this scale. A future project (coastal engineering digital twin) will involve 3D and should be shareable via web or desktop, but its implementation details are explicitly out of scope; only transferable foundations matter here.

Two constraints discovered during investigation shape everything below: ULaval's portal renders HTML on the server (no JSON API is exposed to the browser), and the course data will not need mid-semester refresh — updates happen when a new semester or year begins.

## 2. Guiding principles

**Decouple the two projects.** The transferable assets between the schedule builder and the digital twin are Rust proficiency, Cargo workspace organization, and the compile-to-WASM pattern — not any specific UI framework or rendering stack. The schedule app's stack is chosen on its own merits; the digital twin's hard problems (3D rendering, simulation compute) share almost nothing with a schedule builder and their decisions are deferred until they're real.

**Pure core, thin shells.** All domain logic lives in a library crate with zero IO. Everything that touches the network, the filesystem, or the DOM is a thin shell around it. This is simultaneously the best Rust-learning structure (pure functions are testable, and the type system does its best work on domain modeling), the cheapest architecture to change (the frontend is the smallest, most mechanical layer), and the exact pattern the digital twin will need (a Rust simulation core feeding multiple targets).

**Data acquisition is a batch job, not runtime infrastructure.** Course catalogues change on a timescale of semesters. Treating scraping as something that happens at request time would force a server into existence for no benefit; treating it as a periodic batch that emits a static snapshot removes the server entirely.

---

## 3. Decisions

### D1 — Web application, not desktop

**Decision:** Ship as a browser-based app.

**Reasoning:** The audience is a program director and, plausibly, students. This audience clicks links; "download and run this binary" loses most users at the door and creates a support burden (OS variants, updates, security warnings). A web app is also trivially "runs on a server or locally" — it's a static site that can be opened from anywhere.

**Rejected alternative — iced (native desktop):** iced is the most Elm-faithful Rust GUI framework (explicit Elm Architecture: Message/update/view), which made it attractive given the developer's Elm background. It was rejected because its web story is effectively dead: the DOM-targeting `iced_web` crate was dropped and is unmaintained, and the project has refocused on native GUI. Compiling iced to WASM renders the whole app into a canvas, which forfeits text selection, accessibility, native scrolling, mobile input, and — critically — URL semantics (deep links, back/forward, and the URL-encoded schedule sharing described in D2 all die inside an opaque canvas). iced remains a serious candidate for the digital twin's desktop variant later, because it renders through wgpu and can embed custom GPU scenes inside its widget tree.

### D2 — Fully static, serverless architecture

**Decision:** The app is a client-side WASM application served as static files (e.g., GitHub Pages). Course data is a pre-generated JSON snapshot fetched from the same origin. There is no backend at launch.

**Reasoning:** Once data acquisition is a batch job (Principle 3), nothing remains that requires a server: the solver runs fine in the browser, and even schedule *sharing* needs no persistence — a chosen schedule is just a set of section identifiers, which can be encoded in the URL. A static site costs nothing to host, has no ops burden, cannot go down independently of its host, and "deploying" is copying files. Every alternative adds a recurring operational tax (keeping a process alive, patching, paying) that a solo hobby project should not accept without a forcing requirement.

**Deferred, with an explicit trigger:** An Axum server crate is pre-planned in the workspace layout but not built. The concrete condition that would justify building it: a future need for on-demand data refresh faster than the CI cron provides (e.g., tracking section availability during registration week), or shared mutable state such as user accounts. Neither is currently planned (D4 confirms mid-year schedule changes are out of scope).

### D3 — Data acquisition: HTML scraping as an offline batch job

**Decision:** A standalone async Rust binary fetches the catalogue pages and parses the server-rendered HTML into a normalized JSON snapshot, one file per term.

**Reasoning and findings:** The first move was to check the browser network tab for internal JSON endpoints, because deserializing JSON with `serde` beats HTML parsing in robustness and code volume. Investigation confirmed the portal builds HTML on the backend, so scraping it is. Practical consequences:

The portal is session-based (Banner-style), so the fetch layer uses `reqwest` with a cookie store, and search results likely come from form POSTs. The tactic is to inspect the exact request the search form sends and replay it directly with the right parameters, rather than navigating pages like a human — fewer requests, less fragile.

Parsing uses the `scraper` crate with CSS selectors. Fetching and parsing are strictly separated modules: the fetcher saves raw HTML responses to disk, and the parser is tested against those frozen files as fixtures. When ULaval changes its markup, the result is a failing test on a checked-in file, not silent garbage in production data. This separation is also idiomatic Rust structure and forces good error-handling practice (`thiserror` for the library errors, `anyhow` at the binary boundary).

**Fragility is accepted and contained:** scraping server-rendered HTML will break when the markup changes. The mitigation is not to prevent breakage (impossible) but to make it loud (fixture tests, CI failure notifications from D4) and cheap to fix (parsing isolated in one module).

### D4 — Catalogue updates via scheduled CI, not in-app refresh

**Decision:** The scraper binary runs on a GitHub Actions cron schedule. It writes the JSON snapshot, commits it, and the static site redeploys automatically. There is no refresh button and no in-app scraping.

**Reasoning:** The stated requirement was that the director should never run a script — updates should happen when he opens the app after a new semester, or via a button. The button is a proxy for the real requirement, which is *the data is always current when he looks*. A scheduled cron satisfies that requirement strictly better than a button: it doesn't depend on a human remembering to press anything, and the director simply always sees fresh data. Since mid-year schedule-change handling is explicitly not planned, a nightly (or even weekly) cadence exceeds the actual freshness need by orders of magnitude. The entire update mechanism is ~30 lines of workflow YAML wrapped around the binary that exists anyway.

**Rejected alternative — scraping from the frontend:** technically impossible, and worth recording why. Parsing could run in the browser (the `scraper` crate compiles to WASM; parsing is pure computation), but *fetching* cannot: the browser's same-origin policy blocks reading cross-origin responses unless the target server opts in with CORS headers, which a university portal does not. The session-cookie/form-POST flow from D3 makes it doubly impossible. This is a browser platform constraint, identical for JS and WASM.

**Rejected alternative — refresh button triggering CI (`workflow_dispatch`):** works mechanically, but the API call requires a token, and there is no safe place for a token in a public client-side app. Solving that means adding a serverless function to hold the token — a worse version of just having a server.

**Rejected alternative — small Axum server with a `/refresh` endpoint:** the clean solution if on-demand refresh were required (frontend fetches from its own origin, so CORS vanishes). Rejected because the requirement doesn't exist, and the cost is not the code but the permanent deployment to keep alive. Remains the documented escalation path per D2.

### D5 — Cargo workspace with a pure core

**Decision:** One repository, one Cargo workspace, three crates now and up to two later:

`core` (library) — domain types (`Course`, `Section`, `TimeBlock`, `Schedule`), conflict detection, and the schedule generator. Zero IO, zero async, no dependencies beyond `serde` and utility crates. Compiles identically to native (for the scraper and tests) and to WASM (for the UI).

`scraper` (binary) — the batch job from D3. Depends on `core` for output types. Native-only, async (`tokio` + `reqwest`).

`ui` (binary, WASM) — the frontend from D6. Depends on `core`. Loads the JSON snapshot, drives the solver, renders.

`server` (Axum) and a Tauri desktop wrapper are reserved names, built only if their trigger conditions (D2, or a desktop distribution need) materialize.

**Reasoning:** Schedule generation is a constraint-satisfaction problem — enumerate section combinations per course, prune time conflicts, score survivors against preferences (compact days, no early mornings, lunch gaps, minimal campus walking). This is the intellectual heart of the project and the richest Rust-learning surface: enums and pattern matching for the domain model, iterators for the search, traits for pluggable scoring strategies, and a real test suite because everything is a pure function. Keeping it IO-free is what makes the single codebase serve both native and browser targets, which is the exact single-source-of-truth pattern the digital twin will need.

The isolation also caps the blast radius of every risky decision elsewhere in this document: if the frontend framework choice (D6) sours, the view layer is the smallest and most mechanical part of the codebase to port.

### D6 — Frontend: Dioxus (over Leptos, Yew, and an Elm hybrid)

**Decision:** Build the UI in Dioxus as a client-side rendered WASM app.

**Reasoning, and the honest shape of the tradeoff:** For a client-side schedule app, the loudly-debated differences between the top frameworks mostly wash out — both Dioxus and Leptos are far faster than a human clicking a schedule grid, and bundle-size deltas are tens of KB. The decision therefore rests on mental-model fit, tooling friction, and option value:

*Mental model.* Dioxus uses a VDOM with component re-runs plus signals — React-shaped, which is, counterintuitively, closer to Elm's "view is a function of state, re-run and diff" than Leptos is. Leptos components run once and wire fine-grained signal subscriptions to real DOM nodes (Solid-shaped); it's arguably the more elegant model but requires genuinely unlearning TEA rendering instincts, spending learning budget on a reactivity paradigm instead of on Rust.

*Tooling.* Dioxus has a funded full-time team and, as of 0.7, the `dx` CLI ships hot-reloading including experimental runtime hot-patching of Rust code, integrated debugging, and WASM code splitting. On a solo hobby project, friction kills momentum, and Dioxus currently has the least of it. Known caveat: hot-patching only tracks the tip crate, so edits to the `core` workspace dependency still require normal recompiles — acceptable, since the solver is where the compile-check-test loop is wanted anyway.

*Option value.* Dioxus targets web, desktop, and mobile from one codebase, which keeps a door open toward the digital twin's "web or desktop" question at zero present cost.

**Leptos (runner-up):** the purer web framework — smaller binaries, fine-grained updates, a router deliberately built on web fundamentals (relevant to URL-encoded sharing), and the best SSR/server-function story in Rust. It would be the pick if the eventual backend were a certainty or if learning fine-grained reactivity were itself a goal. Both frameworks are pre-1.0 and will ship breaking changes; that tax is ecosystem-wide and not a differentiator.

**Yew (rejected):** the oldest option, and dominated on every axis relevant here. Its historical advantages (maturity, accumulated ecosystem) serve teams maintaining existing Yew apps, not greenfield projects. It went roughly two and a half years without a release (v0.21 in late 2023 to v0.22 in early 2026) — it has resumed activity, but a volunteer project that stalled that long carries momentum risk, and community energy has visibly moved to Dioxus and Leptos. It offers no desktop story, no first-class fullstack story, and no funded development.

**Elm frontend + Rust WASM core (rejected):** would leverage the strongest existing skill, but Rust WASM and Elm only communicate through JS glue and ports, with serialization at every boundary crossing. The effort would go into plumbing instead of Rust, defeating the learning goal.

---

## 4. Data flow (end to end)

GitHub Actions cron fires → `scraper` binary authenticates a session, replays the catalogue search requests, saves raw HTML, parses it via `core` types into `data/<term>.json` → workflow commits the snapshot → static-site deploy republishes → `ui` (WASM in the visitor's browser) fetches the JSON, and all schedule generation runs locally in the browser via the same `core` crate → a chosen schedule is shareable as a URL encoding its section IDs. No server exists anywhere in the path.

## 5. Build order

1. **Scraper first.** It resolves the project's single biggest external risk — whether the data is obtainable at all, and in what shape — before any code depends on that shape. It also front-loads async Rust, error handling, and serde. Deliverable: `data/<term>.json` plus HTML fixtures and parser tests.
2. **Core solver second.** Pure Rust against real data from step 1. Deliverable: a CLI or test harness that prints valid schedules for a set of course codes, with a scoring trait and at least conflict-freedom property-tested.
3. **UI last.** By this point it is a rendering problem, not a design problem. Deliverable: course selection, preference controls, schedule grid, URL sharing.
4. **CI cron.** Wire the step-1 binary into a scheduled workflow with failure notifications (a broken scrape must page a human, since the failure mode is silently stale data).

Each stage consumes real output from the previous one, and the riskiest unknown dies first.

## 6. Risks and mitigations

**Markup drift** (certainty, not risk): contained by fetch/parse separation, fixture tests, and CI failure alerts (D3, D4). Time-to-fix is the metric, not prevention.

**Portal access friction:** if the catalogue requires authenticated sessions even for public course data, the CI job needs credentials in repository secrets, and the scraper must be a polite citizen — throttled requests, honest user agent, and a check of the portal's terms of use. If access from CI proves impractical, fallback is running the scraper from a personal machine on a schedule, which degrades automation but changes nothing architecturally.

**Pre-1.0 framework churn:** both viable frontends ship breaking changes between minors. Mitigated by the pure-core split (D5): migrations touch only the thin view layer. Pin versions; upgrade deliberately, not automatically.

**WASM bundle size:** not a launch concern at this app's scale; if it grows, Dioxus 0.7's code splitting and `wasm-opt` are the levers.

## 7. Notes for the digital twin (deliberately deferred)

What transfers directly from this project: workspace organization, the pure-core/IO-shell split, WASM build pipeline, serde modeling, async Rust. What is deliberately *not* being decided now: the 3D stack. When that decision becomes real, the candidates are `wgpu` (Rust-native, compiles to WebGPU — future-proof but low-level; browser support still uneven outside Chrome), Bevy (higher-level, web-capable, game-engine ergonomics), a hybrid where three.js renders and Rust/WASM computes (three.js's scientific-visualization ecosystem is years ahead of Rust's), or iced hosting a custom wgpu viewport for a desktop-native variant. The pure-simulation-core-plus-thin-rendering-shell pattern established here is the correct preparation for all four.

## 8. Open questions

Whether the public course catalogue is reachable without credentials (determines CI secret handling — resolve during step 1). Snapshot format finalization: plain JSON is the default; SQLite via `rusqlite` only if the data outgrows what the browser comfortably holds in memory, which is unlikely for one program's catalogue. Exact preference/scoring model for the solver — to be designed against real data in step 2.
