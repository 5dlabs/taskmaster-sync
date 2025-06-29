//! Integration tests for the sync engine

use std::fs;
use std::path::PathBuf;
use task_master_sync::{
    error::Result,
    sync::{SyncDirection, SyncEngine, SyncOptions},
};
use tempfile::TempDir;

// GitHub test organization and project
const TEST_ORG: &str = "5dlabs";
const TEST_PROJECT_NUMBER: i32 = 9;

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_sync_engine_basic() -> Result<()> {
    println!("Testing basic sync engine functionality...");

    // Create temporary config
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("sync-config.json");

    // Create basic config with repository for real issues
    let config_content = format!(
        r#"{{
        "version": "1.0.0",
        "organization": "{}",
        "project_mappings": {{
            "test": {{
                "project_number": {},
                "project_id": "test-project",
                "repository": "{}/taskmaster-sync",
                "subtask_mode": "nested"
            }}
        }}
    }}"#,
        TEST_ORG, TEST_PROJECT_NUMBER, TEST_ORG
    );

    fs::write(&config_path, config_content)?;

    // Create test tasks file in the expected location
    let taskmaster_dir = PathBuf::from(".taskmaster/tasks");
    fs::create_dir_all(&taskmaster_dir)?;

    let tasks_file = taskmaster_dir.join("tasks.json");
    let tasks_content = r#"{
        "test": {
            "tasks": [
                {
                    "id": "sync-test-1",
                    "title": "Sync Engine Test Task 1",
                    "description": "Testing sync engine",
                    "status": "pending",
                    "priority": "high",
                    "dependencies": [],
                    "details": "This is a test task for sync engine",
                    "testStrategy": "Verify sync works",
                    "subtasks": [],
                    "assignee": null
                },
                {
                    "id": "sync-test-2",
                    "title": "Sync Engine Test Task 2",
                    "description": "Another sync test",
                    "status": "in-progress",
                    "priority": "medium",
                    "dependencies": ["sync-test-1"],
                    "details": null,
                    "testStrategy": null,
                    "subtasks": [
                        {
                            "id": "sub-1",
                            "title": "Subtask 1",
                            "description": "First subtask",
                            "status": "done",
                            "priority": null,
                            "dependencies": [],
                            "details": null,
                            "testStrategy": null,
                            "subtasks": [],
                            "assignee": null
                        }
                    ],
                    "assignee": null
                }
            ]
        }
    }"#;

    fs::write(&tasks_file, tasks_content)?;

    // Create sync engine
    println!("Creating sync engine...");
    let mut engine =
        SyncEngine::new(config_path.to_str().unwrap(), "test", TEST_PROJECT_NUMBER).await?;

    // Run sync
    println!("Running sync to GitHub...");
    let options = SyncOptions {
        dry_run: false,
        force: false,
        direction: SyncDirection::ToGitHub,
        batch_size: 50,
        include_archived: false,
        use_delta_sync: true,
    };

    let result = engine.sync("test", options.clone()).await?;

    // Verify results
    println!("\nSync Results:");
    println!("  Created: {}", result.stats.created);
    println!("  Updated: {}", result.stats.updated);
    println!("  Deleted: {}", result.stats.deleted);
    println!("  Skipped: {}", result.stats.skipped);
    println!("  Errors: {}", result.stats.errors.len());

    // The first run should create items
    assert!(result.stats.created > 0 || result.stats.updated > 0);
    assert_eq!(result.stats.errors.len(), 0);

    // Run sync again - should update, not create
    println!("\nRunning sync again to test updates...");
    let result2 = engine.sync("test", options.clone()).await?;

    println!("\nSecond Sync Results:");
    println!("  Created: {}", result2.stats.created);
    println!("  Updated: {}", result2.stats.updated);
    println!("  Deleted: {}", result2.stats.deleted);
    println!("  Skipped: {}", result2.stats.skipped);
    println!("  Errors: {}", result2.stats.errors.len());

    // Second run should update, not create
    assert_eq!(result2.stats.created, 0);

    println!("\n✅ Sync engine basic test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_sync_dry_run() -> Result<()> {
    println!("Testing sync engine dry run mode...");

    // Create temporary config
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("sync-config.json");

    let config_content = format!(
        r#"{{
        "version": "1.0.0",
        "organization": "{}",
        "project_mappings": {{}}
    }}"#,
        TEST_ORG
    );

    fs::write(&config_path, config_content)?;

    // Create test tasks
    let taskmaster_dir = PathBuf::from(".taskmaster/tasks");
    fs::create_dir_all(&taskmaster_dir)?;

    let tasks_file = taskmaster_dir.join("tasks.json");
    let tasks_content = r#"{
        "dry-run-test": {
            "tasks": [
                {
                    "id": "dry-run-1",
                    "title": "Dry Run Test Task",
                    "description": "Should not be created",
                    "status": "pending",
                    "priority": null,
                    "dependencies": [],
                    "details": null,
                    "testStrategy": null,
                    "subtasks": [],
                    "assignee": null
                }
            ]
        }
    }"#;

    fs::write(&tasks_file, tasks_content)?;

    // Create sync engine
    let mut engine = SyncEngine::new(
        config_path.to_str().unwrap(),
        "dry-run-test",
        TEST_PROJECT_NUMBER,
    )
    .await?;

    // Run sync in dry run mode
    let options = SyncOptions {
        dry_run: true,
        force: false,
        direction: SyncDirection::ToGitHub,
        batch_size: 50,
        include_archived: false,
        use_delta_sync: true,
    };

    let result = engine.sync("dry-run-test", options).await?;

    // In dry run mode, everything should be skipped
    assert_eq!(result.stats.created, 0);
    assert_eq!(result.stats.updated, 0);
    assert_eq!(result.stats.deleted, 0);
    assert_eq!(result.stats.skipped, 1);

    println!("✅ Dry run test passed!");
    Ok(())
}
