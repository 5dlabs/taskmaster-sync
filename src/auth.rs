//! GitHub CLI authentication wrapper
//!
//! This module provides an async wrapper around GitHub CLI (gh) commands
//! to handle authentication without storing any credentials.
//!
//! Key features:
//! - Async command execution using tokio
//! - No credential storage - relies on gh CLI
//! - Automatic validation of gh installation and auth status

use crate::error::{Result, TaskMasterError};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

/// Authentication status returned by GitHub CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub username: Option<String>,
    pub scopes: Vec<String>,
}

/// GitHub CLI authentication wrapper
pub struct GitHubAuth;

impl GitHubAuth {
    /// Checks if GitHub CLI is installed and available
    pub async fn is_gh_installed() -> bool {
        Command::new("gh")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Verifies GitHub CLI authentication status
    pub async fn verify_authentication() -> Result<AuthStatus> {
        if !Self::is_gh_installed().await {
            return Err(TaskMasterError::AuthError(
                "GitHub CLI (gh) is not installed. Please install it from https://cli.github.com/"
                    .to_string(),
            ));
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg("unset GITHUB_TOKEN && gh auth status 2>&1")
            .output()
            .await
            .map_err(|e| {
                TaskMasterError::AuthError(format!("Failed to run gh auth status: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        // Parse the output to determine auth status
        if combined.contains("Logged in to github.com") {
            // Extract username
            let username = Self::extract_username(&combined);

            // Extract scopes
            let scopes = Self::extract_scopes(&combined);

            Ok(AuthStatus {
                authenticated: true,
                username,
                scopes,
            })
        } else {
            Err(TaskMasterError::AuthError(
                "Not authenticated with GitHub. Please run 'gh auth login'".to_string(),
            ))
        }
    }

    /// Executes a GitHub CLI command asynchronously
    pub async fn execute_gh_command(args: &[&str]) -> Result<String> {
        // First verify authentication
        Self::verify_authentication().await?;

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("unset GITHUB_TOKEN && gh {}", args.join(" ")))
            .output()
            .await
            .map_err(|e| {
                TaskMasterError::GitHubError(format!("Failed to execute gh command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskMasterError::GitHubError(format!(
                "GitHub CLI error: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Executes a GraphQL query using GitHub CLI
    pub async fn execute_graphql(
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request = serde_json::json!({
            "query": query,
            "variables": variables
        });

        let json_input =
            serde_json::to_string(&request).map_err(|e| TaskMasterError::JsonError(e))?;

        // Create a child process with stdin piped
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("unset GITHUB_TOKEN && gh api graphql --input -")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                TaskMasterError::GitHubError(format!("Failed to spawn gh command: {}", e))
            })?;

        // Write the JSON to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(json_input.as_bytes()).await.map_err(|e| {
                TaskMasterError::GitHubError(format!("Failed to write to stdin: {}", e))
            })?;
        }

        // Wait for the command to complete
        let output = child.wait_with_output().await.map_err(|e| {
            TaskMasterError::GitHubError(format!("Failed to execute GraphQL query: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskMasterError::GitHubError(format!(
                "GraphQL query failed: {}",
                stderr
            )));
        }

        let response =
            serde_json::from_slice(&output.stdout).map_err(|e| TaskMasterError::JsonError(e))?;

        Ok(response)
    }

    /// Extracts username from gh auth status output
    fn extract_username(output: &str) -> Option<String> {
        // Look for pattern: "Logged in to github.com account USERNAME ("
        if let Some(start) = output.find("account ") {
            let after_account = &output[start + 8..];
            if let Some(end) = after_account.find(" (") {
                return Some(after_account[..end].to_string());
            }
        }
        None
    }

    /// Extracts OAuth scopes from gh auth status output
    fn extract_scopes(output: &str) -> Vec<String> {
        // Look for pattern: "Token scopes: 'scope1', 'scope2', ..."
        if let Some(start) = output.find("Token scopes: ") {
            let after_scopes = &output[start + 14..];
            if let Some(end) = after_scopes.find('\n') {
                let scopes_str = &after_scopes[..end];
                return scopes_str
                    .split(", ")
                    .map(|s| s.trim_matches('\'').to_string())
                    .collect();
            }
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gh_installed_check() {
        // This test will pass if gh is installed on the system
        let installed = GitHubAuth::is_gh_installed().await;
        // We can't assert true/false as it depends on the system
        // Just ensure the function runs without panic
        println!("GitHub CLI installed: {}", installed);
    }

    #[test]
    fn test_extract_username() {
        let output = "✓ Logged in to github.com account testuser (keyring)
✓ Git operations for github.com configured to use https protocol.
✓ Token: gho_************************************
✓ Token scopes: 'admin:public_key', 'gist', 'read:org', 'repo'";

        let username = GitHubAuth::extract_username(output);
        assert_eq!(username, Some("testuser".to_string()));
    }

    #[test]
    fn test_extract_scopes() {
        let output = "✓ Logged in to github.com account testuser (keyring)
✓ Git operations for github.com configured to use https protocol.
✓ Token: gho_************************************
✓ Token scopes: 'admin:public_key', 'gist', 'read:org', 'repo'";

        let scopes = GitHubAuth::extract_scopes(output);
        assert_eq!(scopes, vec!["admin:public_key", "gist", "read:org", "repo"]);
    }

    #[tokio::test]
    async fn test_execute_gh_command_without_auth() {
        // This test shows how errors are handled when not authenticated
        // We can't actually test this without affecting the system's auth state

        // If gh is installed, try to run a simple command
        if GitHubAuth::is_gh_installed().await {
            // Try to get version - this should work even without auth
            let result = Command::new("gh").arg("--version").output().await;

            assert!(result.is_ok());
        }
    }
}
