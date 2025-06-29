use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: Option<String>,
    pub dependencies: Vec<String>,
    pub details: Option<String>,
    #[serde(rename = "testStrategy")]
    pub test_strategy: Option<String>,
    pub subtasks: Vec<Task>,
    pub assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskmasterFile {
    #[serde(flatten)]
    pub tasks: TaskmasterTasks,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TaskmasterTasks {
    // Legacy format: { "tasks": [...] }
    Legacy { tasks: Vec<Task> },
    // Tagged format: { "tag1": { "tasks": [...] }, "tag2": { "tasks": [...] } }
    Tagged(HashMap<String, TaggedTasks>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedTasks {
    pub tasks: Vec<Task>,
    pub metadata: Option<TaskMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub created: Option<String>,
    pub updated: Option<String>,
    pub description: Option<String>,
}
