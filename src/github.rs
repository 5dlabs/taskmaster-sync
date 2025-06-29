//! GitHub API client for Projects v2
//!
//! This module provides a high-level async API for interacting with GitHub Projects v2
//! using GraphQL queries via the GitHub CLI.

use crate::auth::GitHubAuth;
use crate::error::{Result, TaskMasterError};
use crate::models::github::{CustomField, FieldValue, Project, ProjectItem};
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

/// GitHub API client for project management
pub struct GitHubAPI {
    organization: String,
    retry_count: u32,
    retry_delay: Duration,
}

/// Result from creating a project item
#[derive(Debug, Clone)]
pub struct CreateItemResult {
    pub project_item_id: String,
    pub draft_issue_id: String,
}

/// Result from creating a repository issue
#[derive(Debug, Clone)]
struct CreateIssueResult {
    pub issue_id: String,
    pub issue_number: i32,
}

/// Result from adding item to project
#[derive(Debug, Clone)]
struct AddToProjectResult {
    pub project_item_id: String,
}

impl GitHubAPI {
    /// Creates a new GitHub API client
    pub fn new(organization: String) -> Self {
        Self {
            organization,
            retry_count: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }

    /// Gets a project by number
    pub async fn get_project(&self, project_number: i32) -> Result<Project> {
        let query = r#"
            query($org: String!, $number: Int!) {
                organization(login: $org) {
                    projectV2(number: $number) {
                        id
                        number
                        title
                        url
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "org": self.organization,
            "number": project_number
        });

        let response = self.execute_with_retry(query, variables).await?;

        let project = response["data"]["organization"]["projectV2"].clone();

        serde_json::from_value(project).map_err(|e| TaskMasterError::JsonError(e))
    }

    /// Lists all items in a project with pagination
    pub async fn list_project_items(&self, project_id: &str) -> Result<Vec<ProjectItem>> {
        let mut all_items = Vec::new();
        let mut has_next_page = true;
        let mut cursor: Option<String> = None;

        while has_next_page {
            let query = r#"
                query($projectId: ID!, $cursor: String) {
                    node(id: $projectId) {
                        ... on ProjectV2 {
                            items(first: 100, after: $cursor) {
                                pageInfo {
                                    hasNextPage
                                    endCursor
                                }
                                nodes {
                                    id
                                    content {
                                        ... on DraftIssue {
                                            title
                                            body
                                        }
                                        ... on Issue {
                                            title
                                            body
                                            number
                                        }
                                        ... on PullRequest {
                                            title
                                            body
                                            number
                                        }
                                    }
                                    fieldValues(first: 20) {
                                        nodes {
                                            ... on ProjectV2ItemFieldTextValue {
                                                text
                                                field {
                                                    ... on ProjectV2Field {
                                                        id
                                                        name
                                                    }
                                                }
                                            }
                                            ... on ProjectV2ItemFieldSingleSelectValue {
                                                name
                                                field {
                                                    ... on ProjectV2SingleSelectField {
                                                        id
                                                        name
                                                    }
                                                }
                                            }
                                            ... on ProjectV2ItemFieldNumberValue {
                                                number
                                                field {
                                                    ... on ProjectV2Field {
                                                        id
                                                        name
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            "#;

            let variables = serde_json::json!({
                "projectId": project_id,
                "cursor": cursor
            });

            let response = self.execute_with_retry(query, variables).await?;

            let items_data = &response["data"]["node"]["items"];
            let page_info = &items_data["pageInfo"];

            has_next_page = page_info["hasNextPage"].as_bool().unwrap_or(false);
            cursor = page_info["endCursor"].as_str().map(String::from);

            // Parse items
            if let Some(nodes) = items_data["nodes"].as_array() {
                for node in nodes {
                    if let Ok(item) = self.parse_project_item(node) {
                        all_items.push(item);
                    }
                }
            }
        }

        Ok(all_items)
    }

    /// Creates a new project item (either draft issue or real repository issue)
    /// Returns the project item ID (used for deletion and field updates)
    pub async fn create_project_item(
        &self,
        project_id: &str,
        title: &str,
        body: &str,
    ) -> Result<CreateItemResult> {
        self.create_draft_issue(project_id, title, body).await
    }

    /// Creates a repository issue and adds it to the project
    /// Returns the project item ID (used for deletion and field updates)
    pub async fn create_project_item_with_issue(
        &self,
        project_id: &str,
        repository: &str,
        title: &str,
        body: &str,
        assignees: Option<Vec<String>>,
    ) -> Result<CreateItemResult> {
        // First create the repository issue
        let issue_result = self.create_repository_issue(repository, title, body, assignees).await?;

        // Then add it to the project
        let project_item_result = self.add_issue_to_project(project_id, &issue_result.issue_id).await?;

        Ok(CreateItemResult {
            project_item_id: project_item_result.project_item_id,
            draft_issue_id: issue_result.issue_id, // Store the real issue ID
        })
    }

    /// Creates a new draft issue in the project (internal method)
    async fn create_draft_issue(
        &self,
        project_id: &str,
        title: &str,
        body: &str,
    ) -> Result<CreateItemResult> {
        let mutation = r#"
            mutation($projectId: ID!, $title: String!, $body: String!) {
                addProjectV2DraftIssue(input: {
                    projectId: $projectId,
                    title: $title,
                    body: $body
                }) {
                    projectItem {
                        id
                        content {
                            ... on DraftIssue {
                                id
                            }
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "projectId": project_id,
            "title": title,
            "body": body
        });

        let response = self.execute_with_retry(mutation, variables).await?;

        // Extract both IDs
        let project_item_id = response["data"]["addProjectV2DraftIssue"]["projectItem"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let draft_issue_id = response["data"]["addProjectV2DraftIssue"]["projectItem"]["content"]
            ["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(CreateItemResult {
            project_item_id,
            draft_issue_id,
        })
    }

    /// Creates a repository issue
    async fn create_repository_issue(
        &self,
        repository: &str,
        title: &str,
        body: &str,
        assignees: Option<Vec<String>>,
    ) -> Result<CreateIssueResult> {
        let mutation = r#"
            mutation($repositoryId: ID!, $title: String!, $body: String!, $assigneeIds: [ID!]) {
                createIssue(input: {
                    repositoryId: $repositoryId,
                    title: $title,
                    body: $body,
                    assigneeIds: $assigneeIds
                }) {
                    issue {
                        id
                        number
                    }
                }
            }
        "#;

        // Parse repository owner/name
        let parts: Vec<&str> = repository.split('/').collect();
        if parts.len() != 2 {
            return Err(TaskMasterError::ConfigError(format!(
                "Invalid repository format '{}'. Expected 'owner/name'",
                repository
            )));
        }

        // Get repository ID
        let repo_id = self.get_repository_id(parts[0], parts[1]).await?;

        // Convert assignee usernames to user IDs (if provided)
        let mut assignee_ids = Vec::new();
        if let Some(assignee_usernames) = assignees {
            for username in assignee_usernames {
                if let Ok(user_id) = self.get_user_id(&username).await {
                    assignee_ids.push(user_id);
                }
            }
        }

        let variables = serde_json::json!({
            "repositoryId": repo_id,
            "title": title,
            "body": body,
            "assigneeIds": assignee_ids
        });

        let response = self.execute_with_retry(mutation, variables).await?;

        let issue_id = response["data"]["createIssue"]["issue"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let issue_number = response["data"]["createIssue"]["issue"]["number"]
            .as_i64()
            .unwrap_or(0) as i32;

        Ok(CreateIssueResult {
            issue_id,
            issue_number,
        })
    }

    /// Adds an issue to a project
    async fn add_issue_to_project(
        &self,
        project_id: &str,
        issue_id: &str,
    ) -> Result<AddToProjectResult> {
        let mutation = r#"
            mutation($projectId: ID!, $contentId: ID!) {
                addProjectV2ItemById(input: {
                    projectId: $projectId,
                    contentId: $contentId
                }) {
                    item {
                        id
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "projectId": project_id,
            "contentId": issue_id
        });

        let response = self.execute_with_retry(mutation, variables).await?;

        let project_item_id = response["data"]["addProjectV2ItemById"]["item"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(AddToProjectResult { project_item_id })
    }

    /// Gets repository ID from owner and name
    async fn get_repository_id(&self, owner: &str, name: &str) -> Result<String> {
        let query = r#"
            query($owner: String!, $name: String!) {
                repository(owner: $owner, name: $name) {
                    id
                }
            }
        "#;

        let variables = serde_json::json!({
            "owner": owner,
            "name": name
        });

        let response = self.execute_with_retry(query, variables).await?;

        let repo_id = response["data"]["repository"]["id"]
            .as_str()
            .ok_or_else(|| {
                TaskMasterError::GitHubError(format!(
                    "Repository {}/{} not found",
                    owner, name
                ))
            })?
            .to_string();

        Ok(repo_id)
    }

    /// Updates an existing project item
    /// NOTE: This requires a DraftIssue ID, not a ProjectItem ID
    /// TODO: Add method to get DraftIssue ID from ProjectItem ID
    pub async fn update_project_item(
        &self,
        project_id: &str,
        item_id: &str,
        title: &str,
        body: &str,
    ) -> Result<()> {
        let mutation = r#"
            mutation($draftIssueId: ID!, $title: String!, $body: String!) {
                updateProjectV2DraftIssue(input: {
                    draftIssueId: $draftIssueId,
                    title: $title,
                    body: $body
                }) {
                    draftIssue {
                        id
                        title
                        body
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "draftIssueId": item_id,
            "title": title,
            "body": body
        });

        self.execute_with_retry(mutation, variables).await?;
        Ok(())
    }

    /// Updates a field value for a project item
    pub async fn update_field_value(
        &self,
        project_id: &str,
        item_id: &str,
        field_id: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        let mutation = r#"
            mutation($projectId: ID!, $itemId: ID!, $fieldId: ID!, $value: ProjectV2FieldValue!) {
                updateProjectV2ItemFieldValue(input: {
                    projectId: $projectId,
                    itemId: $itemId,
                    fieldId: $fieldId,
                    value: $value
                }) {
                    projectV2Item {
                        id
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "projectId": project_id,
            "itemId": item_id,
            "fieldId": field_id,
            "value": value
        });

        self.execute_with_retry(mutation, variables).await?;
        Ok(())
    }

    /// Deletes a project item
    pub async fn delete_project_item(&self, project_id: &str, item_id: &str) -> Result<()> {
        let mutation = r#"
            mutation($projectId: ID!, $itemId: ID!) {
                deleteProjectV2Item(input: {
                    projectId: $projectId,
                    itemId: $itemId
                }) {
                    deletedItemId
                }
            }
        "#;

        let variables = serde_json::json!({
            "projectId": project_id,
            "itemId": item_id
        });

        self.execute_with_retry(mutation, variables).await?;
        Ok(())
    }

    /// Gets project fields
    pub async fn get_project_fields(&self, project_id: &str) -> Result<Vec<CustomField>> {
        let query = r#"
            query($projectId: ID!) {
                node(id: $projectId) {
                    ... on ProjectV2 {
                        fields(first: 100) {
                            nodes {
                                ... on ProjectV2Field {
                                    id
                                    name
                                    dataType
                                }
                                ... on ProjectV2SingleSelectField {
                                    id
                                    name
                                    dataType
                                    options {
                                        id
                                        name
                                        color
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "projectId": project_id
        });

        let response = self.execute_with_retry(query, variables).await?;

        let fields_nodes = &response["data"]["node"]["fields"]["nodes"];

        if let Some(nodes) = fields_nodes.as_array() {
            Ok(nodes
                .iter()
                .filter_map(|node| serde_json::from_value::<CustomField>(node.clone()).ok())
                .collect())
        } else {
            Ok(vec![])
        }
    }

    /// Creates a custom field in the project
    pub async fn create_custom_field(
        &self,
        project_id: &str,
        name: &str,
        data_type: &str,
    ) -> Result<String> {
        let (mutation, variables) = match data_type {
            "TEXT" => {
                let mutation = r#"
                mutation($projectId: ID!, $name: String!) {
                    createProjectV2Field(input: {
                        projectId: $projectId,
                        dataType: TEXT,
                        name: $name
                    }) {
                        projectV2Field {
                            ... on ProjectV2Field {
                                id
                            }
                        }
                    }
                }
            "#;
                let variables = serde_json::json!({
                    "projectId": project_id,
                    "name": name
                });
                (mutation, variables)
            }
            "SINGLE_SELECT" => {
                let mutation = r#"
                mutation($projectId: ID!, $name: String!, $options: [ProjectV2SingleSelectFieldOptionInput!]!) {
                    createProjectV2Field(input: {
                        projectId: $projectId,
                        dataType: SINGLE_SELECT,
                        name: $name,
                        singleSelectOptions: $options
                    }) {
                        projectV2Field {
                            ... on ProjectV2SingleSelectField {
                                id
                            }
                        }
                    }
                }
            "#;

                // Provide default options based on field name
                let options = match name {
                    "Priority" => serde_json::json!([
                        {"name": "high", "color": "RED", "description": "High priority task"},
                        {"name": "medium", "color": "YELLOW", "description": "Medium priority task"},
                        {"name": "low", "color": "GREEN", "description": "Low priority task"}
                    ]),
                    "Status" => serde_json::json!([
                        {"name": "To Do", "color": "GRAY", "description": "Task is pending"},
                        {"name": "In Progress", "color": "YELLOW", "description": "Task is in progress"},
                        {"name": "QA Review", "color": "BLUE", "description": "Task completed, awaiting QA approval"},
                        {"name": "Done", "color": "GREEN", "description": "Task completed and QA approved"},
                        {"name": "Blocked", "color": "RED", "description": "Task is blocked"}
                    ]),
                    "Agent" => serde_json::json!([
                        {"name": "Unassigned", "color": "GRAY", "description": "No agent assigned"}
                    ]),
                    _ => serde_json::json!([
                        {"name": "Default", "color": "GRAY", "description": "Default option"}
                    ]),
                };

                let variables = serde_json::json!({
                    "projectId": project_id,
                    "name": name,
                    "options": options
                });
                (mutation, variables)
            }
            _ => {
                return Err(TaskMasterError::InvalidTaskFormat(format!(
                    "Unsupported field type: {}",
                    data_type
                )))
            }
        };

        let response = self.execute_with_retry(mutation, variables).await?;

        Ok(
            response["data"]["createProjectV2Field"]["projectV2Field"]["id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    }

    /// Gets user ID by username
    async fn get_user_id(&self, username: &str) -> Result<String> {
        let query = r#"
            query($login: String!) {
                user(login: $login) {
                    id
                }
            }
        "#;

        let variables = serde_json::json!({
            "login": username
        });

        let response = self.execute_with_retry(query, variables).await?;

        Ok(response["data"]["user"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Creates a new option for a single select field
    pub async fn create_field_option(
        &self,
        project_id: &str,
        field_id: &str,
        option_name: &str,
        color: &str,
    ) -> Result<String> {
        let mutation = r#"
            mutation($projectId: ID!, $fieldId: ID!, $option: ProjectV2SingleSelectFieldOptionInput!) {
                updateProjectV2Field(input: {
                    projectId: $projectId,
                    fieldId: $fieldId,
                    singleSelectOptions: [$option]
                }) {
                    projectV2Field {
                        ... on ProjectV2SingleSelectField {
                            options {
                                id
                                name
                            }
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "projectId": project_id,
            "fieldId": field_id,
            "option": {
                "name": option_name,
                "color": color,
                "description": format!("{} option", option_name)
            }
        });

        let response = self.execute_with_retry(mutation, variables).await?;

        // Find the newly created option ID
        if let Some(options) = response["data"]["updateProjectV2Field"]["projectV2Field"]["options"].as_array() {
            for option in options {
                if option["name"].as_str() == Some(option_name) {
                    return Ok(option["id"].as_str().unwrap_or("").to_string());
                }
            }
        }

        Err(TaskMasterError::GitHubError(
            format!("Failed to create option '{}' for field", option_name)
        ))
    }

    /// Executes a GraphQL query with retry logic
    async fn execute_with_retry(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count < self.retry_count {
            match GitHubAuth::execute_graphql(query, variables.clone()).await {
                Ok(response) => {
                    // Check for GraphQL errors
                    if let Some(errors) = response.get("errors") {
                        if errors.is_array() && !errors.as_array().unwrap().is_empty() {
                            let error_msg = serde_json::to_string_pretty(errors)
                                .unwrap_or_else(|_| "Unknown GraphQL error".to_string());
                            return Err(TaskMasterError::GitHubError(error_msg));
                        }
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    retry_count += 1;

                    if retry_count < self.retry_count {
                        // Exponential backoff
                        let delay = self.retry_delay * 2u32.pow(retry_count - 1);
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| TaskMasterError::GitHubError("Max retries exceeded".to_string())))
    }

    /// Parses a project item from GraphQL response
    fn parse_project_item(&self, node: &Value) -> Result<ProjectItem> {
        let id = node["id"].as_str().unwrap_or("").to_string();

        let (title, body) = if let Some(content) = node.get("content") {
            (
                content["title"].as_str().unwrap_or("").to_string(),
                content["body"].as_str().map(String::from),
            )
        } else {
            ("".to_string(), None)
        };

        let mut field_values = Vec::new();
        if let Some(field_nodes) = node["fieldValues"]["nodes"].as_array() {
            for field_node in field_nodes {
                if let Ok(field_value) = serde_json::from_value::<FieldValue>(field_node.clone()) {
                    field_values.push(field_value);
                }
            }
        }

        Ok(ProjectItem {
            id,
            title,
            body,
            field_values,
        })
    }
}

/// Utility functions for GitHub operations
pub mod utils {
    use super::*;

    /// Parses a GitHub project URL to extract organization and project number
    pub fn parse_project_url(url: &str) -> Result<(String, i32)> {
        // Expected format: https://github.com/orgs/ORG/projects/NUMBER
        let parts: Vec<&str> = url.split('/').collect();

        if parts.len() < 6 || parts[3] != "orgs" || parts[5] != "projects" {
            return Err(TaskMasterError::InvalidTaskFormat(
                "Invalid GitHub project URL format".to_string(),
            ));
        }

        let org = parts[4].to_string();
        let project_number = parts[6].parse::<i32>().map_err(|_| {
            TaskMasterError::InvalidTaskFormat("Invalid project number in URL".to_string())
        })?;

        Ok((org, project_number))
    }

    /// Formats a field value for GraphQL mutation
    pub fn format_field_value(value: &str, field_type: &str) -> serde_json::Value {
        match field_type {
            "TEXT" => serde_json::json!({ "text": value }),
            "NUMBER" => {
                let number = value.parse::<f64>().unwrap_or(0.0);
                serde_json::json!({ "number": number })
            }
            "SINGLE_SELECT" => serde_json::json!({ "singleSelectOptionId": value }),
            _ => serde_json::json!({ "text": value }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_project_url() {
        let url = "https://github.com/orgs/myorg/projects/123";
        let (org, number) = utils::parse_project_url(url).unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(number, 123);
    }

    #[test]
    fn test_parse_invalid_url() {
        let url = "https://github.com/user/repo";
        assert!(utils::parse_project_url(url).is_err());
    }

    #[test]
    fn test_format_field_value() {
        let text_value = utils::format_field_value("Hello", "TEXT");
        assert_eq!(text_value, serde_json::json!({ "text": "Hello" }));

        let number_value = utils::format_field_value("42", "NUMBER");
        assert_eq!(number_value, serde_json::json!({ "number": 42.0 }));
    }
}
