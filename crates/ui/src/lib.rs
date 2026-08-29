#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// The Dioxus app as a library: pure modules compile (and are tested)
// natively; the view and the browser glue exist only under wasm32 — the
// same split as `wasm/src/boundary.rs`. `main.rs` only launches.

pub mod alerts;
pub mod capsule;
pub mod cheminement;
pub mod data;
pub mod export;
pub mod import;
pub mod panel;
pub mod persist;
pub mod present;
pub mod solve;
pub mod state;

#[cfg(target_arch = "wasm32")]
pub mod browser;
#[cfg(target_arch = "wasm32")]
pub mod components;
