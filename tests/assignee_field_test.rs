//! Test to verify assignee field mapping to Agent field works correctly

use task_master_sync::fields::FieldManager;
use task_master_sync::models::task::Task;

#[test]
fn test_assignee_maps_to_agent_field() {
    let manager = FieldManager::new();

    // Verify the field mapping configuration
    let mapping = manager
        .get_mapping("assignee")
        .expect("assignee mapping should exist");

    assert_eq!(mapping.taskmaster_field, "assignee");
    assert_eq!(
        mapping.github_field, "Agent",
        "Should map to Agent, not Assignee"
    );
}

#[test]
fn test_task_with_assignee_creates_agent_field() {
    let manager = FieldManager::new();

    let task = Task {
        id: "test-123".to_string(),
        title: "Test Task".to_string(),
        description: "Test description".to_string(),
        status: "in-progress".to_string(),
        priority: Some("high".to_string()),
        dependencies: vec![],
        subtasks: vec![],
        test_strategy: None,
        details: None,
        assignee: Some("swe-1-5dlabs".to_string()),
    };

    let fields = manager
        .map_task_to_github(&task)
        .expect("mapping should succeed");

    // Verify Agent field is present with correct value
    assert!(
        fields.contains_key("Agent"),
        "Agent field should be present"
    );
    assert_eq!(
        fields.get("Agent").unwrap().as_str().unwrap(),
        "swe-1-5dlabs",
        "Agent field should contain the assignee value"
    );

    // Verify other expected fields
    assert!(fields.contains_key("TM_ID"));
    assert!(fields.contains_key("Status"));
}

#[test]
fn test_multiple_assignees_map_correctly() {
    let manager = FieldManager::new();

    let test_cases = vec![
        ("swe-1-5dlabs", "SWE-1 assignee"),
        ("swe-2-5dlabs", "SWE-2 assignee"),
        ("qa0-5dlabs", "QA assignee"),
        ("pm0-5dlabs", "PM assignee"),
    ];

    for (assignee, description) in test_cases {
        let task = Task {
            id: format!("test-{assignee}"),
            title: description.to_string(),
            description: "Test".to_string(),
            status: "pending".to_string(),
            priority: None,
            dependencies: vec![],
            subtasks: vec![],
            test_strategy: None,
            details: None,
            assignee: Some(assignee.to_string()),
        };

        let fields = manager
            .map_task_to_github(&task)
            .expect("mapping should succeed");

        assert_eq!(
            fields.get("Agent").unwrap().as_str().unwrap(),
            assignee,
            "Agent field should map {description} correctly"
        );
    }
}

#[test]
fn test_no_assignee_no_agent_field() {
    let manager = FieldManager::new();

    let task = Task {
        id: "test-no-assignee".to_string(),
        title: "Task without assignee".to_string(),
        description: "Test".to_string(),
        status: "pending".to_string(),
        priority: None,
        dependencies: vec![],
        subtasks: vec![],
        test_strategy: None,
        details: None,
        assignee: None, // No assignee
    };

    let fields = manager
        .map_task_to_github(&task)
        .expect("mapping should succeed");

    // Agent field should not be present when there's no assignee
    assert!(
        !fields.contains_key("Agent"),
        "Agent field should not be present when task has no assignee"
    );
}
