//! File system watcher for automatic synchronization
//!
//! This module handles:
//! - Watching tasks.json for changes
//! - Debouncing rapid changes
//! - Triggering automatic sync
//! - Managing watch state

use crate::error::Result;
use crate::sync::{SyncEngine, SyncOptions};
use crate::TaskMasterError;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use tracing::{error, info, warn};

/// Watches TaskMaster files for changes and triggers sync
pub struct TaskWatcher {
    watcher: Box<dyn Watcher + Send>,
    _sync_engine: Arc<Mutex<SyncEngine>>,
    _debounce_duration: Duration,
    watch_path: PathBuf,
}

/// Events from the file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    TasksChanged,
    ConfigChanged,
    Error(String),
}

impl TaskWatcher {
    /// Creates a new task watcher
    pub fn new(
        project_root: impl AsRef<Path>,
        sync_engine: Arc<Mutex<SyncEngine>>,
        debounce_duration: Duration,
    ) -> Result<Self> {
        let watch_path = project_root.as_ref().join(".taskmaster/tasks/tasks.json");

        if !watch_path.exists() {
            return Err(TaskMasterError::ConfigError(format!(
                "Tasks file not found at: {}",
                watch_path.display()
            )));
        }

        // Create a channel for events
        let (tx, rx) = mpsc::channel(100);

        // Create the watcher
        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                match result {
                    Ok(event) => {
                        // Only care about write/create events
                        if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            if let Err(e) = tx.blocking_send(WatchEvent::TasksChanged) {
                                error!("Failed to send watch event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {}", e);
                        let _ = tx.blocking_send(WatchEvent::Error(e.to_string()));
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| TaskMasterError::WatchError(e.to_string()))?;

        let watcher_instance = TaskWatcher {
            watcher: Box::new(watcher),
            _sync_engine: sync_engine.clone(),
            _debounce_duration: debounce_duration,
            watch_path: watch_path.clone(),
        };

        // Spawn the event processor
        tokio::spawn(Self::event_processor(rx, sync_engine, debounce_duration));

        Ok(watcher_instance)
    }

    /// Starts watching for file changes
    pub fn start(&mut self) -> Result<()> {
        info!("Starting file watcher for: {}", self.watch_path.display());

        // Watch the parent directory to catch file replacements
        let watch_dir = self
            .watch_path
            .parent()
            .ok_or_else(|| TaskMasterError::ConfigError("Invalid watch path".to_string()))?;

        self.watcher
            .watch(watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| TaskMasterError::WatchError(e.to_string()))?;

        info!("File watcher started successfully");
        Ok(())
    }

    /// Stops watching
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping file watcher");

        let watch_dir = self
            .watch_path
            .parent()
            .ok_or_else(|| TaskMasterError::ConfigError("Invalid watch path".to_string()))?;

        self.watcher
            .unwatch(watch_dir)
            .map_err(|e| TaskMasterError::WatchError(e.to_string()))?;

        info!("File watcher stopped");
        Ok(())
    }

    /// Processes events with debouncing
    async fn event_processor(
        mut rx: mpsc::Receiver<WatchEvent>,
        sync_engine: Arc<Mutex<SyncEngine>>,
        debounce_duration: Duration,
    ) {
        let mut debouncer = Debouncer::new(debounce_duration);
        let mut pending_sync = false;

        loop {
            // Wait for event or timeout
            match time::timeout(debounce_duration, rx.recv()).await {
                Ok(Some(event)) => match event {
                    WatchEvent::TasksChanged => {
                        info!("File change detected");
                        pending_sync = true;
                        debouncer.reset();
                    }
                    WatchEvent::ConfigChanged => {
                        info!("Config change detected");
                        pending_sync = true;
                        debouncer.reset();
                    }
                    WatchEvent::Error(e) => {
                        error!("Watch error: {}", e);
                    }
                },
                Ok(None) => {
                    // Channel closed
                    warn!("Watch event channel closed");
                    break;
                }
                Err(_) => {
                    // Timeout - check if we should sync
                    if pending_sync && debouncer.should_trigger() {
                        info!("Triggering sync after debounce period");

                        let engine = sync_engine.lock().await;
                        let tag = engine.tag.clone();
                        let options = SyncOptions {
                            dry_run: false,
                            force: false,
                            direction: crate::sync::SyncDirection::ToGitHub,
                            batch_size: 50,
                            include_archived: false,
                            use_delta_sync: true,
                            quiet: false,
                        };

                        drop(engine); // Release lock before sync

                        let mut engine = sync_engine.lock().await;
                        match engine.sync(&tag, options).await {
                            Ok(result) => {
                                info!(
                                    "Auto-sync completed: created={}, updated={}, deleted={}",
                                    result.stats.created,
                                    result.stats.updated,
                                    result.stats.deleted
                                );
                            }
                            Err(e) => {
                                error!("Auto-sync failed: {}", e);
                            }
                        }

                        pending_sync = false;
                    }
                }
            }
        }
    }
}

/// Watch mode for automatic sync
#[derive(Debug, Clone)]
pub struct WatchMode {
    pub auto_sync: bool,
    pub sync_options: SyncOptions,
    pub debounce_ms: u64,
    pub ignore_patterns: Vec<String>,
}

impl Default for WatchMode {
    fn default() -> Self {
        Self {
            auto_sync: true,
            sync_options: SyncOptions::default(),
            debounce_ms: 1000,
            ignore_patterns: vec!["*.tmp".to_string(), "*.swp".to_string(), "*~".to_string()],
        }
    }
}

/// Debouncer for file change events
struct Debouncer {
    duration: Duration,
    last_event: Option<time::Instant>,
}

impl Debouncer {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            last_event: None,
        }
    }

    /// Checks if enough time has passed since last event
    fn should_trigger(&mut self) -> bool {
        match self.last_event {
            None => {
                self.last_event = Some(time::Instant::now());
                true
            }
            Some(last) => {
                let now = time::Instant::now();
                if now.duration_since(last) >= self.duration {
                    self.last_event = Some(now);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Resets the debouncer
    fn reset(&mut self) {
        self.last_event = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_watcher_creation() {
        // TODO: Test watcher creation
    }

    #[tokio::test]
    async fn test_debouncing() {
        // TODO: Test debouncing logic
    }

    #[test]
    fn test_watch_mode_defaults() {
        let mode = WatchMode::default();
        assert!(mode.auto_sync);
        assert_eq!(mode.debounce_ms, 1000);
    }
}
