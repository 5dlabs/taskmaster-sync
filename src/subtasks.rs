//! Subtask handling and hierarchy management
//!
//! This module handles:
//! - Subtask creation and linking in GitHub
//! - Hierarchy representation in project items
//! - Parent-child relationship management
//! - Subtask-specific field handling

use crate::error::Result;
use crate::github::{CreateItemResult, GitHubAPI};
use crate::models::github::GitHubProjectItem;
use crate::models::task::Task;
use std::collections::HashMap;

/// Manages subtask relationships and hierarchy
pub struct SubtaskHandler {
    /// Maps parent task IDs to their child task IDs
    parent_child_map: HashMap<String, Vec<String>>,
    /// Maps task IDs to their GitHub item IDs
    github_item_map: HashMap<String, String>,
    /// Enhanced subtask mode configuration
    enhanced_mode: bool,
}

/// Represents a task hierarchy node
#[derive(Debug, Clone)]
pub struct TaskNode {
    pub task: Task,
    pub children: Vec<TaskNode>,
    pub github_item_id: Option<String>,
    pub parent_id: Option<String>,
    pub level: usize,
}

/// Configuration for subtask handling
#[derive(Debug, Clone)]
pub struct SubtaskConfig {
    /// Create separate issues for subtasks with these properties
    pub create_separate_if_has_subtasks: bool,
    pub create_separate_if_has_assignee: bool,
    pub create_separate_if_complex: bool,
    /// Minimum complexity threshold (based on description length, etc.)
    pub complexity_threshold: usize,
}

impl SubtaskHandler {
    /// Creates a new subtask handler
    pub fn new() -> Self {
        Self {
            parent_child_map: HashMap::new(),
            github_item_map: HashMap::new(),
            enhanced_mode: true,
        }
    }

    /// Creates a new subtask handler with enhanced mode disabled
    pub fn new_basic() -> Self {
        Self {
            parent_child_map: HashMap::new(),
            github_item_map: HashMap::new(),
            enhanced_mode: false,
        }
    }

    /// Checks if enhanced subtask handling is enabled
    pub fn is_enhanced_mode(&self) -> bool {
        self.enhanced_mode
    }

    /// Builds a task hierarchy from flat task list
    pub fn build_hierarchy(&self, tasks: Vec<Task>) -> Vec<TaskNode> {
        let mut nodes = Vec::new();

        for task in tasks {
            let node = TaskNode {
                level: 0, // Top-level tasks
                parent_id: None,
                task,
                children: Vec::new(),
                github_item_id: None,
            };
            nodes.push(node);
        }

        nodes
    }

    /// Flattens a task hierarchy into a list
    pub fn flatten_hierarchy(&self, nodes: Vec<TaskNode>) -> Vec<Task> {
        let mut tasks = Vec::new();

        for node in nodes {
            tasks.push(node.task.clone());
            // Recursively flatten children
            if !node.children.is_empty() {
                let child_tasks = self.flatten_hierarchy(node.children);
                tasks.extend(child_tasks);
            }
        }

        tasks
    }

    /// Processes subtasks for a task, creating separate issues if needed
    pub async fn process_subtasks(
        &mut self,
        task: &Task,
        parent_github_item_id: &str,
        github: &GitHubAPI,
        project_id: &str,
        repository: Option<&str>,
        config: &SubtaskConfig,
    ) -> Result<Vec<CreateItemResult>> {
        let mut results = Vec::new();

        if !self.enhanced_mode {
            return Ok(results);
        }

        for subtask in &task.subtasks {
            if self.should_create_separate_issue(subtask, config) {
                let result = self
                    .create_subtask_issue(task, subtask, github, project_id, repository)
                    .await?;

                // Record the relationship
                self.github_item_map
                    .insert(subtask.id.clone(), result.project_item_id.clone());
                self.parent_child_map
                    .entry(task.id.clone())
                    .or_insert_with(Vec::new)
                    .push(subtask.id.clone());

                results.push(result);
            }
        }

        Ok(results)
    }

    /// Creates a separate GitHub issue for a subtask
    async fn create_subtask_issue(
        &self,
        parent: &Task,
        subtask: &Task,
        github: &GitHubAPI,
        project_id: &str,
        repository: Option<&str>,
    ) -> Result<CreateItemResult> {
        // Build subtask title with parent context
        let title = format!("{} [{}]", subtask.title, parent.title);

        // Build subtask body with parent reference
        let mut body = subtask.description.clone();
        body.push_str(&format!("\n\n**Parent Task:** {}", parent.title));

        if let Some(details) = &subtask.details {
            body.push_str(&format!("\n\n## Details\n{}", details));
        }

        if let Some(test_strategy) = &subtask.test_strategy {
            body.push_str(&format!("\n\n## Test Strategy\n{}", test_strategy));
        }

        // Extract assignees
        let assignees = subtask.assignee.as_ref().map(|a| vec![a.clone()]);

        // Create the issue
        if let Some(repo) = repository {
            github
                .create_project_item_with_issue(project_id, repo, &title, &body, assignees)
                .await
        } else {
            github.create_project_item(project_id, &title, &body).await
        }
    }

    /// Determines if a subtask should get its own GitHub issue
    fn should_create_separate_issue(&self, subtask: &Task, config: &SubtaskConfig) -> bool {
        // Don't create separate issues for very simple subtasks
        if subtask.description.len() < config.complexity_threshold {
            return false;
        }

        // Create separate issue if subtask has its own subtasks
        if config.create_separate_if_has_subtasks && !subtask.subtasks.is_empty() {
            return true;
        }

        // Create separate issue if subtask has an assignee
        if config.create_separate_if_has_assignee && subtask.assignee.is_some() {
            return true;
        }

        // Create separate issue if subtask is complex
        if config.create_separate_if_complex {
            // Consider it complex if it has details or test strategy
            if subtask.details.is_some() || subtask.test_strategy.is_some() {
                return true;
            }

            // Or if description is long
            if subtask.description.len() > config.complexity_threshold {
                return true;
            }
        }

        false
    }

    /// Gets the parent task ID from a subtask ID
    pub fn get_parent_id(&self, task_id: &str) -> Option<String> {
        // Look through the parent-child mappings
        for (parent_id, child_ids) in &self.parent_child_map {
            if child_ids.contains(&task_id.to_string()) {
                return Some(parent_id.clone());
            }
        }
        None
    }

    /// Gets all child task IDs for a parent
    pub fn get_child_ids(&self, parent_id: &str) -> Vec<String> {
        self.parent_child_map
            .get(parent_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Checks if a task is a subtask
    pub fn is_subtask(&self, task_id: &str) -> bool {
        task_id.contains('.')
    }

    /// Gets the hierarchy level of a task
    pub fn get_task_level(&self, task_id: &str) -> usize {
        task_id.matches('.').count()
    }

    /// Formats task hierarchy for display
    pub fn format_hierarchy_field(&self, task: &Task) -> String {
        let child_count = task.subtasks.len();
        if child_count > 0 {
            format!("Parent task ({} subtasks)", child_count)
        } else if self.get_parent_id(&task.id).is_some() {
            "Subtask".to_string()
        } else {
            "Root task".to_string()
        }
    }

    /// Updates GitHub item with hierarchy information
    pub fn add_hierarchy_fields(
        &self,
        fields: &mut HashMap<String, serde_json::Value>,
        task: &Task,
    ) {
        // Add hierarchy level field
        let hierarchy_info = self.format_hierarchy_field(task);
        fields.insert("Hierarchy".to_string(), serde_json::json!(hierarchy_info));

        // Add parent reference if this is a subtask
        if let Some(parent_id) = self.get_parent_id(&task.id) {
            fields.insert("Parent Task".to_string(), serde_json::json!(parent_id));
        }

        // Add child count if this is a parent
        let child_count = task.subtasks.len();
        if child_count > 0 {
            fields.insert("Subtask Count".to_string(), serde_json::json!(child_count));
        }
    }

    /// Validates task hierarchy consistency
    pub fn validate_hierarchy(&self, tasks: &[Task]) -> Result<()> {
        // Check for circular references
        for task in tasks {
            let mut visited = std::collections::HashSet::new();
            if self.has_circular_reference(&task.id, &mut visited) {
                return Err(crate::error::TaskMasterError::InvalidTaskFormat(format!(
                    "Circular reference detected in task hierarchy starting with {}",
                    task.id
                )));
            }
        }

        Ok(())
    }

    /// Checks for circular references in the hierarchy
    fn has_circular_reference(
        &self,
        task_id: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        if visited.contains(task_id) {
            return true;
        }

        visited.insert(task_id.to_string());

        // Check all children
        if let Some(child_ids) = self.parent_child_map.get(task_id) {
            for child_id in child_ids {
                if self.has_circular_reference(child_id, visited) {
                    return true;
                }
            }
        }

        visited.remove(task_id);
        false
    }

    /// Gets the default subtask configuration
    pub fn default_config() -> SubtaskConfig {
        SubtaskConfig {
            create_separate_if_has_subtasks: true,
            create_separate_if_has_assignee: true,
            create_separate_if_complex: true,
            complexity_threshold: 100, // characters
        }
    }
}

/// Utility functions for subtask operations
mod utils {
    use super::*;

    /// Sorts tasks by hierarchy (parents before children)
    pub fn sort_by_hierarchy(tasks: &mut Vec<Task>) {
        todo!("Sort tasks so parents come before children")
    }

    /// Generates a visual tree representation
    pub fn format_task_tree(nodes: &[TaskNode]) -> String {
        todo!("Generate ASCII tree representation of tasks")
    }

    /// Finds all root tasks (no parent)
    pub fn find_root_tasks(tasks: &[Task]) -> Vec<&Task> {
        tasks.iter().filter(|t| !t.id.contains('.')).collect()
    }

    /// Finds all leaf tasks (no children)
    pub fn find_leaf_tasks<'a>(tasks: &'a [Task], all_ids: &[String]) -> Vec<&'a Task> {
        todo!("Find tasks with no children")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_building() {
        // TODO: Test hierarchy building from flat list
    }

    #[test]
    fn test_parent_child_relationships() {
        // TODO: Test parent-child relationship management
    }

    #[test]
    fn test_hierarchy_validation() {
        // TODO: Test hierarchy validation
    }
}
