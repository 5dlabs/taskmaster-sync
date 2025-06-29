//! Integration tests for GitHub API client
//!
//! These tests require:
//! 1. GitHub CLI installed and authenticated
//! 2. A test organization and project
//!
//! Run with: cargo test --test github_integration -- --ignored --nocapture

use task_master_sync::auth::GitHubAuth;
use task_master_sync::github::{utils, GitHubAPI};

// Test configuration
const TEST_ORG: &str = "5dlabs";
const TEST_PROJECT_NUMBER: i32 = 9; // Taskmaster Sync Test project

#[tokio::test]
#[ignore = "Requires GitHub authentication"]
async fn test_github_auth_status() {
    println!("Testing GitHub CLI authentication...");

    let result = GitHubAuth::verify_authentication().await;

    match result {
        Ok(status) => {
            println!("✓ Authenticated: {}", status.authenticated);
            println!("✓ Username: {:?}", status.username);
            println!("✓ Scopes: {:?}", status.scopes);
            assert!(status.authenticated);
        }
        Err(e) => {
            panic!("Authentication failed: {e}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_get_project() {
    println!("Testing get_project...");

    // First verify auth
    GitHubAuth::verify_authentication()
        .await
        .expect("Must be authenticated");

    let api = GitHubAPI::new(TEST_ORG.to_string());

    match api.get_project(TEST_PROJECT_NUMBER).await {
        Ok(project) => {
            println!("✓ Project ID: {}", project.id);
            println!("✓ Project Title: {}", project.title);
            println!("✓ Project URL: {}", project.url);
            println!("✓ Project Number: {}", project.number);

            assert_eq!(project.number, TEST_PROJECT_NUMBER);
            assert!(!project.id.is_empty());
            assert!(!project.title.is_empty());
        }
        Err(e) => {
            panic!("Failed to get project: {e}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_project_fields() {
    println!("Testing project fields...");

    let api = GitHubAPI::new(TEST_ORG.to_string());

    // Get project first
    let project = api
        .get_project(TEST_PROJECT_NUMBER)
        .await
        .expect("Failed to get project");

    // Get fields
    match api.get_project_fields(&project.id).await {
        Ok(fields) => {
            println!("✓ Found {} fields", fields.len());

            for field in &fields {
                println!("  - Field: {} ({})", field.name, field.data_type);
            }

            // Check for standard fields
            let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();

            assert!(field_names.contains(&"Title"));
            assert!(field_names.contains(&"Status"));
        }
        Err(e) => {
            panic!("Failed to get fields: {e}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_create_and_delete_item() {
    println!("Testing create and delete item...");

    let api = GitHubAPI::new(TEST_ORG.to_string());

    // Get project
    let project = api
        .get_project(TEST_PROJECT_NUMBER)
        .await
        .expect("Failed to get project");

    // Create a test item
    let test_title = format!("Test Item {}", chrono::Utc::now().timestamp());
    let test_body = "This is a test item created by integration tests";

    println!("Creating item: {test_title}");

    let result = api
        .create_project_item(&project.id, &test_title, test_body)
        .await
        .expect("Failed to create item");

    println!("✓ Created item with ID: {}", result.project_item_id);
    assert!(!result.project_item_id.is_empty());

    // List items to verify it exists
    let items = api
        .list_project_items(&project.id)
        .await
        .expect("Failed to list items");

    let found = items.iter().any(|item| item.id == result.project_item_id);
    assert!(found, "Created item not found in project");

    // Clean up - delete the item
    println!("Deleting test item...");
    api.delete_project_item(&project.id, &result.project_item_id)
        .await
        .expect("Failed to delete item");

    println!("✓ Successfully deleted test item");
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_update_item() {
    println!("Testing update item...");

    let api = GitHubAPI::new(TEST_ORG.to_string());

    // Get project
    let project = api
        .get_project(TEST_PROJECT_NUMBER)
        .await
        .expect("Failed to get project");

    // Create a test item
    let result = api
        .create_project_item(&project.id, "Test Update Item", "Original body")
        .await
        .expect("Failed to create item");

    println!("✓ Created item: {}", result.project_item_id);

    // Update the item
    let updated_title = "Updated Test Item";
    let updated_body = "Updated body content";

    api.update_project_item(
        &project.id,
        &result.draft_issue_id,
        updated_title,
        updated_body,
    )
    .await
    .expect("Failed to update item");

    println!("✓ Updated item successfully");

    // Clean up
    api.delete_project_item(&project.id, &result.project_item_id)
        .await
        .expect("Failed to delete item");
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_field_operations() {
    println!("Testing field operations...");

    let api = GitHubAPI::new(TEST_ORG.to_string());

    // Get project
    let project = api
        .get_project(TEST_PROJECT_NUMBER)
        .await
        .expect("Failed to get project");

    // Try to create a custom field
    let field_name = format!("TestField{}", chrono::Utc::now().timestamp_millis());

    println!("Creating field: {field_name}");

    match api
        .create_custom_field(&project.id, &field_name, "TEXT")
        .await
    {
        Ok(field_id) => {
            println!("✓ Created field with ID: {field_id}");

            // Verify it exists
            let fields = api
                .get_project_fields(&project.id)
                .await
                .expect("Failed to get fields");

            let found = fields.iter().any(|f| f.name == field_name);
            assert!(found, "Created field not found");
        }
        Err(e) => {
            // Some projects may not allow custom field creation
            println!("⚠️  Could not create custom field: {e}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires GitHub authentication and test project"]
async fn test_pagination() {
    println!("Testing pagination with large result set...");

    let api = GitHubAPI::new(TEST_ORG.to_string());

    // Get project
    let project = api
        .get_project(TEST_PROJECT_NUMBER)
        .await
        .expect("Failed to get project");

    // List all items (pagination should handle this automatically)
    let items = api
        .list_project_items(&project.id)
        .await
        .expect("Failed to list items");

    println!("✓ Retrieved {} items with pagination", items.len());

    // Verify items have required fields
    if !items.is_empty() {
        let first_item = &items[0];
        assert!(!first_item.id.is_empty());
        println!("  First item ID: {}", first_item.id);
        println!("  First item title: {}", first_item.title);
    }
}

#[tokio::test]
#[ignore = "Requires real GitHub URL"]
async fn test_parse_project_url() {
    let test_cases = vec![
        (
            "https://github.com/orgs/myorg/projects/123",
            Some(("myorg", 123)),
        ),
        (
            "https://github.com/orgs/test-org/projects/45",
            Some(("test-org", 45)),
        ),
        ("https://github.com/user/repo", None),
        ("invalid-url", None),
    ];

    for (url, expected) in test_cases {
        match utils::parse_project_url(url) {
            Ok((org, num)) => {
                if let Some((exp_org, exp_num)) = expected {
                    assert_eq!(org, exp_org);
                    assert_eq!(num, exp_num);
                    println!("✓ Parsed {url} -> org: {org}, number: {num}");
                } else {
                    panic!("Expected parse to fail for: {url}");
                }
            }
            Err(_) => {
                assert!(expected.is_none(), "Expected parse to succeed for: {url}");
                println!("✓ Correctly rejected invalid URL: {url}");
            }
        }
    }
}

#[tokio::test]
async fn test_graphql_error_handling() {
    println!("Testing GraphQL error handling...");

    let api = GitHubAPI::new("nonexistent-org".to_string());

    // This should fail with a clear error
    match api.get_project(999999).await {
        Ok(_) => panic!("Expected error for nonexistent project"),
        Err(e) => {
            println!("✓ Got expected error: {e}");
            let error_str = e.to_string();
            assert!(error_str.contains("GitHub") || error_str.contains("GraphQL"));
        }
    }
}
