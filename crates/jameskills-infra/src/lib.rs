#![forbid(unsafe_code)]

//! Concrete storage and operating-system adapters for JameSkills.
//!
//! Implementations are introduced with their ports; this crate depends on the
//! domain crate and never defines domain policy.

pub mod agents;
pub mod composition;
pub mod crypto;
pub mod fs;
pub mod github;
pub mod platform;
pub mod process;
pub mod snapshot_archive;
pub mod sqlite;
