#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod catalogue;
pub mod common;
pub mod course;
pub mod feasibility;
pub mod intake;
pub mod organigramme;
pub mod program;
pub mod rules;
pub mod week;
pub mod weekly;

pub use catalogue::*;
pub use common::*;
pub use course::*;
pub use feasibility::*;
pub use intake::*;
pub use organigramme::*;
pub use program::*;
pub use rules::*;
pub use week::*;
pub use weekly::*;
