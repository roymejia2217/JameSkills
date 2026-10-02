use serde::{Deserialize, Serialize};

/// Context in which a policy, installation, or related operation is scoped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
}
