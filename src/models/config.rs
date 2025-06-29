use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub version: String,
    pub organization: String,
    #[serde(default)]
    pub project_mappings: HashMap<String, ProjectMapping>,
    #[serde(default)]
    pub last_sync: HashMap<String, chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub agent_mapping: HashMap<String, AgentMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMapping {
    pub project_number: i32,
    pub project_id: String,
    /// Repository to create issues in (e.g., "owner/repo")
    pub repository: Option<String>,
    #[serde(default)]
    pub subtask_mode: SubtaskMode,
    pub field_mappings: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtaskMode {
    Nested,   // Subtasks shown as checklists in parent task body
    Separate, // Subtasks created as separate GitHub Project items
}

impl Default for SubtaskMode {
    fn default() -> Self {
        SubtaskMode::Nested
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMapping {
    pub services: Vec<String>,
    pub github_username: String,
    pub rules: Vec<AssignmentRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentRule {
    pub pattern: String,
    pub priority: i32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            organization: String::new(),
            project_mappings: HashMap::new(),
            last_sync: HashMap::new(),
            agent_mapping: HashMap::new(),
        }
    }
}
