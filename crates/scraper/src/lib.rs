// the coverage attribute is only used inside #[cfg(test)] modules, so only
// declare the feature there — declaring it in the non-test build would trip
// the unused_features lint
#![cfg_attr(all(coverage_nightly, test), feature(coverage_attribute))]

pub mod catalogue;
pub mod cli;
pub mod fetch;
pub mod parser;
mod print;
