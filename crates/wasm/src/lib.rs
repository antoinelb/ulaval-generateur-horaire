#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// The frontier crate: `core` exposed to the Dioxus app's Web Worker (a
// JSON-string protocol), plus the pure functions the app calls natively —
// one orchestration (ADR `2026-08-fusion-des-crates-wasm-et-ui-calculations`).
// A second, frozen surface of eight `JsValue` functions rides along in
// `boundary.rs`: still built and published, no longer a contract anything
// is designed around (ADR `2026-08-surface-javascript-plus-une-contrainte`).

#[cfg(all(target_arch = "wasm32", feature = "boundary"))]
mod boundary;
pub mod catalogue;
pub mod credits;
pub mod merge;
pub mod organigramme;
pub mod protocol;
pub mod questions;
pub mod schedule;
