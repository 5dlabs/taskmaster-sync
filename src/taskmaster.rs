//! TaskMaster file reader and writer
//!
//! This module handles:
//! - Reading tasks from .taskmaster/tasks/tasks.json
//! - Writing updates back to TaskMaster
//! - Task validation and formatting
//! - File locking for concurrent access

use crate::error::{Result, TaskMasterError};
use crate::models::task::{TaggedTasks, Task, TaskmasterFile, TaskmasterTasks};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;

/// Reads and writes TaskMaster task files
pub struct TaskMasterReader {
    tasks_path: PathBuf,
    tasks: RwLock<HashMap<String, TaggedTasks>>,
}

impl TaskMasterReader {
    /// Creates a new TaskMaster reader
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let tasks_path = project_root
            .as_ref()
            .join(".taskmaster")
            .join("tasks")
            .join("tasks.json");

        Self {
            tasks_path,
            tasks: RwLock::new(HashMap::new()),
        }
    }

    /// Loads tasks from tasks.json
    pub async fn load_tasks(&self) -> Result<HashMap<String, TaggedTasks>> {
        // Check if file exists
        if !self.tasks_path.exists() {
            return Err(TaskMasterError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Tasks file not found at: {:?}", self.tasks_path),
            )));
        }

        // Read file content
        let content = fs::read_to_string(&self.tasks_path)
            .await
            .map_err(|e| TaskMasterError::IoError(e))?;

        // Parse JSON
        let parsed: TaskmasterFile =
            serde_json::from_str(&content).map_err(|e| TaskMasterError::JsonError(e))?;

        // Handle different formats
        let tasks_map = match parsed.tasks {
            TaskmasterTasks::Legacy { tasks } => {
                // Convert legacy format to tagged format
                let mut map = HashMap::new();
                map.insert(
                    "master".to_string(),
                    TaggedTasks {
                        tasks,
                        metadata: None,
                    },
                );
                map
            }
            TaskmasterTasks::Tagged(map) => map,
        };

        // Update internal cache
        let mut cache = self.tasks.write().await;
        *cache = tasks_map.clone();

        Ok(tasks_map)
    }

    /// Saves tasks back to tasks.json
    pub fn save_tasks(&self, _tasks: Vec<Task>) -> Result<()> {
        todo!("Save tasks to tasks.json with proper formatting")
    }

    /// Gets tasks for a specific tag
    pub async fn get_tasks_for_tag(&self, tag: &str) -> Option<Vec<Task>> {
        let cache = self.tasks.read().await;
        cache.get(tag).map(|tagged| tagged.tasks.clone())
    }

    /// Gets all tasks across all tags
    pub async fn get_all_tasks(&self) -> Vec<Task> {
        let cache = self.tasks.read().await;
        cache
            .values()
            .flat_map(|tagged| tagged.tasks.iter().cloned())
            .collect()
    }

    /// Gets a specific task by ID
    pub fn get_task(&self, _task_id: &str) -> Option<Task> {
        todo!("Find task by ID")
    }

    /// Updates a specific task
    pub fn update_task(&self, _task: Task) -> Result<()> {
        todo!("Update specific task in memory and save")
    }

    /// Adds a new task
    pub fn add_task(&self, _task: Task) -> Result<()> {
        todo!("Add new task and save")
    }

    /// Removes a task
    pub fn remove_task(&self, _task_id: &str) -> Result<()> {
        todo!("Remove task and save")
    }

    /// Validates task data
    pub fn validate_task(&self, _task: &Task) -> Result<()> {
        todo!("Validate task fields and consistency")
    }

    /// Reloads tasks from disk
    pub fn reload(&self) -> Result<()> {
        todo!("Reload tasks from disk")
    }

    /// Checks if tasks.json exists
    pub fn exists(&self) -> bool {
        self.tasks_path.exists()
    }

    /// Gets tasks that match a filter
    pub async fn filter_tasks<F>(&self, predicate: F) -> Vec<Task>
    where
        F: Fn(&Task) -> bool,
    {
        let cache = self.tasks.read().await;
        cache
            .values()
            .flat_map(|tagged| &tagged.tasks)
            .filter(|t| predicate(t))
            .cloned()
            .collect()
    }

    /// Updates multiple tasks in a batch
    pub fn batch_update(&self, _updates: Vec<Task>) -> Result<()> {
        todo!("Update multiple tasks efficiently")
    }
}

/// Task file format utilities
pub mod format {
    use super::*;

    /// Formats tasks.json with proper indentation
    pub fn format_tasks_json(tasks_map: &HashMap<String, TaggedTasks>) -> Result<String> {
        let json =
            serde_json::to_string_pretty(tasks_map).map_err(|e| TaskMasterError::JsonError(e))?;
        Ok(json)
    }

    /// Parses raw JSON into tasks
    pub fn parse_tasks_json(content: &str) -> Result<TaskmasterFile> {
        serde_json::from_str(content).map_err(|e| TaskMasterError::JsonError(e))
    }

    /// Validates JSON structure
    pub fn validate_json_structure(value: &Value) -> Result<()> {
        // Check if it's an object
        if !value.is_object() {
            return Err(TaskMasterError::InvalidTaskFormat(
                "Root must be an object".to_string(),
            ));
        }

        let obj = value.as_object().unwrap();

        // Check for legacy format
        if obj.contains_key("tasks") && obj["tasks"].is_array() {
            return Ok(()); // Valid legacy format
        }

        // Check for tagged format
        for (tag, tag_value) in obj {
            if !tag_value.is_object() {
                return Err(TaskMasterError::InvalidTaskFormat(format!(
                    "Tag '{}' must contain an object",
                    tag
                )));
            }

            let tag_obj = tag_value.as_object().unwrap();
            if !tag_obj.contains_key("tasks") || !tag_obj["tasks"].is_array() {
                return Err(TaskMasterError::InvalidTaskFormat(format!(
                    "Tag '{}' must contain a 'tasks' array",
                    tag
                )));
            }
        }

        Ok(())
    }
}

/// File locking utilities
mod lock {
    use super::*;

    /// Acquires a file lock for safe writing
    pub fn acquire_lock(_path: &Path) -> Result<FileLock> {
        todo!("Implement file locking mechanism")
    }

    pub struct FileLock {
        // TODO: Implement file lock
    }

    impl Drop for FileLock {
        fn drop(&mut self) {
            // TODO: Release lock on drop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_tasks_tagged_format() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_path = temp_dir.path().join(".taskmaster").join("tasks");

        tokio::fs::create_dir_all(&tasks_path).await.unwrap();

        let test_json = r#"{
            "master": {
                "tasks": [
                    {
                        "id": "1",
                        "title": "Test Task",
                        "description": "Test Description",
                        "status": "pending",
                        "priority": "high",
                        "dependencies": [],
                        "details": null,
                        "testStrategy": null,
                        "subtasks": [],
                        "assignee": null
                    }
                ],
                "metadata": {
                    "created": "2024-01-01T00:00:00Z",
                    "updated": "2024-01-01T00:00:00Z",
                    "description": "Test tasks"
                }
            }
        }"#;

        let file_path = tasks_path.join("tasks.json");
        tokio::fs::write(&file_path, test_json).await.unwrap();

        let reader = TaskMasterReader::new(temp_dir.path());
        let tasks = reader.load_tasks().await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains_key("master"));
        assert_eq!(tasks["master"].tasks.len(), 1);
        assert_eq!(tasks["master"].tasks[0].title, "Test Task");
    }

    #[tokio::test]
    async fn test_load_tasks_legacy_format() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_path = temp_dir.path().join(".taskmaster").join("tasks");

        tokio::fs::create_dir_all(&tasks_path).await.unwrap();

        let test_json = r#"{
            "tasks": [
                {
                    "id": "1",
                    "title": "Legacy Task",
                    "description": "Legacy Description",
                    "status": "done",
                    "priority": "low",
                    "dependencies": ["2"],
                    "details": "Some details",
                    "testStrategy": "Test strategy",
                    "subtasks": [],
                    "assignee": "john"
                }
            ]
        }"#;

        let file_path = tasks_path.join("tasks.json");
        tokio::fs::write(&file_path, test_json).await.unwrap();

        let reader = TaskMasterReader::new(temp_dir.path());
        let tasks = reader.load_tasks().await.unwrap();

        // Legacy format should be converted to tagged format with "master" tag
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains_key("master"));
        assert_eq!(tasks["master"].tasks.len(), 1);
        assert_eq!(tasks["master"].tasks[0].title, "Legacy Task");
    }

    #[test]
    fn test_json_structure_validation() {
        use serde_json::json;

        // Valid tagged format
        let valid_tagged = json!({
            "master": {
                "tasks": []
            }
        });
        assert!(format::validate_json_structure(&valid_tagged).is_ok());

        // Valid legacy format
        let valid_legacy = json!({
            "tasks": []
        });
        assert!(format::validate_json_structure(&valid_legacy).is_ok());

        // Invalid - not an object
        let invalid_array = json!([]);
        assert!(format::validate_json_structure(&invalid_array).is_err());

        // Invalid - missing tasks array in tag
        let invalid_tag = json!({
            "master": {
                "notTasks": []
            }
        });
        assert!(format::validate_json_structure(&invalid_tag).is_err());
    }

    #[test]
    fn test_task_serialization_roundtrip() {
        let task = Task {
            id: "123".to_string(),
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            status: "pending".to_string(),
            priority: Some("high".to_string()),
            dependencies: vec!["456".to_string()],
            details: Some("Details".to_string()),
            test_strategy: Some("Strategy".to_string()),
            subtasks: vec![],
            assignee: Some("alice".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&task).unwrap();

        // Deserialize back
        let deserialized: Task = serde_json::from_str(&json).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.id, task.id);
        assert_eq!(deserialized.title, task.title);
        assert_eq!(deserialized.description, task.description);
        assert_eq!(deserialized.status, task.status);
        assert_eq!(deserialized.priority, task.priority);
        assert_eq!(deserialized.dependencies, task.dependencies);
        assert_eq!(deserialized.details, task.details);
        assert_eq!(deserialized.test_strategy, task.test_strategy);
        assert_eq!(deserialized.assignee, task.assignee);
    }
}
