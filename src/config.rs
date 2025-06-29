//! Configuration management for TaskMaster GitHub sync
//!
//! This module handles:
//! - Loading and saving configuration from .taskmaster/sync-config.json
//! - Managing GitHub project settings
//! - Field mapping configurations
//! - Sync preferences and options

use crate::error::{Result, TaskMasterError};
use crate::models::config::{ProjectMapping, SubtaskMode, SyncConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Configuration manager for TaskMaster sync
pub struct ConfigManager {
    config_path: PathBuf,
    config: SyncConfig,
}

impl ConfigManager {
    /// Creates a new configuration manager
    pub fn new(config_path: impl AsRef<Path>) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            config: SyncConfig::default(),
        }
    }

    /// Loads configuration from disk
    pub async fn load(&mut self) -> Result<()> {
        // Check if config file exists
        if !self.config_path.exists() {
            // Create default config if it doesn't exist
            self.save().await?;
            return Ok(());
        }

        // Read the config file
        let content = fs::read_to_string(&self.config_path).await.map_err(|e| {
            TaskMasterError::ConfigError(format!("Failed to read config file: {}", e))
        })?;

        // Parse the JSON
        self.config = serde_json::from_str(&content).map_err(|e| {
            TaskMasterError::ConfigError(format!("Failed to parse config JSON: {}", e))
        })?;

        Ok(())
    }

    /// Saves configuration to disk
    pub async fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                TaskMasterError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        // Serialize config to pretty JSON
        let content = serde_json::to_string_pretty(&self.config).map_err(|e| {
            TaskMasterError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        // Write to file
        fs::write(&self.config_path, content).await.map_err(|e| {
            TaskMasterError::ConfigError(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// Gets the current configuration
    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    /// Gets mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut SyncConfig {
        &mut self.config
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<()> {
        // Check organization is set
        if self.config.organization.is_empty() {
            return Err(TaskMasterError::ConfigError(
                "Organization name is required".to_string(),
            ));
        }

        // Validate project mappings
        for (tag, mapping) in &self.config.project_mappings {
            if mapping.project_id.is_empty() {
                return Err(TaskMasterError::ConfigError(format!(
                    "Project ID is missing for tag: {}",
                    tag
                )));
            }
            if mapping.project_number <= 0 {
                return Err(TaskMasterError::ConfigError(format!(
                    "Invalid project number for tag: {}",
                    tag
                )));
            }
        }

        Ok(())
    }

    /// Adds or updates a project mapping
    pub fn add_project_mapping(&mut self, tag: &str, project_number: i32, project_id: String) {
        self.config.project_mappings.insert(
            tag.to_string(),
            ProjectMapping {
                project_number,
                project_id,
                repository: None,
                subtask_mode: SubtaskMode::default(),
                field_mappings: None,
            },
        );
    }


    /// Updates last sync time for a tag
    pub fn update_last_sync(&mut self, tag: &str) {
        self.config
            .last_sync
            .insert(tag.to_string(), chrono::Utc::now());
    }

    /// Gets field mapping configuration for a tag
    pub fn field_mappings(&self, tag: &str) -> Option<&HashMap<String, String>> {
        self.config
            .project_mappings
            .get(tag)
            .and_then(|m| m.field_mappings.as_ref())
    }

    /// Updates field mapping configuration
    pub fn update_field_mappings(&mut self, tag: &str, mappings: HashMap<String, String>) {
        if let Some(project) = self.config.project_mappings.get_mut(tag) {
            project.field_mappings = Some(mappings);
        }
    }
}

/// Default configuration values
impl Default for ConfigManager {
    fn default() -> Self {
        Self::new(".taskmaster/sync-config.json")
    }
}

/// Helper methods for ConfigManager
impl ConfigManager {
    /// Creates a config manager for a specific taskmaster directory
    pub fn for_project(taskmaster_dir: impl AsRef<Path>) -> Self {
        let config_path = taskmaster_dir.as_ref().join("sync-config.json");
        Self::new(config_path)
    }

    /// Checks if configuration exists
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Gets the organization name
    pub fn organization(&self) -> &str {
        &self.config.organization
    }

    /// Gets project mapping for a tag
    pub fn get_project_mapping(&self, tag: &str) -> Option<&ProjectMapping> {
        self.config.project_mappings.get(tag)
    }

    /// Sets the organization name
    pub fn set_organization(&mut self, org: String) {
        self.config.organization = org;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_load_save() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("sync-config.json");

        let mut manager = ConfigManager::new(&config_path);
        manager.set_organization("test-org".to_string());
        manager.add_project_mapping("master", 123, "PVT_123".to_string());

        // Save config
        manager.save().await.unwrap();
        assert!(config_path.exists());

        // Load config
        let mut loaded_manager = ConfigManager::new(&config_path);
        loaded_manager.load().await.unwrap();

        assert_eq!(loaded_manager.organization(), "test-org");
        assert_eq!(
            loaded_manager
                .get_project_mapping("master")
                .unwrap()
                .project_number,
            123
        );
    }

    #[test]
    fn test_config_validation() {
        let mut manager = ConfigManager::default();

        // Should fail - no organization
        assert!(manager.validate().is_err());

        // Set organization
        manager.set_organization("test-org".to_string());

        // Should pass now
        assert!(manager.validate().is_ok());

        // Add invalid project mapping
        manager.config_mut().project_mappings.insert(
            "invalid".to_string(),
            ProjectMapping {
                project_number: 0,
                project_id: "".to_string(),
                repository: None,
                subtask_mode: SubtaskMode::default(),
                field_mappings: None,
            },
        );

        // Should fail - invalid project mapping
        assert!(manager.validate().is_err());
    }

    #[test]
    fn test_field_mappings() {
        let mut manager = ConfigManager::default();
        manager.add_project_mapping("master", 123, "PVT_123".to_string());

        let mut mappings = HashMap::new();
        mappings.insert("tm_id".to_string(), "TM_ID".to_string());
        mappings.insert("dependencies".to_string(), "Dependencies".to_string());

        manager.update_field_mappings("master", mappings.clone());

        let retrieved = manager.field_mappings("master").unwrap();
        assert_eq!(retrieved.get("tm_id").unwrap(), "TM_ID");
        assert_eq!(retrieved.get("dependencies").unwrap(), "Dependencies");
    }
}
