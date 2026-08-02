#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[cfg(target_arch = "wasm32")]
mod boundary;
pub mod organigramme;
pub mod schedule;
