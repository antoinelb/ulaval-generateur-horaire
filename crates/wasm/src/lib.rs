#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// The frontier crate: `core` exposed to the two browser consumers — the
// plain-JavaScript interface (eight `JsValue` functions) and the Dioxus app's
// Web Worker (a JSON-string protocol) — plus the pure functions the Dioxus
// app calls natively. One orchestration, two boundaries (ADR
// `2026-08-fusion-des-crates-wasm-et-ui-calculations`).

#[cfg(all(target_arch = "wasm32", feature = "boundary"))]
mod boundary;
pub mod catalogue;
pub mod credits;
pub mod merge;
pub mod organigramme;
pub mod protocol;
pub mod questions;
pub mod schedule;
