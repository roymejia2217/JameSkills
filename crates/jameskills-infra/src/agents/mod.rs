//! Conservative registry of app-owned agents.

mod common;
mod registry;
pub mod codex;
pub mod opencode;

pub use common::AgentArtifact;
pub(crate) use common::{find_agent_executable_candidate, plan_file_copy_artifact};
pub use registry::AgentRegistry;
