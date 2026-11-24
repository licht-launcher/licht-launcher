//! Helpers shared by the tests of every workspace crate.
//!
//! This crate is only ever a dev-dependency, so it follows the rules for test code:
//! a helper may panic when the test environment itself is broken. Helpers used by a
//! single crate stay in that crate; move one here once a second crate needs it.

pub mod logs;
