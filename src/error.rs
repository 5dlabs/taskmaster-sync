use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskMasterError {
    #[error("GitHub authentication failed: {0}")]
    AuthError(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("GitHub API error: {0}")]
    GitHubError(String),

    #[error("File watch error: {0}")]
    WatchError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid task format: {0}")]
    InvalidTaskFormat(String),

    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

pub type Result<T> = std::result::Result<T, TaskMasterError>;
