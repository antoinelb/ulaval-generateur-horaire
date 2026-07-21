#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod catalogue;
pub mod common;
pub mod course;
pub mod program;

pub use catalogue::*;
pub use common::*;
pub use course::*;
pub use program::*;
