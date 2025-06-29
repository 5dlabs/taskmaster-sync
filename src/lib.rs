// Task Master Sync - Library root
//
// This crate provides functionality to sync Taskmaster tasks with GitHub Projects

#![allow(dead_code)] // Allow dead code for incomplete functionality

pub mod auth;
pub mod config;
pub mod delta;
pub mod error;
pub mod fields;
pub mod github;
pub mod models;
pub mod progress;
pub mod state;
pub mod subtasks;
pub mod sync;
pub mod taskmaster;
pub mod watcher;

// Re-export commonly used types
pub use error::{Result, TaskMasterError};
pub use models::config::SyncConfig;
pub use models::task::Task;
pub use sync::{SyncDirection, SyncEngine, SyncOptions};
