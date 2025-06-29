//! Field management and mapping for TaskMaster to GitHub project fields
//!
//! This module handles:
//! - Mapping TaskMaster fields to GitHub project fields
//! - Field type conversions
//! - Custom field creation and management
//! - Field validation and formatting

use crate::error::{Result, TaskMasterError};
use crate::github::GitHubAPI;
use crate::models::github::{CustomField, GitHubFieldType};
use crate::models::task::Task;
use serde_json::Value;
use std::collections::HashMap;

/// Manages field mappings between TaskMaster and GitHub
pub struct FieldManager {
    field_mappings: HashMap<String, FieldMapping>,
    github_fields: HashMap<String, CustomField>,
    required_fields: Vec<RequiredField>,
}

/// Represents a mapping between TaskMaster and GitHub fields
#[derive(Debug, Clone)]
pub struct FieldMapping {
    pub taskmaster_field: String,
    pub github_field: String,
    pub field_type: GitHubFieldType,
    pub transformer: Option<FieldTransformer>,
}

/// Field transformation functions
#[derive(Debug, Clone)]
pub enum FieldTransformer {
    StatusMapper,
    PriorityMapper,
    DateFormatter,
    Custom(String),
}

/// Required custom fields for TaskMaster sync
#[derive(Debug, Clone)]
pub struct RequiredField {
    pub name: &'static str,
    pub field_type: GitHubFieldType,
    pub description: &'static str,
}

impl FieldManager {
    /// Creates a new field manager with default mappings
    pub fn new() -> Self {
        let mut manager = Self {
            field_mappings: HashMap::new(),
            github_fields: HashMap::new(),
            required_fields: vec![
                RequiredField {
                    name: "TM_ID",
                    field_type: GitHubFieldType::Text,
                    description: "TaskMaster task ID",
                },
                RequiredField {
                    name: "Dependencies",
                    field_type: GitHubFieldType::Text,
                    description: "Comma-separated list of dependency task IDs",
                },
                RequiredField {
                    name: "Test Strategy",
                    field_type: GitHubFieldType::Text,
                    description: "Testing approach for the task",
                },
                RequiredField {
                    name: "Priority",
                    field_type: GitHubFieldType::SingleSelect,
                    description: "Task priority level",
                },
                RequiredField {
                    name: "Agent",
                    field_type: GitHubFieldType::SingleSelect,
                    description: "Assigned agent/service",
                },
            ],
        };

        // Add default field mappings
        manager.add_default_mappings();
        manager
    }

    /// Adds default field mappings
    fn add_default_mappings(&mut self) {
        // Map TaskMaster ID
        self.field_mappings.insert(
            "id".to_string(),
            FieldMapping {
                taskmaster_field: "id".to_string(),
                github_field: "TM_ID".to_string(),
                field_type: GitHubFieldType::Text,
                transformer: None,
            },
        );

        // Map status
        self.field_mappings.insert(
            "status".to_string(),
            FieldMapping {
                taskmaster_field: "status".to_string(),
                github_field: "Status".to_string(),
                field_type: GitHubFieldType::SingleSelect,
                transformer: Some(FieldTransformer::StatusMapper),
            },
        );

        // Map priority
        self.field_mappings.insert(
            "priority".to_string(),
            FieldMapping {
                taskmaster_field: "priority".to_string(),
                github_field: "Priority".to_string(),
                field_type: GitHubFieldType::SingleSelect,
                transformer: Some(FieldTransformer::PriorityMapper),
            },
        );

        // Map dependencies
        self.field_mappings.insert(
            "dependencies".to_string(),
            FieldMapping {
                taskmaster_field: "dependencies".to_string(),
                github_field: "Dependencies".to_string(),
                field_type: GitHubFieldType::Text,
                transformer: None,
            },
        );

        // Map test strategy
        self.field_mappings.insert(
            "testStrategy".to_string(),
            FieldMapping {
                taskmaster_field: "testStrategy".to_string(),
                github_field: "Test Strategy".to_string(),
                field_type: GitHubFieldType::Text,
                transformer: None,
            },
        );
        
        // Map assignee
        self.field_mappings.insert(
            "assignee".to_string(),
            FieldMapping {
                taskmaster_field: "assignee".to_string(),
                github_field: "Assignee".to_string(),
                field_type: GitHubFieldType::Text,
                transformer: None,
            },
        );
    }

    /// Initializes field mappings from configuration
    pub fn init_mappings(&mut self, mappings: HashMap<String, String>) -> Result<()> {
        for (tm_field, gh_field) in mappings {
            // Determine field type based on field name
            let field_type = match gh_field.as_str() {
                "Status" | "Priority" | "Agent" => GitHubFieldType::SingleSelect,
                _ => GitHubFieldType::Text,
            };

            self.field_mappings.insert(
                tm_field.clone(),
                FieldMapping {
                    taskmaster_field: tm_field,
                    github_field: gh_field,
                    field_type,
                    transformer: None,
                },
            );
        }
        Ok(())
    }

    /// Maps TaskMaster task fields to GitHub project item fields
    pub fn map_task_to_github(&self, task: &Task) -> Result<HashMap<String, Value>> {
        let mut github_fields = HashMap::new();

        // Map ID
        if let Some(mapping) = self.field_mappings.get("id") {
            github_fields.insert(
                mapping.github_field.clone(),
                Value::String(task.id.to_string()),
            );
        }

        // Map status with option ID lookup
        if let Some(mapping) = self.field_mappings.get("status") {
            let status_value = if let Some(FieldTransformer::StatusMapper) = &mapping.transformer {
                self.transform_status(&task.status)?
            } else {
                task.status.clone()
            };
            github_fields.insert(mapping.github_field.clone(), Value::String(status_value));
        }

        // Map priority with option ID lookup
        if let Some(mapping) = self.field_mappings.get("priority") {
            if let Some(priority) = &task.priority {
                let priority_value = if let Some(FieldTransformer::PriorityMapper) = &mapping.transformer {
                    self.transform_priority(priority)?
                } else {
                    priority.clone()
                };
                github_fields.insert(mapping.github_field.clone(), Value::String(priority_value));
            }
        }
        
        // Note: Assignee is handled differently - it's set directly on the GitHub issue,
        // not as a custom field in the project. This is handled in the GitHub API calls.

        // Map dependencies
        if let Some(mapping) = self.field_mappings.get("dependencies") {
            let deps = task
                .dependencies
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(",");
            github_fields.insert(mapping.github_field.clone(), Value::String(deps));
        }

        // Map test strategy
        if let Some(mapping) = self.field_mappings.get("testStrategy") {
            if let Some(test_strategy) = &task.test_strategy {
                github_fields.insert(
                    mapping.github_field.clone(),
                    Value::String(test_strategy.clone()),
                );
            }
        }

        Ok(github_fields)
    }

    /// Maps GitHub project item fields to TaskMaster task
    pub fn map_github_to_task(&self, _github_fields: &HashMap<String, Value>) -> Result<Task> {
        // This would be used for bidirectional sync
        todo!("Implement GitHub to TaskMaster mapping when needed")
    }

    /// Creates or updates GitHub project fields
    pub async fn sync_fields_to_github(
        &self,
        github_api: &GitHubAPI,
        project_id: &str,
    ) -> Result<()> {
        // Get existing fields from GitHub
        let existing_fields = github_api.get_project_fields(project_id).await?;

        // Create a map of existing fields by name
        let existing_map: HashMap<String, &CustomField> = existing_fields
            .iter()
            .map(|f| (f.name.clone(), f))
            .collect();

        // Check and create required fields
        for required_field in &self.required_fields {
            if !existing_map.contains_key(required_field.name) {
                // Field doesn't exist, create it
                let field_type = match required_field.field_type {
                    GitHubFieldType::Text => "TEXT",
                    GitHubFieldType::SingleSelect => "SINGLE_SELECT",
                    GitHubFieldType::Number => "NUMBER",
                    GitHubFieldType::Date => "DATE",
                    GitHubFieldType::Iteration => "ITERATION",
                };

                github_api
                    .create_custom_field(project_id, required_field.name, field_type)
                    .await?;
            }
        }

        Ok(())
    }

    /// Validates field compatibility
    pub fn validate_field_mapping(&self, mapping: &FieldMapping) -> Result<()> {
        // Check if the field types are compatible
        match (&mapping.field_type, &mapping.transformer) {
            (GitHubFieldType::SingleSelect, Some(FieldTransformer::StatusMapper)) => Ok(()),
            (GitHubFieldType::SingleSelect, Some(FieldTransformer::PriorityMapper)) => Ok(()),
            (GitHubFieldType::Text, None) => Ok(()),
            (GitHubFieldType::Text, Some(FieldTransformer::DateFormatter)) => Ok(()),
            (GitHubFieldType::Number, None) => Ok(()),
            (GitHubFieldType::Date, None) => Ok(()),
            (GitHubFieldType::Iteration, None) => Ok(()),
            _ => Err(TaskMasterError::InvalidTaskFormat(format!(
                "Incompatible field type and transformer for field: {}",
                mapping.taskmaster_field
            ))),
        }
    }

    /// Gets all available GitHub fields
    pub fn github_fields(&self) -> Vec<CustomField> {
        self.github_fields.values().cloned().collect()
    }

    /// Updates the list of GitHub fields
    pub fn set_github_fields(&mut self, fields: Vec<CustomField>) {
        self.github_fields = fields
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();
    }
    
    /// Gets the option ID for a single select field value
    pub fn get_option_id(&self, field_name: &str, option_name: &str) -> Option<String> {
        if let Some(field) = self.github_fields.get(field_name) {
            if let Some(options) = &field.options {
                for option in options {
                    if option.name.to_lowercase() == option_name.to_lowercase() {
                        return Some(option.id.clone());
                    }
                }
            }
        }
        None
    }
    
    /// Creates option if it doesn't exist for a single select field
    pub async fn ensure_option_exists(
        &mut self,
        github_api: &GitHubAPI,
        project_id: &str,
        field_name: &str,
        option_name: &str,
    ) -> Result<String> {
        // First check if option already exists
        if let Some(option_id) = self.get_option_id(field_name, option_name) {
            return Ok(option_id);
        }
        
        // Option doesn't exist, create it
        if let Some(field) = self.github_fields.get(field_name) {
            let option_id = github_api.create_field_option(
                project_id,
                &field.id,
                option_name,
                "#f0f0f0" // Default color
            ).await?;
            
            // Refresh the field definition to include new option
            let updated_fields = github_api.get_project_fields(project_id).await?;
            self.set_github_fields(updated_fields);
            
            Ok(option_id)
        } else {
            Err(crate::error::TaskMasterError::ConfigError(
                format!("Field '{}' not found", field_name)
            ))
        }
    }

    /// Adds a custom field mapping
    pub fn add_custom_mapping(&mut self, mapping: FieldMapping) -> Result<()> {
        self.validate_field_mapping(&mapping)?;
        self.field_mappings
            .insert(mapping.taskmaster_field.clone(), mapping);
        Ok(())
    }

    /// Transform status values with QA workflow
    fn transform_status(&self, status: &str) -> Result<String> {
        Ok(match status.to_lowercase().as_str() {
            "pending" => "To Do".to_string(),
            "in-progress" => "In Progress".to_string(),
            // Key change: done becomes QA Review instead of Done
            // Only manual human intervention can set Done status
            "done" | "completed" => "QA Review".to_string(),
            "blocked" => "Blocked".to_string(),
            _ => status.to_string(),
        })
    }

    /// Transform priority values
    fn transform_priority(&self, priority: &str) -> Result<String> {
        Ok(match priority.to_lowercase().as_str() {
            "high" => "🔴 High".to_string(),
            "medium" => "🟡 Medium".to_string(),
            "low" => "🟢 Low".to_string(),
            _ => priority.to_string(),
        })
    }

    /// Gets field mapping for a TaskMaster field
    pub fn get_mapping(&self, taskmaster_field: &str) -> Option<&FieldMapping> {
        self.field_mappings.get(taskmaster_field)
    }

    /// Gets the GitHub field ID for a field name
    pub fn get_github_field_id(&self, field_name: &str) -> Option<String> {
        self.github_fields
            .get(field_name)
            .map(|f| f.id.clone())
    }
}

/// Default field mappings
impl Default for FieldManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::Task;

    #[test]
    fn test_field_mapping() {
        let manager = FieldManager::new();

        // Test default mappings exist
        assert!(manager.get_mapping("id").is_some());
        assert!(manager.get_mapping("status").is_some());
        assert!(manager.get_mapping("priority").is_some());
        assert!(manager.get_mapping("dependencies").is_some());
        assert!(manager.get_mapping("testStrategy").is_some());
    }

    #[test]
    fn test_value_transformation() {
        let manager = FieldManager::new();

        // Test status transformation
        assert_eq!(manager.transform_status("pending").unwrap(), "Todo");
        assert_eq!(
            manager.transform_status("in-progress").unwrap(),
            "In Progress"
        );
        assert_eq!(manager.transform_status("done").unwrap(), "Done");

        // Test priority transformation
        assert_eq!(manager.transform_priority("high").unwrap(), "🔴 High");
        assert_eq!(manager.transform_priority("medium").unwrap(), "🟡 Medium");
        assert_eq!(manager.transform_priority("low").unwrap(), "🟢 Low");
    }

    #[test]
    fn test_task_to_github_mapping() {
        let manager = FieldManager::new();

        let task = Task {
            id: "1".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            status: "pending".to_string(),
            priority: Some("high".to_string()),
            dependencies: vec!["2".to_string(), "3".to_string()],
            subtasks: vec![],
            test_strategy: Some("Unit tests".to_string()),
            details: Some("".to_string()),
            assignee: None,
        };

        let mapped_fields = manager.map_task_to_github(&task).unwrap();

        // Check mapped values
        assert_eq!(
            mapped_fields.get("TM_ID").unwrap(),
            &Value::String("1".to_string())
        );
        assert_eq!(
            mapped_fields.get("Status").unwrap(),
            &Value::String("Todo".to_string())
        );
        assert_eq!(
            mapped_fields.get("Priority").unwrap(),
            &Value::String("🔴 High".to_string())
        );
        assert_eq!(
            mapped_fields.get("Dependencies").unwrap(),
            &Value::String("2,3".to_string())
        );
        assert_eq!(
            mapped_fields.get("Test Strategy").unwrap(),
            &Value::String("Unit tests".to_string())
        );
    }

    #[test]
    fn test_custom_mapping() {
        let mut manager = FieldManager::new();

        let custom_mapping = FieldMapping {
            taskmaster_field: "complexity".to_string(),
            github_field: "Story Points".to_string(),
            field_type: GitHubFieldType::Number,
            transformer: None,
        };

        manager.add_custom_mapping(custom_mapping.clone()).unwrap();

        let retrieved = manager.get_mapping("complexity").unwrap();
        assert_eq!(retrieved.github_field, "Story Points");
    }
}
