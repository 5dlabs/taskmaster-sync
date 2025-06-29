//! Main synchronization engine for TaskMaster to GitHub
//!
//! This module handles:
//! - Full and incremental sync operations
//! - Conflict resolution
//! - Batch optimization
//! - Two-way sync logic

use crate::config::ConfigManager;
use crate::delta::{DeltaSyncEngine, TaskChange};
use crate::error::{Result, TaskMasterError};
use crate::fields::FieldManager;
use crate::github::{CreateItemResult, GitHubAPI};
use crate::models::github::{FieldValueContent, Project, ProjectItem};
use crate::models::task::Task;
use crate::progress::{ProgressTracker, SyncStats};
use crate::state::StateTracker;
use crate::subtasks::{SubtaskConfig, SubtaskHandler};
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
    pub tag: String,
}

/// Sync operation options
#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub dry_run: bool,
    pub force: bool,
    pub direction: SyncDirection,
    pub batch_size: usize,
    pub include_archived: bool,
    pub use_delta_sync: bool,
    pub quiet: bool,
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
    pub project_number: i32,
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
        let state_file = PathBuf::from(".taskmaster").join(format!("sync-state-{tag}.json"));
        let state = StateTracker::new(state_file).await?;

        // Get or create project
        let project = if project_number == 0 {
            // Special case: 0 means auto-create
            tracing::info!("Auto-creating new project...");

            // Try to detect repository from GitHub Actions environment or git remote
            let detected_repository = Self::detect_repository();
            
            // Determine project title from tag and config
            let title = if let Some(mapping) = config.get_project_mapping(tag) {
                format!(
                    "TaskMaster - {} ({})",
                    mapping
                        .repository
                        .as_ref()
                        .map(|r| r.split('/').last().unwrap_or(tag))
                        .unwrap_or(tag),
                    tag
                )
            } else if let Some(ref repo) = detected_repository {
                format!("TaskMaster - {} ({})", repo.split('/').last().unwrap_or(tag), tag)
            } else {
                format!("TaskMaster Project - {}", tag)
            };

            // Use repository from config or detected
            let repository = config
                .get_project_mapping(tag)
                .and_then(|m| m.repository.as_ref())
                .map(|s| s.as_str())
                .or(detected_repository.as_deref());
            
            // Clone repository for later use
            let repository_clone = repository.map(|s| s.to_string());

            let created_project = github
                .create_project(
                    &title,
                    Some("Auto-created by taskmaster-sync GitHub Action"),
                    repository,
                )
                .await?;

            tracing::info!(
                "Created project '{}' with number #{}",
                created_project.title,
                created_project.number
            );
            if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                println!(
                    "🎉 Created new GitHub Project: {} (#{}) ",
                    created_project.title, created_project.number
                );
            }
            if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                println!("   URL: {}", created_project.url);
            }

            // Set up required fields
            Self::setup_project_fields(&github, &created_project.id).await?;

            // Update config with the new project number and repository
            let needs_new_mapping = config.get_project_mapping(tag).is_none();
            
            if needs_new_mapping {
                // Create new mapping if it doesn't exist
                let new_mapping = crate::models::config::ProjectMapping {
                    project_number: created_project.number,
                    project_id: created_project.id.clone(),
                    repository: repository_clone.clone(),
                    subtask_mode: crate::models::config::SubtaskMode::Nested,
                    field_mappings: None,
                };
                config.add_project_mapping(tag.to_string(), new_mapping);
            } else {
                // Update existing mapping
                if let Some(mapping) = config.get_project_mapping_mut(tag) {
                    mapping.project_number = created_project.number;
                    mapping.project_id = created_project.id.clone();
                    // Save repository if it was detected and not already set
                    if mapping.repository.is_none() && repository_clone.is_some() {
                        mapping.repository = repository_clone.clone();
                    }
                }
            }
            config.save().await?;
            if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                println!(
                    "   ✅ Updated config with project number: {}",
                    created_project.number
                );
            }

            created_project
        } else {
            // Try to get existing project
            match github.get_project(project_number).await {
                Ok(p) => {
                    tracing::info!("Found existing project #{}", project_number);
                    p
                }
                Err(e) => {
                    // Check if we should auto-create
                    if std::env::var("TASKMASTER_AUTO_CREATE_PROJECT").unwrap_or_default() == "true"
                    {
                        tracing::info!(
                            "Project #{} not found, auto-creating new project...",
                            project_number
                        );

                        // Try to detect repository
                        let detected_repository = Self::detect_repository();
                        
                        let title = if let Some(ref repo) = detected_repository {
                            format!("TaskMaster - {} ({})", repo.split('/').last().unwrap_or(tag), tag)
                        } else {
                            format!("TaskMaster Project - {}", tag)
                        };
                        
                        // Use repository from config or detected
                        let repository = config
                            .get_project_mapping(tag)
                            .and_then(|m| m.repository.as_ref())
                            .map(|s| s.as_str())
                            .or(detected_repository.as_deref());
                        
                        let created_project = github
                            .create_project(
                                &title,
                                Some("Auto-created by taskmaster-sync"),
                                repository,
                            )
                            .await?;

                        tracing::info!(
                            "Created project '{}' with number #{}",
                            created_project.title,
                            created_project.number
                        );
                        if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                            println!(
                                "🎉 Created new GitHub Project: {} (#{})",
                                created_project.title, created_project.number
                            );
                        }
                        if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                            println!(
                                "   Note: Requested project #{} was not available",
                                project_number
                            );
                        }

                        // Set up required fields
                        Self::setup_project_fields(&github, &created_project.id).await?;

                        created_project
                    } else {
                        return Err(TaskMasterError::ConfigError(format!(
                            "Project #{} not found. To auto-create a project:\n\
                            \n\
                            Option 1: Use project number '0' to auto-create:\n\
                               task-master-sync sync {} 0\n\
                            \n\
                            Option 2: Set environment variable:\n\
                               export TASKMASTER_AUTO_CREATE_PROJECT=true\n\
                            \n\
                            Option 3: Create manually:\n\
                               task-master-sync create-project 'Project Name' --org {}\n\
                               task-master-sync setup-project <PROJECT_NUMBER>",
                            project_number, tag, org
                        )));
                    }
                }
            }
        };

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
            tag: tag.to_string(),
        })
    }

    /// Performs a full synchronization
    pub async fn sync(&mut self, tag: &str, options: SyncOptions) -> Result<SyncResult> {
        // Validate setup
        self.validate_sync_setup()?;

        // Run appropriate sync based on direction
        match options.direction {
            SyncDirection::ToGitHub => self.sync_to_github(tag, &options).await,
            SyncDirection::FromGitHub => self.sync_from_github(tag, &options),
            SyncDirection::Bidirectional => self.sync_bidirectional(tag, &options),
        }
    }

    /// Syncs tasks to GitHub
    async fn sync_to_github(&mut self, tag: &str, options: &SyncOptions) -> Result<SyncResult> {
        let start_time = std::time::Instant::now();
        let project = self.project.as_ref().unwrap();
        let project_id = project.id.clone(); // Extract to avoid borrow issues

        // Load tasks for the tag
        let all_tasks = self.taskmaster.load_tasks().await?;
        let tasks = all_tasks
            .get(tag)
            .ok_or_else(|| TaskMasterError::InvalidTaskFormat(format!("Tag '{tag}' not found")))?;
        let tasks_clone = tasks.clone(); // Clone for later use

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
        let mut title_to_github: HashMap<String, Vec<ProjectItem>> = HashMap::new();

        for item in github_items {
            // Extract TM_ID from field values
            if let Some(tm_id) = self.extract_tm_id(&item) {
                tm_id_to_github.insert(tm_id, item.clone());
            }

            // Also track by title for duplicate detection
            title_to_github
                .entry(item.title.clone())
                .or_default()
                .push(item);
        }

        // Track sync statistics
        let mut created = 0;
        let mut updated = 0;
        let mut deleted = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        // Determine which tasks to process based on sync mode
        let tasks_to_process: Vec<&Task> = if options.use_delta_sync && !options.force {
            // Use delta sync for performance
            let delta_engine = DeltaSyncEngine::new(tag);

            // Convert TaggedTasks to Vec<Task> format for delta engine
            let tasks_map: HashMap<String, Vec<Task>> = all_tasks
                .iter()
                .map(|(tag, tagged_tasks)| (tag.clone(), tagged_tasks.tasks.clone()))
                .collect();

            let change_set = delta_engine.detect_changes(&tasks_map, tag).await?;

            tracing::info!(
                "Delta sync detected {} changes out of {} total tasks",
                change_set.changes.len(),
                tasks_clone.tasks.len()
            );

            // Convert changes to task references
            let mut tasks_to_sync = Vec::new();
            for change in &change_set.changes {
                match change {
                    TaskChange::Added(task) | TaskChange::Modified(_, task) => {
                        if let Some(task_ref) = tasks_clone.tasks.iter().find(|t| t.id == task.id) {
                            tasks_to_sync.push(task_ref);
                        }
                    }
                    TaskChange::Removed(task) => {
                        // Handle removal separately
                        if let Some(github_item) = tm_id_to_github.get(&task.id) {
                            if !options.dry_run {
                                if let Err(e) = self
                                    .github
                                    .delete_project_item(&project_id, &github_item.id)
                                    .await
                                {
                                    errors.push(format!(
                                        "Failed to delete removed task {}: {e}",
                                        task.id
                                    ));
                                } else {
                                    deleted += 1;
                                    self.state.remove_task(&task.id).await?;
                                }
                            } else {
                                if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                                    println!("DRY RUN: Would delete removed task {}", task.id);
                                }
                                deleted += 1;
                            }
                        }
                    }
                }
            }
            tasks_to_sync
        } else {
            // Full sync - process all tasks
            tracing::info!("Performing full sync of {} tasks", tasks_clone.tasks.len());
            tasks_clone.tasks.iter().collect()
        };

        // Create progress tracker
        let progress = ProgressTracker::new(tasks_to_process.len());

        // Process tasks in batches
        for task in &tasks_to_process {
            progress.update_main(
                created + updated + skipped,
                &format!("Processing: {}", task.title),
            );

            if options.dry_run {
                if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                    println!("DRY RUN: Would process task {}: {}", task.id, task.title);
                }
                skipped += 1;
                continue;
            }

            // Check if task is already synced
            if let Some(github_item) = tm_id_to_github.get(&task.id) {
                // Update existing item
                if let Err(e) = self.update_github_item(task, github_item, &progress).await {
                    errors.push(format!("Failed to update {}: {e}", task.id));
                    progress
                        .record_error(format!("Error updating {}: {e}", task.id))
                        .await;
                } else {
                    updated += 1;
                    progress.record_updated(&task.id).await;
                    self.state.update_task_metadata(&task.id, task).await?;
                }
            } else {
                // Before creating, check if there's already an item with the same title (possible duplicate)
                if let Some(existing_items) = title_to_github.get(&task.title) {
                    if !existing_items.is_empty() {
                        tracing::warn!(
                            "Found {} existing items with title '{}' but no TM_ID match. Possible duplicates.",
                            existing_items.len(),
                            task.title
                        );

                        // Try to find the best match and update it instead of creating a new one
                        if existing_items.len() == 1 {
                            let existing = &existing_items[0];
                            tracing::info!(
                                "Updating existing item without TM_ID for task: {}",
                                task.id
                            );

                            // Update the existing item
                            if let Err(e) = self.update_github_item(task, existing, &progress).await
                            {
                                errors.push(format!("Failed to update duplicate {}: {e}", task.id));
                                progress
                                    .record_error(format!(
                                        "Error updating duplicate {}: {e}",
                                        task.id
                                    ))
                                    .await;
                            } else {
                                updated += 1;
                                progress.record_updated(&task.id).await;
                                self.state.update_task_metadata(&task.id, task).await?;

                                // Add to our mapping to prevent further duplicates in this run
                                tm_id_to_github.insert(task.id.clone(), existing.clone());
                            }
                            continue;
                        }
                        // Multiple duplicates - log warning but create new one
                        // In production, you might want to handle this differently
                        tracing::error!(
                            "Multiple duplicates ({}) found for '{}'. Creating new item anyway.",
                            existing_items.len(),
                            task.title
                        );
                    }
                }

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
                        errors.push(format!("Failed to create {}: {e}", task.id));
                        progress
                            .record_error(format!("Error creating {}: {}", task.id, e))
                            .await;
                    }
                }
            }
        }

        // Handle orphaned items (in GitHub but not in TaskMaster)
        // Only check for orphans in full sync mode (delta sync handles removals explicitly)
        if !options.use_delta_sync || options.force {
            let _current_task_ids: Vec<String> =
                tasks_clone.tasks.iter().map(|t| t.id.clone()).collect();
            let orphaned = self.state.find_orphaned_items(&tasks_clone.tasks).await;

            for orphan_id in orphaned {
                if let Some(github_item) = tm_id_to_github.get(&orphan_id) {
                    if !options.dry_run {
                        if let Err(e) = self
                            .github
                            .delete_project_item(&project_id, &github_item.id)
                            .await
                        {
                            errors.push(format!("Failed to delete orphaned item {orphan_id}: {e}"));
                        } else {
                            deleted += 1;
                            self.state.remove_task(&orphan_id).await?;
                        }
                    } else {
                        if std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
                            println!("DRY RUN: Would delete orphaned item {orphan_id}");
                        }
                    }
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

        if !errors.is_empty() && std::env::var("TASKMASTER_QUIET").unwrap_or_default() != "1" {
            eprintln!("\nSync completed with {} errors:", errors.len());
            for error in &errors {
                eprintln!("  - {error}");
            }
        }

        Ok(SyncResult {
            stats,
            conflicts: vec![],
            project_number: self.project.as_ref().map(|p| p.number).unwrap_or(0),
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

        // Determine GitHub assignee based on task status
        let github_assignee = self.fields.get_github_assignee(task);
        let assignees = github_assignee.map(|a| vec![a]);

        // Check if we should create a repository issue or draft issue
        let result = if let Some(mapping) = &self.project_mapping {
            if let Some(repository) = &mapping.repository {
                // Create repository issue and add to project
                self.github
                    .create_project_item_with_issue(
                        &project_id,
                        repository,
                        &task.title,
                        &body,
                        assignees,
                    )
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

        // Process subtasks - temporarily disabled for performance and to focus on main task sync
        // TODO: Re-enable optimized subtask processing after main task sync is perfected
        if false {
            let repository = self
                .project_mapping
                .as_ref()
                .and_then(|m| m.repository.as_deref());

            let _subtask_results = self
                .subtasks
                .process_subtasks(
                    task,
                    &result.project_item_id,
                    &self.github,
                    &project_id,
                    repository,
                    &self.subtask_config,
                )
                .await?;

            // TODO: Store subtask results in state for tracking
        }

        // Map task fields to GitHub fields
        let field_values = self.fields.map_task_to_github(task)?;

        // DISABLED FOR MVS: Add hierarchy fields
        // self.subtasks.add_hierarchy_fields(&mut field_values, task);

        // Track whether TM_ID was successfully set
        let mut tm_id_set = false;

        // Update each field
        for (field_name, value) in field_values {
            tracing::debug!("Processing field: {} = {:?}", field_name, value);
            // DEBUG: Processing field

            if let Some(field_id) = self.fields.get_github_field_id(&field_name) {
                tracing::debug!("Found field ID for {}: {}", field_name, field_id);
                // DEBUG: Found field ID

                // Format value based on field type with option ID lookup for single select
                let formatted_value = self
                    .format_field_value_enhanced(&field_name, value, &project_id)
                    .await?;

                tracing::debug!("Formatted value for {}: {:?}", field_name, formatted_value);

                match self
                    .github
                    .update_field_value(
                        &project_id,
                        &result.project_item_id,
                        &field_id,
                        formatted_value,
                    )
                    .await
                {
                    Ok(_) => {
                        tracing::debug!("Successfully updated field: {}", field_name);
                        // DEBUG: Successfully updated field
                        if field_name == "TM_ID" {
                            tm_id_set = true;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to update field {}: {}", field_name, e);
                        tracing::error!("Failed to update field {field_name}: {e}");
                    }
                }

                // Small delay to avoid rate limiting - reduced for performance
                sleep(Duration::from_millis(50)).await;
            } else {
                tracing::warn!(
                    "No field ID found for field: {} (available fields: {:?})",
                    field_name,
                    self.fields
                        .github_fields()
                        .iter()
                        .map(|f| &f.name)
                        .collect::<Vec<_>>()
                );
                tracing::warn!(
                    "No field ID found for field: {} (available fields: {:?})",
                    field_name,
                    self.fields
                        .github_fields()
                        .iter()
                        .map(|f| &f.name)
                        .collect::<Vec<_>>()
                );

                // Try to refresh GitHub fields and retry once
                let github_fields = self.github.get_project_fields(&project_id).await?;
                self.fields.set_github_fields(github_fields);

                if let Some(field_id) = self.fields.get_github_field_id(&field_name) {
                    tracing::info!(
                        "Found field ID after refresh for {}: {}",
                        field_name,
                        field_id
                    );

                    let formatted_value = self
                        .format_field_value_enhanced(&field_name, value, &project_id)
                        .await?;

                    match self
                        .github
                        .update_field_value(
                            &project_id,
                            &result.project_item_id,
                            &field_id,
                            formatted_value,
                        )
                        .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                "Successfully updated field after refresh: {}",
                                field_name
                            );
                            if field_name == "TM_ID" {
                                tm_id_set = true;
                            }
                        }
                        Err(e) => tracing::error!(
                            "Failed to update field {} after refresh: {}",
                            field_name,
                            e
                        ),
                    }

                    sleep(Duration::from_millis(50)).await;
                } else {
                    tracing::error!("Field {} not found even after refresh", field_name);
                    tracing::error!("Field {field_name} not found even after refresh");
                }
            }
        }

        // Critical: Ensure TM_ID was set, otherwise this item will become a duplicate
        if !tm_id_set {
            tracing::error!(
                "CRITICAL: Failed to set TM_ID for task '{}'. This will cause duplicates!",
                task.id
            );

            // Try one more time to set TM_ID
            if let Some(field_id) = self.fields.get_github_field_id("TM_ID") {
                tracing::warn!("Attempting emergency TM_ID update for task: {}", task.id);
                let tm_id_value = serde_json::json!({ "text": &task.id });

                if let Err(e) = self
                    .github
                    .update_field_value(
                        &project_id,
                        &result.project_item_id,
                        &field_id,
                        tm_id_value,
                    )
                    .await
                {
                    tracing::error!("Emergency TM_ID update failed: {}", e);

                    // Consider deleting the item to prevent duplicates
                    tracing::error!(
                        "WARNING: Item created without TM_ID. Consider manual cleanup for: {}",
                        task.title
                    );
                } else {
                    tracing::info!("Emergency TM_ID update succeeded for: {}", task.id);
                }
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

            // Update GitHub assignees based on task status (for repository issues)
            if let Some(github_assignee) = self.fields.get_github_assignee(task) {
                if let Err(e) = self
                    .github
                    .update_issue_assignees(&draft_id, vec![github_assignee.clone()])
                    .await
                {
                    tracing::debug!("Could not update assignees (might be draft issue): {}", e);
                    // This is expected for draft issues, only repository issues support assignees
                }
            }
        }

        // Update fields
        let field_values = self.fields.map_task_to_github(task)?;

        // DISABLED FOR MVS: Add hierarchy fields
        // self.subtasks.add_hierarchy_fields(&mut field_values, task);

        for (field_name, value) in field_values {
            tracing::debug!("Updating existing item field: {} = {:?}", field_name, value);

            if let Some(field_id) = self.fields.get_github_field_id(&field_name) {
                tracing::debug!(
                    "Found field ID for existing item {}: {}",
                    field_name,
                    field_id
                );

                let formatted_value = self
                    .format_field_value_enhanced(&field_name, value, &project_id)
                    .await?;

                match self
                    .github
                    .update_field_value(&project_id, &github_item.id, &field_id, formatted_value)
                    .await
                {
                    Ok(_) => {
                        tracing::debug!("Successfully updated existing item field: {}", field_name)
                    }
                    Err(e) => tracing::error!(
                        "Failed to update existing item field {}: {}",
                        field_name,
                        e
                    ),
                }

                sleep(Duration::from_millis(50)).await;
            } else {
                tracing::warn!(
                    "No field ID found for existing item field: {} (available fields: {:?})",
                    field_name,
                    self.fields
                        .github_fields()
                        .iter()
                        .map(|f| &f.name)
                        .collect::<Vec<_>>()
                );

                // Try to refresh GitHub fields and retry once
                let github_fields = self.github.get_project_fields(&project_id).await?;
                self.fields.set_github_fields(github_fields);

                if let Some(field_id) = self.fields.get_github_field_id(&field_name) {
                    tracing::info!(
                        "Found field ID after refresh for existing item {}: {}",
                        field_name,
                        field_id
                    );

                    let formatted_value = self
                        .format_field_value_enhanced(&field_name, value, &project_id)
                        .await?;

                    match self
                        .github
                        .update_field_value(
                            &project_id,
                            &github_item.id,
                            &field_id,
                            formatted_value,
                        )
                        .await
                    {
                        Ok(_) => tracing::info!(
                            "Successfully updated existing item field after refresh: {}",
                            field_name
                        ),
                        Err(e) => tracing::error!(
                            "Failed to update existing item field {} after refresh: {}",
                            field_name,
                            e
                        ),
                    }

                    sleep(Duration::from_millis(50)).await;
                } else {
                    tracing::error!(
                        "Existing item field {} not found even after refresh",
                        field_name
                    );
                }
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
            body.push_str(&format!("\n\n## Details\n{details}"));
        }

        if let Some(test_strategy) = &task.test_strategy {
            body.push_str(&format!("\n\n## Test Strategy\n{test_strategy}"));
        }

        if !task.subtasks.is_empty() {
            body.push_str("\n\n## Subtasks\n");

            let mut separate_subtasks = Vec::new();
            let mut inline_subtasks = Vec::new();

            // Separate subtasks into those getting separate issues vs inline
            for subtask in &task.subtasks {
                if self.subtasks.is_enhanced_mode()
                    && self.should_create_separate_subtask_issue(subtask)
                {
                    separate_subtasks.push(subtask);
                } else {
                    inline_subtasks.push(subtask);
                }
            }

            // Add inline subtasks as checklist
            for (i, subtask) in inline_subtasks.iter().enumerate() {
                let checkbox = if subtask.status == "done" {
                    "[x]"
                } else {
                    "[ ]"
                };
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
            "Priority" | "Status" | "Agent" => {
                // Try to get or create the option ID
                match self
                    .fields
                    .ensure_option_exists(&self.github, project_id, field_name, value_str)
                    .await
                {
                    Ok(option_id) => {
                        tracing::debug!(
                            "Created/found option ID for {}: {} = {}",
                            field_name,
                            value_str,
                            option_id
                        );
                        Ok(serde_json::json!({
                            "singleSelectOptionId": option_id
                        }))
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to create option for {} field '{}': {}",
                            field_name,
                            value_str,
                            e
                        );
                        Err(e)
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
                if let FieldValueContent::Text(tm_id) = &field_value.value {
                    return Some(tm_id.clone());
                }
            }
        }
        None
    }

    /// Syncs tasks from GitHub
    fn sync_from_github(&mut self, _tag: &str, _options: &SyncOptions) -> Result<SyncResult> {
        // TODO: Implement sync from GitHub to TaskMaster
        Err(TaskMasterError::NotImplemented(
            "Sync from GitHub not yet implemented".to_string(),
        ))
    }

    /// Performs bidirectional sync
    fn sync_bidirectional(&mut self, _tag: &str, _options: &SyncOptions) -> Result<SyncResult> {
        // TODO: Implement bidirectional sync
        Err(TaskMasterError::NotImplemented(
            "Bidirectional sync not yet implemented".to_string(),
        ))
    }

    /// Validates sync prerequisites
    fn validate_sync_setup(&self) -> Result<()> {
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

    /// Detects repository from environment or git configuration
    fn detect_repository() -> Option<String> {
        // First try GitHub Actions environment variable
        if let Ok(repository) = std::env::var("GITHUB_REPOSITORY") {
            tracing::info!("Detected repository from GITHUB_REPOSITORY: {}", repository);
            return Some(repository);
        }
        
        // Try to get from git remote
        if let Ok(output) = std::process::Command::new("git")
            .args(&["config", "--get", "remote.origin.url"])
            .output()
        {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // Parse GitHub URL formats
                if let Some(repo) = Self::parse_github_url(&url) {
                    tracing::info!("Detected repository from git remote: {}", repo);
                    return Some(repo);
                }
            }
        }
        
        None
    }
    
    /// Parses GitHub repository from various URL formats
    fn parse_github_url(url: &str) -> Option<String> {
        // Handle SSH format: git@github.com:owner/repo.git
        if url.starts_with("git@github.com:") {
            let parts: Vec<&str> = url.split(':').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim_end_matches(".git").to_string());
            }
        }
        
        // Handle HTTPS format: https://github.com/owner/repo.git
        if url.contains("github.com/") {
            let parts: Vec<&str> = url.split("github.com/").collect();
            if parts.len() == 2 {
                return Some(parts[1].trim_end_matches(".git").to_string());
            }
        }
        
        None
    }

    /// Sets up required fields for a newly created project
    async fn setup_project_fields(github_api: &GitHubAPI, project_id: &str) -> Result<()> {
        tracing::info!("Setting up required fields for project");

        // Initialize field manager
        let field_manager = FieldManager::new();

        // Create required custom fields
        field_manager
            .sync_fields_to_github(github_api, project_id)
            .await?;

        // Get the updated fields to find the Status field
        let fields = github_api.get_project_fields(project_id).await?;

        // Find the Status field and add QA Review option
        for field in fields {
            if field.name == "Status" && field.data_type == "SINGLE_SELECT" {
                if let Some(options) = &field.options {
                    let has_qa_review = options.iter().any(|opt| opt.name == "QA Review");

                    if !has_qa_review {
                        tracing::info!("Adding 'QA Review' option to Status field");
                        // Create the QA Review option (it will be inserted before "Done")
                        github_api
                            .create_field_option(project_id, &field.id, "QA Review", "YELLOW")
                            .await?;
                    }
                }
            }
        }

        tracing::info!("Project fields setup completed");
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
            use_delta_sync: true, // Default to delta sync for performance
            quiet: false,
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

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
