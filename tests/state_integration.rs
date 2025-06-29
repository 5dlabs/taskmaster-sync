//! Integration tests for state tracking with GitHub synchronization

use task_master_sync::{error::Result, github::GitHubAPI, models::task::Task, state::StateTracker};

// GitHub test organization and project
const TEST_ORG: &str = "5dlabs";
const TEST_PROJECT_NUMBER: i32 = 9;

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_state_tracking_with_github() -> Result<()> {
    println!("Testing state tracking with GitHub integration...");

    // Create temporary state file
    let temp_dir = tempfile::TempDir::new()?;
    let state_file = temp_dir.path().join("test-state.json");

    // Initialize state tracker
    let tracker = StateTracker::new(&state_file).await?;

    // Create test tasks
    let tasks = vec![
        Task {
            id: "state-test-1".to_string(),
            title: "State Test Task 1".to_string(),
            description: "Testing state tracking".to_string(),
            status: "pending".to_string(),
            priority: Some("medium".to_string()),
            dependencies: vec![],
            details: None,
            test_strategy: Some("Verify state tracking works".to_string()),
            subtasks: vec![],
            assignee: None,
        },
        Task {
            id: "state-test-2".to_string(),
            title: "State Test Task 2".to_string(),
            description: "Another state tracking test".to_string(),
            status: "in-progress".to_string(),
            priority: Some("high".to_string()),
            dependencies: vec!["state-test-1".to_string()],
            details: None,
            test_strategy: None,
            subtasks: vec![],
            assignee: None,
        },
    ];

    // Initialize GitHub API
    let api = GitHubAPI::new(TEST_ORG.to_string());

    // Get project
    let project = api.get_project(TEST_PROJECT_NUMBER).await?;
    println!("Using test project: {} ({})", project.title, project.id);

    // Track which items we create for cleanup
    let mut created_items = Vec::new();

    // Test 1: Create new items and track state
    println!("\nTest 1: Creating new items...");
    for task in &tasks {
        if !tracker.is_synced(&task.id).await {
            let result = api
                .create_project_item(&project.id, &task.title, &task.description)
                .await?;

            // Record in state tracker
            tracker
                .record_synced(
                    &task.id,
                    &result.project_item_id,
                    Some(&result.draft_issue_id),
                    task,
                )
                .await?;

            created_items.push(result.project_item_id.clone());
            println!(
                "✓ Created and tracked: {} -> {}",
                task.id, result.project_item_id
            );
        }
    }

    // Save state
    tracker.save().await?;

    // Test 2: Verify items are marked as synced
    println!("\nTest 2: Verifying sync state...");
    for task in &tasks {
        assert!(tracker.is_synced(&task.id).await);
        let github_id = tracker.get_github_item_id(&task.id).await;
        assert!(github_id.is_some());
        println!("✓ Task {} is synced to {}", task.id, github_id.unwrap());
    }

    // Test 3: Load state from file and verify persistence
    println!("\nTest 3: Testing state persistence...");
    let tracker2 = StateTracker::new(&state_file).await?;
    for task in &tasks {
        assert!(tracker2.is_synced(&task.id).await);
        let metadata = tracker2.get_task_metadata(&task.id).await;
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.title, task.title);
        println!("✓ Persisted metadata for {}: {}", task.id, meta.title);
    }

    // Test 4: Test orphaned detection
    println!("\nTest 4: Testing orphaned detection...");
    let current_tasks = vec![tasks[0].clone()]; // Only first task remains
    let orphaned = tracker.find_orphaned_items(&current_tasks).await;
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0], "state-test-2");
    println!("✓ Correctly identified orphaned task: {}", orphaned[0]);

    // Test 5: Update task metadata
    println!("\nTest 5: Testing metadata updates...");
    let mut updated_task = tasks[0].clone();
    updated_task.status = "done".to_string();
    tracker
        .update_task_metadata(&updated_task.id, &updated_task)
        .await?;

    let metadata = tracker.get_task_metadata(&updated_task.id).await.unwrap();
    assert_eq!(metadata.status, "done");
    println!("✓ Updated task status to: {}", metadata.status);

    // Test 6: Get sync statistics
    println!("\nTest 6: Testing sync statistics...");
    let stats = tracker.get_stats().await;
    assert_eq!(stats.total_synced, 2);
    assert!(stats.last_sync.is_some());
    println!(
        "✓ Stats: {} tasks synced, last sync: {:?}",
        stats.total_synced, stats.last_sync
    );

    // Cleanup - delete created items
    println!("\nCleaning up test items...");
    for item_id in created_items {
        api.delete_project_item(&project.id, &item_id).await?;
        println!("✓ Deleted test item: {}", item_id);
    }

    println!("\n✅ All state tracking tests passed!");
    Ok(())
}

#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    println!("Testing batch state operations...");

    let temp_dir = tempfile::TempDir::new()?;
    let state_file = temp_dir.path().join("batch-state.json");
    let tracker = StateTracker::new(&state_file).await?;

    // Create batch updates
    let batch_updates = vec![
        (
            "batch-1".to_string(),
            "gh-batch-1".to_string(),
            None,
            Task {
                id: "batch-1".to_string(),
                title: "Batch Task 1".to_string(),
                description: "".to_string(),
                status: "pending".to_string(),
                priority: None,
                dependencies: vec![],
                details: None,
                test_strategy: None,
                subtasks: vec![],
                assignee: None,
            },
        ),
        (
            "batch-2".to_string(),
            "gh-batch-2".to_string(),
            Some("draft-batch-2".to_string()),
            Task {
                id: "batch-2".to_string(),
                title: "Batch Task 2".to_string(),
                description: "".to_string(),
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
            "batch-3".to_string(),
            "gh-batch-3".to_string(),
            None,
            Task {
                id: "batch-3".to_string(),
                title: "Batch Task 3".to_string(),
                description: "".to_string(),
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

    // Perform batch update
    tracker.batch_record_synced(batch_updates).await?;

    // Verify all items were recorded
    assert!(tracker.is_synced("batch-1").await);
    assert!(tracker.is_synced("batch-2").await);
    assert!(tracker.is_synced("batch-3").await);

    // Verify metadata
    let meta2 = tracker.get_task_metadata("batch-2").await.unwrap();
    assert_eq!(meta2.draft_issue_id, Some("draft-batch-2".to_string()));
    assert_eq!(meta2.status, "done");

    // Save and reload
    tracker.save().await?;
    let tracker2 = StateTracker::new(&state_file).await?;

    let synced_ids = tracker2.get_synced_ids().await;
    assert_eq!(synced_ids.len(), 3);

    println!("✅ Batch operations test passed!");
    Ok(())
}

#[tokio::test]
async fn test_state_removal() -> Result<()> {
    println!("Testing state removal operations...");

    let temp_dir = tempfile::TempDir::new()?;
    let state_file = temp_dir.path().join("removal-state.json");
    let tracker = StateTracker::new(&state_file).await?;

    // Add a task
    let task = Task {
        id: "remove-me".to_string(),
        title: "Task to Remove".to_string(),
        description: "".to_string(),
        status: "pending".to_string(),
        priority: None,
        dependencies: vec![],
        details: None,
        test_strategy: None,
        subtasks: vec![],
        assignee: None,
    };

    tracker
        .record_synced("remove-me", "gh-remove", None, &task)
        .await?;
    assert!(tracker.is_synced("remove-me").await);

    // Remove it
    tracker.remove_task("remove-me").await?;
    assert!(!tracker.is_synced("remove-me").await);
    assert!(tracker.get_github_item_id("remove-me").await.is_none());

    println!("✅ State removal test passed!");
    Ok(())
}
