//! Main synchronization engine for TaskMaster to GitHub
//!
//! This module handles:
//! - Full and incremental sync operations
//! - Conflict resolution
//! - Batch optimization
//! - Two-way sync logic

use crate::config::ConfigManager;
use crate::error::{Result, TaskMasterError};
use crate::fields::FieldManager;
use crate::github::{CreateItemResult, GitHubAPI};
use crate::models::github::{FieldValueContent, GitHubProjectItem, Project, ProjectItem};
use crate::models::task::Task;
use crate::progress::{ProgressTracker, SyncStats};
use crate::state::StateTracker;
use crate::subtasks::{SubtaskHandler, SubtaskConfig};
use crate::taskmaster::TaskMasterReader;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

/// Main synchronization engine
pub struct SyncEngine {
    config: ConfigManager,
    github: GitHubAPI,
    taskmaster: TaskMasterReader,
    fields: FieldManager,
    subtasks: SubtaskHandler,
    state: StateTracker,
    project: Option<Project>,
    project_mapping: Option<crate::models::config::ProjectMapping>,
    subtask_config: SubtaskConfig,
}

/// Sync operation options
#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub dry_run: bool,
    pub force: bool,
    pub direction: SyncDirection,
    pub batch_size: usize,
    pub include_archived: bool,
}

/// Sync direction
#[derive(Debug, Clone, PartialEq)]
pub enum SyncDirection {
    ToGitHub,
    FromGitHub,
    Bidirectional,
}

/// Result of a sync operation
#[derive(Debug)]
pub struct SyncResult {
    pub stats: SyncStats,
    pub conflicts: Vec<SyncConflict>,
}

/// Represents a sync conflict
#[derive(Debug)]
pub struct SyncConflict {
    pub task_id: String,
    pub field: String,
    pub taskmaster_value: serde_json::Value,
    pub github_value: serde_json::Value,
    pub resolution: ConflictResolution,
}

/// How to resolve conflicts
#[derive(Debug)]
pub enum ConflictResolution {
    UseTaskMaster,
    UseGitHub,
    Skip,
    Manual(serde_json::Value),
}

impl SyncEngine {
    /// Creates a new sync engine
    pub async fn new(config_path: &str, tag: &str, project_number: i32) -> Result<Self> {
        // Initialize configuration
        let mut config = ConfigManager::new(config_path);
        config.load().await?;

        // Get organization from config
        let org = config.organization();
        if org.is_empty() {
            return Err(TaskMasterError::ConfigError(
                "Organization not configured".to_string(),
            ));
        }

        // Initialize components
        let github = GitHubAPI::new(org.to_string());
        let taskmaster = TaskMasterReader::new(PathBuf::from("."));
        let fields = FieldManager::new();
        let subtasks = SubtaskHandler::new();

        // Initialize state tracker
        let state_file = PathBuf::from(".taskmaster").join(format!("sync-state-{}.json", tag));
        let state = StateTracker::new(state_file).await?;

        // Get project
        let project = github.get_project(project_number).await?;

        // Get project mapping for repository info
        let project_mapping = config.get_project_mapping(tag).cloned();

        Ok(Self {
            config,
            github,
            taskmaster,
            fields,
            subtasks,
            state,
            project: Some(project),
            project_mapping,
            subtask_config: SubtaskHandler::default_config(),
        })
    }

    /// Performs a full synchronization
    pub async fn sync(&mut self, tag: &str, options: SyncOptions) -> Result<SyncResult> {
        // Validate setup
        self.validate_sync_setup().await?;

        // Run appropriate sync based on direction
        match options.direction {
            SyncDirection::ToGitHub => self.sync_to_github(tag, &options).await,
            SyncDirection::FromGitHub => self.sync_from_github(tag, &options).await,
            SyncDirection::Bidirectional => self.sync_bidirectional(tag, &options).await,
        }
    }

    /// Syncs tasks to GitHub
    async fn sync_to_github(&mut self, tag: &str, options: &SyncOptions) -> Result<SyncResult> {
        let start_time = std::time::Instant::now();
        let project = self.project.as_ref().unwrap();
        let project_id = project.id.clone(); // Extract to avoid borrow issues

        // Load tasks for the tag
        let all_tasks = self.taskmaster.load_tasks().await?;
        let tasks = all_tasks.get(tag).ok_or_else(|| {
            TaskMasterError::InvalidTaskFormat(format!("Tag '{}' not found", tag))
        })?;

        // Sync custom fields to GitHub
        self.fields
            .sync_fields_to_github(&self.github, &project_id)
            .await?;

        // Get updated field list with IDs
        let github_fields = self.github.get_project_fields(&project_id).await?;
        self.fields.set_github_fields(github_fields);

        // Get existing GitHub items
        let github_items = self.github.list_project_items(&project_id).await?;

        // Build TM_ID to GitHub item mapping
        let mut tm_id_to_github: HashMap<String, ProjectItem> = HashMap::new();
        for item in github_items {
            // Extract TM_ID from field values
            if let Some(tm_id) = self.extract_tm_id(&item) {
                tm_id_to_github.insert(tm_id, item);
            }
        }

        // Track sync statistics
        let mut created = 0;
        let mut updated = 0;
        let mut deleted = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        // Create progress tracker
        let progress = ProgressTracker::new(tasks.tasks.len());

        // Process tasks in batches
        for task in &tasks.tasks {
            progress.update_main(
                created + updated + skipped,
                &format!("Processing: {}", task.title),
            );

            if options.dry_run {
                println!("DRY RUN: Would process task {}: {}", task.id, task.title);
                skipped += 1;
                continue;
            }

            // Check if task is already synced
            if let Some(github_item) = tm_id_to_github.get(&task.id) {
                // Update existing item
                if let Err(e) = self.update_github_item(task, github_item, &progress).await {
                    errors.push(format!("Failed to update {}: {}", task.id, e));
                    progress
                        .record_error(format!("Error updating {}: {}", task.id, e))
                        .await;
                } else {
                    updated += 1;
                    progress.record_updated(&task.id).await;
                    self.state.update_task_metadata(&task.id, task).await?;
                }
            } else {
                // Create new item
                match self.create_github_item(task, &progress).await {
                    Ok(result) => {
                        created += 1;
                        progress.record_created(&task.id).await;
                        self.state
                            .record_synced(
                                &task.id,
                                &result.project_item_id,
                                Some(&result.draft_issue_id),
                                task,
                            )
                            .await?;
                    }
                    Err(e) => {
                        errors.push(format!("Failed to create {}: {}", task.id, e));
                        progress
                            .record_error(format!("Error creating {}: {}", task.id, e))
                            .await;
                    }
                }
            }
        }

        // Handle orphaned items (in GitHub but not in TaskMaster)
        let current_task_ids: Vec<String> = tasks.tasks.iter().map(|t| t.id.clone()).collect();
        let orphaned = self.state.find_orphaned_items(&tasks.tasks).await;

        for orphan_id in orphaned {
            if let Some(github_item) = tm_id_to_github.get(&orphan_id) {
                if !options.dry_run {
                    if let Err(e) = self
                        .github
                        .delete_project_item(&project_id, &github_item.id)
                        .await
                    {
                        errors.push(format!(
                            "Failed to delete orphaned item {}: {}",
                            orphan_id, e
                        ));
                    } else {
                        deleted += 1;
                        self.state.remove_task(&orphan_id).await?;
                    }
                } else {
                    println!("DRY RUN: Would delete orphaned item {}", orphan_id);
                }
            }
        }

        // Save state
        self.state.save().await?;

        // Finalize progress
        progress.finish();

        let stats = SyncStats {
            total_tasks: tasks.tasks.len(),
            created,
            updated,
            deleted,
            skipped,
            errors: errors.clone(),
            warnings: vec![],
            start_time: Some(start_time),
            end_time: Some(std::time::Instant::now()),
        };

        if !errors.is_empty() {
            eprintln!("\nSync completed with {} errors:", errors.len());
            for error in &errors {
                eprintln!("  - {}", error);
            }
        }

        Ok(SyncResult {
            stats,
            conflicts: vec![],
        })
    }

    /// Creates a new GitHub item for a task
    async fn create_github_item(
        &mut self,
        task: &Task,
        _progress: &ProgressTracker,
    ) -> Result<CreateItemResult> {
        let project_id = self.project.as_ref().unwrap().id.clone();

        // Create the task body (only include simple subtasks inline)
        let body = self.format_task_body_enhanced(task);
        
        // Extract assignees
        let assignees = task.assignee.as_ref().map(|a| vec![a.clone()]);
        
        // Check if we should create a repository issue or draft issue
        let result = if let Some(mapping) = &self.project_mapping {
            if let Some(repository) = &mapping.repository {
                // Create repository issue and add to project
                self.github
                    .create_project_item_with_issue(&project_id, repository, &task.title, &body, assignees)
                    .await?
            } else {
                // Create draft issue
                self.github
                    .create_project_item(&project_id, &task.title, &body)
                    .await?
            }
        } else {
            // Fallback to draft issue
            self.github
                .create_project_item(&project_id, &task.title, &body)
                .await?
        };

        // Process subtasks - create separate issues for complex ones
        if self.subtasks.is_enhanced_mode() && !task.subtasks.is_empty() {
            let repository = self.project_mapping.as_ref().and_then(|m| m.repository.as_deref());
            
            let _subtask_results = self.subtasks.process_subtasks(
                task,
                &result.project_item_id,
                &self.github,
                &project_id,
                repository,
                &self.subtask_config,
            ).await?;
            
            // TODO: Store subtask results in state for tracking
        }

        // Map task fields to GitHub fields
        let mut field_values = self.fields.map_task_to_github(task)?;
        
        // Add hierarchy fields
        self.subtasks.add_hierarchy_fields(&mut field_values, task);

        // Update each field
        for (field_name, value) in field_values {
            if let Some(field_id) = self.fields.get_github_field_id(&field_name) {
                // Format value based on field type with option ID lookup for single select
                let formatted_value = self.format_field_value_enhanced(&field_name, value, &project_id).await?;

                self.github
                    .update_field_value(
                        &project_id,
                        &result.project_item_id,
                        &field_id,
                        formatted_value,
                    )
                    .await?;

                // Small delay to avoid rate limiting
                sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(result)
    }

    /// Updates an existing GitHub item
    async fn update_github_item(
        &mut self,
        task: &Task,
        github_item: &ProjectItem,
        _progress: &ProgressTracker,
    ) -> Result<()> {
        let project_id = self.project.as_ref().unwrap().id.clone();

        // Get the draft issue ID from state
        let metadata = self.state.get_task_metadata(&task.id).await;
        let draft_issue_id = metadata.and_then(|m| m.draft_issue_id);

        if let Some(draft_id) = draft_issue_id {
            // Update the draft issue content with enhanced subtask handling
            let body = self.format_task_body_enhanced(task);
            self.github
                .update_project_item(&project_id, &draft_id, &task.title, &body)
                .await?;
        }

        // Update fields
        let mut field_values = self.fields.map_task_to_github(task)?;
        
        // Add hierarchy fields
        self.subtasks.add_hierarchy_fields(&mut field_values, task);

        for (field_name, value) in field_values {
            if let Some(field_id) = self.fields.get_github_field_id(&field_name) {
                let formatted_value = self.format_field_value_enhanced(&field_name, value, &project_id).await?;
                
                self.github
                    .update_field_value(&project_id, &github_item.id, &field_id, formatted_value)
                    .await?;

                sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(())
    }

    /// Formats task body for GitHub (legacy method)
    fn format_task_body(&self, task: &Task) -> String {
        self.format_task_body_enhanced(task)
    }
    
    /// Formats task body for GitHub with enhanced subtask handling
    fn format_task_body_enhanced(&self, task: &Task) -> String {
        let mut body = task.description.clone();

        if let Some(details) = &task.details {
            body.push_str(&format!("\n\n## Details\n{}", details));
        }

        if let Some(test_strategy) = &task.test_strategy {
            body.push_str(&format!("\n\n## Test Strategy\n{}", test_strategy));
        }

        if !task.subtasks.is_empty() {
            body.push_str("\n\n## Subtasks\n");
            
            let mut separate_subtasks = Vec::new();
            let mut inline_subtasks = Vec::new();
            
            // Separate subtasks into those getting separate issues vs inline
            for subtask in &task.subtasks {
                if self.subtasks.is_enhanced_mode() && 
                   self.should_create_separate_subtask_issue(subtask) {
                    separate_subtasks.push(subtask);
                } else {
                    inline_subtasks.push(subtask);
                }
            }
            
            // Add inline subtasks as checklist
            for (i, subtask) in inline_subtasks.iter().enumerate() {
                let checkbox = if subtask.status == "done" { "[x]" } else { "[ ]" };
                body.push_str(&format!(
                    "{}. {} {} - {}\n",
                    i + 1,
                    checkbox,
                    subtask.title,
                    subtask.status
                ));
            }
            
            // Reference separate subtask issues
            if !separate_subtasks.is_empty() {
                body.push_str("\n### Complex Subtasks (Separate Issues)\n");
                for subtask in separate_subtasks {
                    body.push_str(&format!(
                        "- {} _(will be created as separate issue)_\n",
                        subtask.title
                    ));
                }
            }
        }

        body
    }
    
    /// Determines if a subtask should get its own GitHub issue
    fn should_create_separate_subtask_issue(&self, subtask: &Task) -> bool {
        // Don't create separate issues for very simple subtasks
        if subtask.description.len() < self.subtask_config.complexity_threshold {
            return false;
        }
        
        // Create separate issue if subtask has its own subtasks
        if self.subtask_config.create_separate_if_has_subtasks && !subtask.subtasks.is_empty() {
            return true;
        }
        
        // Create separate issue if subtask has an assignee
        if self.subtask_config.create_separate_if_has_assignee && subtask.assignee.is_some() {
            return true;
        }
        
        // Create separate issue if subtask is complex
        if self.subtask_config.create_separate_if_complex {
            // Consider it complex if it has details or test strategy
            if subtask.details.is_some() || subtask.test_strategy.is_some() {
                return true;
            }
            
            // Or if description is long
            if subtask.description.len() > self.subtask_config.complexity_threshold {
                return true;
            }
        }
        
        false
    }

    /// Formats field value for GitHub API
    fn format_field_value(&self, field_name: &str, value: Value) -> Value {
        let value_str = value.as_str().unwrap_or("");

        // Simple field value formatting based on known field names
        match field_name {
            "TM_ID" | "Dependencies" | "Test Strategy" | "Assignee" => {
                serde_json::json!({ "text": value_str })
            }
            "Priority" | "Agent" | "Status" => {
                // For single select fields, we need proper option lookup
                // For now, fallback to text format since we don't have option IDs
                serde_json::json!({ "text": value_str })
            }
            _ => serde_json::json!({ "text": value_str }),
        }
    }
    
    /// Enhanced field value formatting with option ID lookup for single select fields
    async fn format_field_value_enhanced(
        &mut self,
        field_name: &str,
        value: Value,
        project_id: &str,
    ) -> Result<Value> {
        let value_str = value.as_str().unwrap_or("");
        
        if value_str.is_empty() {
            return Ok(serde_json::json!({ "text": "" }));
        }

        // Check if this is a single select field that needs option ID
        match field_name {
            "Priority" | "Status" => {
                // Try to get or create the option ID
                match self.fields.ensure_option_exists(
                    &self.github,
                    project_id,
                    field_name,
                    value_str,
                ).await {
                    Ok(option_id) => {
                        Ok(serde_json::json!({
                            "singleSelectOptionId": option_id
                        }))
                    }
                    Err(_) => {
                        // Fallback to text if option creation fails
                        Ok(serde_json::json!({ "text": value_str }))
                    }
                }
            }
            _ => {
                // Text fields
                Ok(serde_json::json!({ "text": value_str }))
            }
        }
    }

    /// Extracts TM_ID from GitHub item
    fn extract_tm_id(&self, item: &ProjectItem) -> Option<String> {
        for field_value in &item.field_values {
            if field_value.field.name == "TM_ID" {
                match &field_value.value {
                    FieldValueContent::Text(tm_id) => return Some(tm_id.clone()),
                    _ => continue,
                }
            }
        }
        None
    }

    /// Syncs tasks from GitHub
    async fn sync_from_github(&mut self, _tag: &str, _options: &SyncOptions) -> Result<SyncResult> {
        // TODO: Implement sync from GitHub to TaskMaster
        Err(TaskMasterError::NotImplemented(
            "Sync from GitHub not yet implemented".to_string(),
        ))
    }

    /// Performs bidirectional sync
    async fn sync_bidirectional(
        &mut self,
        _tag: &str,
        _options: &SyncOptions,
    ) -> Result<SyncResult> {
        // TODO: Implement bidirectional sync
        Err(TaskMasterError::NotImplemented(
            "Bidirectional sync not yet implemented".to_string(),
        ))
    }

    /// Validates sync prerequisites
    async fn validate_sync_setup(&self) -> Result<()> {
        // Check if we have a project
        if self.project.is_none() {
            return Err(TaskMasterError::ConfigError(
                "No project configured".to_string(),
            ));
        }

        // Verify GitHub authentication
        // The GitHub API client already handles this

        Ok(())
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    AlwaysTaskMaster,
    AlwaysGitHub,
    Interactive,
    ByTimestamp,
}

/// Default sync options
impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            force: false,
            direction: SyncDirection::ToGitHub,
            batch_size: 50,
            include_archived: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_engine() {
        // TODO: Test sync engine initialization
    }

    #[tokio::test]
    async fn test_conflict_detection() {
        // TODO: Test conflict detection logic
    }

    #[tokio::test]
    async fn test_batch_operations() {
        // TODO: Test batch create/update/delete
    }
}
