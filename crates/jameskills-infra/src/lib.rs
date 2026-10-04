#![forbid(unsafe_code)]

//! Concrete storage and operating-system adapters for JameSkills.
//!
//! Implementations are introduced with their ports; this crate depends on the
//! domain crate and never defines domain policy.

pub mod composition;
pub mod fs;
pub mod platform;
pub mod process;
pub mod sqlite;
