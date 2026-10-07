//! Conservative registry of app-owned agents.

pub mod antigravity;
pub mod codex;
mod common;
pub mod grok;
pub mod opencode;
pub mod pi;
mod registry;

pub use common::AgentArtifact;
pub(crate) use common::{find_agent_executable_candidate, plan_file_copy_artifact};
pub use registry::AgentRegistry;
