// Scenario tests: what happens when the app's doors are walked in the
// order a student walks them. Each module is one sequence that starts from
// an existing state — a save, a shared link, a relevé — rather than from
// `Plan::default()`. The rationale and the boundary with the browser tests
// are in `docs/conception/adr/2026-08-tests-de-scenario-dans-ui.md`.

mod harness;

mod documents;
mod reload;
mod share;
mod start;
mod transcript;
mod verdict;
