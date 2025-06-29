use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    pub priority: Option<String>,
    pub dependencies: Vec<String>,
    pub details: Option<String>,
    #[serde(rename = "testStrategy")]
    pub test_strategy: Option<String>,
    #[serde(deserialize_with = "deserialize_subtasks")]
    pub subtasks: Vec<Task>,
    pub assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskmasterFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
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

/// Custom deserializer for subtasks that can handle both string arrays and Task arrays
fn deserialize_subtasks<'de, D>(deserializer: D) -> Result<Vec<Task>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use serde_json::Value;

    struct SubtasksVisitor;

    impl<'de> Visitor<'de> for SubtasksVisitor {
        type Value = Vec<Task>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an array of tasks or strings")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut tasks = Vec::new();
            let mut idx = 0;

            while let Some(value) = seq.next_element::<Value>()? {
                match value {
                    Value::String(s) => {
                        // Convert string to a simple task
                        tasks.push(Task {
                            id: format!("subtask-{}", idx),
                            title: s,
                            description: String::new(),
                            status: "pending".to_string(),
                            priority: None,
                            dependencies: Vec::new(),
                            details: None,
                            test_strategy: None,
                            subtasks: Vec::new(),
                            assignee: None,
                        });
                    }
                    Value::Object(_) => {
                        // Deserialize as a full Task
                        let task: Task =
                            serde_json::from_value(value).map_err(de::Error::custom)?;
                        tasks.push(task);
                    }
                    _ => {
                        return Err(de::Error::custom("subtask must be a string or object"));
                    }
                }
                idx += 1;
            }

            Ok(tasks)
        }
    }

    deserializer.deserialize_seq(SubtasksVisitor)
}
