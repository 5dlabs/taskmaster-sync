//! Integration tests for configuration management

use task_master_sync::config::ConfigManager;
use task_master_sync::models::config::SubtaskMode;
use tempfile::TempDir;

#[tokio::test]
async fn test_config_lifecycle() {
    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let taskmaster_dir = temp_dir.path().join(".taskmaster");
    tokio::fs::create_dir_all(&taskmaster_dir).await.unwrap();

    // Create config manager
    let mut config = ConfigManager::for_project(taskmaster_dir);

    // Configure for test
    config.set_organization("5dlabs".to_string());
    config.add_project_mapping("master", 9, "PVT_kwDOC8B7k84A8o3m".to_string());

    // Save configuration
    config.save().await.unwrap();

    // Load in new instance
    let mut loaded_config = ConfigManager::for_project(temp_dir.path().join(".taskmaster"));
    loaded_config.load().await.unwrap();

    // Verify
    assert_eq!(loaded_config.organization(), "5dlabs");

    let mapping = loaded_config.get_project_mapping("master").unwrap();
    assert_eq!(mapping.project_number, 9);
    assert_eq!(mapping.project_id, "PVT_kwDOC8B7k84A8o3m");
    assert!(matches!(mapping.subtask_mode, SubtaskMode::Nested));
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = ConfigManager::default();

    // Should fail without organization
    assert!(config.validate().is_err());

    // Set valid organization
    config.set_organization("test-org".to_string());
    config.add_project_mapping("main", 123, "PVT_123".to_string());

    // Should pass now
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_last_sync_tracking() {
    let mut config = ConfigManager::default();
    config.set_organization("test-org".to_string());

    // No last sync initially
    assert!(config.config().last_sync.get("master").is_none());

    // Update last sync
    config.update_last_sync("master");

    // Should have timestamp now
    assert!(config.config().last_sync.get("master").is_some());
}
