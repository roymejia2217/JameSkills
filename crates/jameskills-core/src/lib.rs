#![forbid(unsafe_code)]

//! Platform-independent domain types and application contracts for JameSkills.
//!
//! This crate deliberately has no GUI, database, filesystem, process, or network
//! dependencies. Domain and application services are added in later slices.

pub mod domain;
mod error;

pub use domain::{
    ContentHash, IdValidationError, OperationId, PathValidationError, PortablePath, RevisionId,
    SkillId,
};
pub use error::{AppError, AppResult, Diagnostic, DiagnosticSeverity};
