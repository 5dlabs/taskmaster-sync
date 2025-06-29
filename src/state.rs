//! Task synchronization state tracking
//!
//! This module implements state tracking to prevent duplicate items by using
//! the TM_ID field to identify which tasks have already been synced.

use crate::error::{Result, TaskMasterError};
use crate::models::task::Task;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Tracks synchronization state between TaskMaster and GitHub
#[derive(Debug, Clone)]
pub struct StateTracker {
    state: Arc<RwLock<SyncState>>,
    state_file: PathBuf,
}

/// The actual synchronization state data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncState {
    /// Maps TM_ID to GitHub Project Item ID
    task_mappings: HashMap<String, String>,

    /// Set of TM_IDs that have been synced
    synced_tasks: HashSet<String>,

    /// Maps TM_ID to task metadata for quick lookups
    task_metadata: HashMap<String, TaskMetadata>,

    /// Last sync timestamp
    #[serde(with = "chrono::serde::ts_seconds_option")]
    last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

/// Metadata about a synced task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub github_item_id: String,
    pub draft_issue_id: Option<String>,
    pub title: String,
    pub status: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl StateTracker {
    /// Creates a new state tracker
    pub async fn new(state_file: impl AsRef<Path>) -> Result<Self> {
        let state_file = state_file.as_ref().to_path_buf();
        let state = if state_file.exists() {
            Self::load_state(&state_file).await?
        } else {
            SyncState::default()
        };

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            state_file,
        })
    }

    /// Loads state from file
    async fn load_state(path: &Path) -> Result<SyncState> {
        let content = fs::read_to_string(path).await?;
        let state = serde_json::from_str(&content).map_err(|e| TaskMasterError::JsonError(e))?;
        Ok(state)
    }

    /// Saves state to file
    pub async fn save(&self) -> Result<()> {
        let state = self.state.read().await;
        let content =
            serde_json::to_string_pretty(&*state).map_err(|e| TaskMasterError::JsonError(e))?;

        // Ensure parent directory exists
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.state_file, content).await?;
        Ok(())
    }

    /// Checks if a task has been synced
    pub async fn is_synced(&self, tm_id: &str) -> bool {
        let state = self.state.read().await;
        state.synced_tasks.contains(tm_id)
    }

    /// Gets the GitHub item ID for a TM_ID
    pub async fn get_github_item_id(&self, tm_id: &str) -> Option<String> {
        let state = self.state.read().await;
        state.task_mappings.get(tm_id).cloned()
    }

    /// Gets task metadata
    pub async fn get_task_metadata(&self, tm_id: &str) -> Option<TaskMetadata> {
        let state = self.state.read().await;
        state.task_metadata.get(tm_id).cloned()
    }

    /// Records a task as synced
    pub async fn record_synced(
        &self,
        tm_id: &str,
        github_item_id: &str,
        draft_issue_id: Option<&str>,
        task: &Task,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        // Update mappings
        state
            .task_mappings
            .insert(tm_id.to_string(), github_item_id.to_string());
        state.synced_tasks.insert(tm_id.to_string());

        // Update metadata
        let metadata = TaskMetadata {
            github_item_id: github_item_id.to_string(),
            draft_issue_id: draft_issue_id.map(String::from),
            title: task.title.clone(),
            status: task.status.clone(),
            last_updated: chrono::Utc::now(),
        };
        state.task_metadata.insert(tm_id.to_string(), metadata);

        // Update last sync time
        state.last_sync = Some(chrono::Utc::now());

        Ok(())
    }

    /// Updates task metadata
    pub async fn update_task_metadata(&self, tm_id: &str, task: &Task) -> Result<()> {
        let mut state = self.state.write().await;

        if let Some(metadata) = state.task_metadata.get_mut(tm_id) {
            metadata.title = task.title.clone();
            metadata.status = task.status.clone();
            metadata.last_updated = chrono::Utc::now();
        }

        state.last_sync = Some(chrono::Utc::now());
        Ok(())
    }

    /// Removes a task from the sync state
    pub async fn remove_task(&self, tm_id: &str) -> Result<()> {
        let mut state = self.state.write().await;

        state.task_mappings.remove(tm_id);
        state.synced_tasks.remove(tm_id);
        state.task_metadata.remove(tm_id);

        Ok(())
    }

    /// Finds orphaned items (in state but not in current task list)
    pub async fn find_orphaned_items(&self, current_tasks: &[Task]) -> Vec<String> {
        let state = self.state.read().await;
        let current_ids: HashSet<_> = current_tasks.iter().map(|t| t.id.clone()).collect();

        state
            .synced_tasks
            .iter()
            .filter(|id| !current_ids.contains(*id))
            .cloned()
            .collect()
    }

    /// Gets all synced task IDs
    pub async fn get_synced_ids(&self) -> HashSet<String> {
        let state = self.state.read().await;
        state.synced_tasks.clone()
    }

    /// Gets sync statistics
    pub async fn get_stats(&self) -> SyncStats {
        let state = self.state.read().await;
        SyncStats {
            total_synced: state.synced_tasks.len(),
            last_sync: state.last_sync,
        }
    }

    /// Clears all state (useful for testing or reset)
    pub async fn clear(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = SyncState::default();
        Ok(())
    }

    /// Batch update for multiple tasks
    pub async fn batch_record_synced(
        &self,
        updates: Vec<(String, String, Option<String>, Task)>,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        for (tm_id, github_item_id, draft_issue_id, task) in updates {
            // Update mappings
            state
                .task_mappings
                .insert(tm_id.clone(), github_item_id.clone());
            state.synced_tasks.insert(tm_id.clone());

            // Update metadata
            let metadata = TaskMetadata {
                github_item_id,
                draft_issue_id,
                title: task.title,
                status: task.status,
                last_updated: chrono::Utc::now(),
            };
            state.task_metadata.insert(tm_id, metadata);
        }

        state.last_sync = Some(chrono::Utc::now());
        Ok(())
    }
}

/// Synchronization statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub total_synced: usize,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_tracker() -> (StateTracker, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("state.json");
        let tracker = StateTracker::new(&state_file).await.unwrap();
        (tracker, temp_dir)
    }

    #[tokio::test]
    async fn test_state_persistence() {
        let (tracker, _temp_dir) = create_test_tracker().await;

        // Create a test task
        let task = Task {
            id: "123".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            status: "pending".to_string(),
            priority: Some("high".to_string()),
            dependencies: vec![],
            details: None,
            test_strategy: None,
            subtasks: vec![],
            assignee: None,
        };

        // Record as synced
        tracker
            .record_synced("123", "github-123", Some("draft-123"), &task)
            .await
            .unwrap();

        // Verify it's tracked
        assert!(tracker.is_synced("123").await);
        assert_eq!(
            tracker.get_github_item_id("123").await,
            Some("github-123".to_string())
        );

        // Save state
        tracker.save().await.unwrap();

        // Create new tracker from same file
        let tracker2 = StateTracker::new(&tracker.state_file).await.unwrap();

        // Verify state was persisted
        assert!(tracker2.is_synced("123").await);
        assert_eq!(
            tracker2.get_github_item_id("123").await,
            Some("github-123".to_string())
        );
    }

    #[tokio::test]
    async fn test_orphaned_detection() {
        let (tracker, _temp_dir) = create_test_tracker().await;

        // Create test tasks
        let task1 = Task {
            id: "1".to_string(),
            title: "Task 1".to_string(),
            description: String::new(),
            status: "pending".to_string(),
            priority: None,
            dependencies: vec![],
            details: None,
            test_strategy: None,
            subtasks: vec![],
            assignee: None,
        };

        let task2 = Task {
            id: "2".to_string(),
            title: "Task 2".to_string(),
            description: String::new(),
            status: "pending".to_string(),
            priority: None,
            dependencies: vec![],
            details: None,
            test_strategy: None,
            subtasks: vec![],
            assignee: None,
        };

        // Record both as synced
        tracker
            .record_synced("1", "gh-1", None, &task1)
            .await
            .unwrap();
        tracker
            .record_synced("2", "gh-2", None, &task2)
            .await
            .unwrap();

        // Find orphaned with only task1 remaining
        let orphaned = tracker.find_orphaned_items(&[task1]).await;
        assert_eq!(orphaned, vec!["2"]);
    }

    #[tokio::test]
    async fn test_batch_updates() {
        let (tracker, _temp_dir) = create_test_tracker().await;

        let updates = vec![
            (
                "1".to_string(),
                "gh-1".to_string(),
                None,
                Task {
                    id: "1".to_string(),
                    title: "Task 1".to_string(),
                    description: String::new(),
                    status: "done".to_string(),
                    priority: None,
                    dependencies: vec![],
                    details: None,
                    test_strategy: None,
                    subtasks: vec![],
                    assignee: None,
                },
            ),
            (
                "2".to_string(),
                "gh-2".to_string(),
                Some("draft-2".to_string()),
                Task {
                    id: "2".to_string(),
                    title: "Task 2".to_string(),
                    description: String::new(),
                    status: "in-progress".to_string(),
                    priority: None,
                    dependencies: vec![],
                    details: None,
                    test_strategy: None,
                    subtasks: vec![],
                    assignee: None,
                },
            ),
        ];

        tracker.batch_record_synced(updates).await.unwrap();

        assert!(tracker.is_synced("1").await);
        assert!(tracker.is_synced("2").await);

        let metadata = tracker.get_task_metadata("2").await.unwrap();
        assert_eq!(metadata.draft_issue_id, Some("draft-2".to_string()));
        assert_eq!(metadata.status, "in-progress");
    }

    #[tokio::test]
    async fn test_clear_state() {
        let (tracker, _temp_dir) = create_test_tracker().await;

        // Add some data
        let task = Task {
            id: "test".to_string(),
            title: "Test".to_string(),
            description: String::new(),
            status: "pending".to_string(),
            priority: None,
            dependencies: vec![],
            details: None,
            test_strategy: None,
            subtasks: vec![],
            assignee: None,
        };

        tracker
            .record_synced("test", "gh-test", None, &task)
            .await
            .unwrap();
        assert!(tracker.is_synced("test").await);

        // Clear state
        tracker.clear().await.unwrap();
        assert!(!tracker.is_synced("test").await);
        assert_eq!(tracker.get_synced_ids().await.len(), 0);
    }
}
