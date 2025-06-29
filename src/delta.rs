//! Delta sync engine for intelligent incremental synchronization
//!
//! This module implements change detection and delta sync capabilities to
//! dramatically improve performance by only syncing changed tasks.

use crate::error::{Result, TaskMasterError};
use crate::models::task::Task;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::fs;

/// Represents a change to a task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskChange {
    Added(Box<Task>),
    Modified(Box<Task>, Box<Task>), // (old, new)
    Removed(Box<Task>),
}

/// Result of change detection
#[derive(Debug)]
pub struct ChangeSet {
    pub changes: Vec<TaskChange>,
    pub impacted_task_ids: HashSet<String>,
}

/// Snapshot of tasks for change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub tasks: HashMap<String, TaskFingerprint>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Lightweight fingerprint of a task for change detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskFingerprint {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub dependencies: Vec<String>,
    pub content_hash: String,
}

/// Delta sync engine for change detection
pub struct DeltaSyncEngine {
    snapshot_path: String,
}

impl DeltaSyncEngine {
    /// Creates a new delta sync engine
    pub fn new(tag: &str) -> Self {
        let snapshot_path = format!(".taskmaster/snapshots/{tag}-snapshot.json");
        Self { snapshot_path }
    }

    /// Detects changes between current tasks and last snapshot
    pub async fn detect_changes(
        &self,
        current_tasks: &HashMap<String, Vec<Task>>,
        tag: &str,
    ) -> Result<ChangeSet> {
        // Load previous snapshot if it exists
        let previous_snapshot = self.load_snapshot().await.ok();

        // Get current tasks for the tag
        let tasks = current_tasks
            .get(tag)
            .ok_or_else(|| TaskMasterError::InvalidTaskFormat(format!("Tag '{tag}' not found")))?;

        // Create current snapshot
        let current_snapshot = self.create_snapshot(tasks);

        // Detect changes
        let changes = if let Some(prev) = previous_snapshot {
            self.compare_snapshots(&prev, &current_snapshot, tasks)?
        } else {
            // First sync - all tasks are new
            tasks
                .iter()
                .map(|task| TaskChange::Added(Box::new(task.clone())))
                .collect()
        };

        // Save current snapshot for next time
        self.save_snapshot(&current_snapshot).await?;

        // Calculate impacted tasks (including dependencies)
        let impacted_task_ids = self.calculate_impacted_tasks(&changes, tasks);

        Ok(ChangeSet {
            changes,
            impacted_task_ids,
        })
    }

    /// Creates a snapshot of current tasks
    fn create_snapshot(&self, tasks: &[Task]) -> TaskSnapshot {
        let mut snapshot_tasks = HashMap::new();

        for task in tasks {
            let fingerprint = self.create_fingerprint(task);
            snapshot_tasks.insert(task.id.clone(), fingerprint);
        }

        TaskSnapshot {
            tasks: snapshot_tasks,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Creates a fingerprint for a task
    fn create_fingerprint(&self, task: &Task) -> TaskFingerprint {
        // Create a content hash of task details for deep comparison
        let content = format!(
            "{:?}:{:?}:{:?}:{:?}",
            task.description,
            task.details,
            task.test_strategy,
            task.subtasks.len()
        );
        let content_hash = format!("{:x}", md5::compute(content));

        TaskFingerprint {
            id: task.id.clone(),
            title: task.title.clone(),
            status: task.status.clone(),
            priority: task.priority.clone(),
            assignee: task.assignee.clone(),
            dependencies: task.dependencies.clone(),
            content_hash,
        }
    }

    /// Compares two snapshots to detect changes
    fn compare_snapshots(
        &self,
        previous: &TaskSnapshot,
        current: &TaskSnapshot,
        current_tasks: &[Task],
    ) -> Result<Vec<TaskChange>> {
        let mut changes = Vec::new();
        let current_task_map: HashMap<String, &Task> =
            current_tasks.iter().map(|t| (t.id.clone(), t)).collect();

        // Check for modified and removed tasks
        for (id, prev_fingerprint) in &previous.tasks {
            if let Some(curr_fingerprint) = current.tasks.get(id) {
                // Task exists in both - check if modified
                if prev_fingerprint != curr_fingerprint {
                    if let Some(task) = current_task_map.get(id) {
                        // For now, we only have the new version
                        // In a real implementation, we'd store the full previous task
                        changes.push(TaskChange::Modified(
                            Box::new((*task).clone()),
                            Box::new((*task).clone()),
                        ));
                    }
                }
            } else {
                // Task was removed
                // We'd need to store full tasks in snapshot for this
                // For now, create a minimal removed task
                let removed_task = Task {
                    id: id.clone(),
                    title: prev_fingerprint.title.clone(),
                    description: String::new(),
                    status: prev_fingerprint.status.clone(),
                    priority: prev_fingerprint.priority.clone(),
                    dependencies: prev_fingerprint.dependencies.clone(),
                    subtasks: vec![],
                    details: None,
                    test_strategy: None,
                    assignee: prev_fingerprint.assignee.clone(),
                };
                changes.push(TaskChange::Removed(Box::new(removed_task)));
            }
        }

        // Check for added tasks
        for id in current.tasks.keys() {
            if !previous.tasks.contains_key(id) {
                if let Some(task) = current_task_map.get(id) {
                    changes.push(TaskChange::Added(Box::new((*task).clone())));
                }
            }
        }

        Ok(changes)
    }

    /// Calculates all tasks impacted by changes (including dependencies)
    fn calculate_impacted_tasks(
        &self,
        changes: &[TaskChange],
        all_tasks: &[Task],
    ) -> HashSet<String> {
        let mut impacted = HashSet::new();

        // First, add all directly changed tasks
        for change in changes {
            match change {
                TaskChange::Added(task) | TaskChange::Modified(_, task) => {
                    impacted.insert(task.id.clone());
                }
                TaskChange::Removed(task) => {
                    impacted.insert(task.id.clone());
                }
            }
        }

        // Then, add all tasks that depend on changed tasks
        let changed_ids: HashSet<String> = impacted.clone();
        for task in all_tasks {
            for dep in &task.dependencies {
                if changed_ids.contains(dep) {
                    impacted.insert(task.id.clone());
                }
            }
        }

        impacted
    }

    /// Loads the previous snapshot from disk
    async fn load_snapshot(&self) -> Result<TaskSnapshot> {
        let content = fs::read_to_string(&self.snapshot_path).await?;
        let snapshot = serde_json::from_str(&content)?;
        Ok(snapshot)
    }

    /// Saves the current snapshot to disk
    async fn save_snapshot(&self, snapshot: &TaskSnapshot) -> Result<()> {
        // Ensure directory exists
        if let Some(parent) = Path::new(&self.snapshot_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(snapshot)?;
        fs::write(&self.snapshot_path, content).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_creation() {
        let task = Task {
            id: "test-1".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            status: "pending".to_string(),
            priority: Some("high".to_string()),
            dependencies: vec!["dep-1".to_string()],
            subtasks: vec![],
            details: None,
            test_strategy: None,
            assignee: Some("user1".to_string()),
        };

        let engine = DeltaSyncEngine::new("test");
        let fingerprint = engine.create_fingerprint(&task);

        assert_eq!(fingerprint.id, "test-1");
        assert_eq!(fingerprint.title, "Test Task");
        assert_eq!(fingerprint.status, "pending");
        assert_eq!(fingerprint.priority, Some("high".to_string()));
        assert_eq!(fingerprint.assignee, Some("user1".to_string()));
        assert_eq!(fingerprint.dependencies, vec!["dep-1".to_string()]);
        assert!(!fingerprint.content_hash.is_empty());
    }

    #[test]
    fn test_change_detection() {
        // TODO: Add comprehensive change detection tests
    }
}
