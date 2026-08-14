#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// The UI's calculation crate (ADR
// `2026-08-crate-ui-calculations-et-worker`): pure functions the Dioxus
// app calls natively, and the same crate compiled to its own wasm module
// so a Web Worker runs solver B off the main thread (AIR LAT-3).

pub mod credits;
pub mod merge;
pub mod protocol;

#[cfg(target_arch = "wasm32")]
mod boundary;
