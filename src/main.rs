use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "task-master-sync")]
#[command(about = "Sync Taskmaster tasks to GitHub Projects", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync Taskmaster tasks to GitHub Projects
    Sync {
        /// Taskmaster tag to sync
        tag: String,
        /// GitHub Project number or name
        project: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        subtasks_as_items: bool,
        #[arg(long)]
        subtasks_in_body: bool,
    },
    /// Watch for changes and auto-sync
    Watch {
        tag: String,
        project: String,
        #[arg(long, default_value = "1000")]
        debounce: u64,
    },
    /// Show sync status
    Status { project: Option<String> },
    /// List available tags
    ListTags,
    /// Configure project mappings
    Configure {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Create a new GitHub Project
    CreateProject {
        title: String,
        #[arg(long)]
        org: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        public: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    match cli.command {
        Commands::Sync {
            tag,
            project,
            dry_run,
            subtasks_as_items: _,
            subtasks_in_body: _,
        } => {
            tracing::info!("Syncing tag '{}' to project '{}'", tag, project);

            // Initialize sync engine
            let config_path = ".taskmaster/sync-config.json";
            let mut sync_engine = match task_master_sync::sync::SyncEngine::new(
                config_path,
                &tag,
                project.parse().unwrap_or(0),
            )
            .await
            {
                Ok(engine) => engine,
                Err(e) => {
                    eprintln!("Failed to initialize sync engine: {}", e);
                    std::process::exit(1);
                }
            };

            // Set up sync options
            let options = task_master_sync::sync::SyncOptions {
                dry_run,
                force: false,
                direction: task_master_sync::sync::SyncDirection::ToGitHub,
                batch_size: 50,
                include_archived: false,
            };

            // Run sync
            match sync_engine.sync(&tag, options).await {
                Ok(result) => {
                    println!("\n✅ Sync completed successfully!");
                    println!("   Created: {}", result.stats.created);
                    println!("   Updated: {}", result.stats.updated);
                    println!("   Deleted: {}", result.stats.deleted);
                    println!("   Skipped: {}", result.stats.skipped);

                    if !result.stats.errors.is_empty() {
                        println!("   Errors: {}", result.stats.errors.len());
                        for error in &result.stats.errors {
                            eprintln!("     - {}", error);
                        }
                    }

                    if dry_run {
                        println!("\n🔍 This was a dry run - no changes were made to GitHub");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Sync failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Watch {
            tag,
            project,
            debounce,
        } => {
            tracing::info!("Watching for changes with {}ms debounce", debounce);
            // TODO: Implement watch command
            println!("Watch command not yet implemented");
        }
        Commands::Status { project } => {
            // TODO: Implement status command
            println!("Status command not yet implemented");
        }
        Commands::ListTags => {
            // TODO: Implement list-tags command
            println!("List tags command not yet implemented");
        }
        Commands::Configure { project, tag } => {
            // TODO: Implement configure command
            println!("Configure command not yet implemented");
        }
        Commands::CreateProject {
            title,
            org,
            description,
            public: _,
        } => {
            // TODO: Implement create-project command
            println!("Create project command not yet implemented");
        }
    }

    Ok(())
}
