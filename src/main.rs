use anyhow::Result;
use clap::{Parser, Subcommand};

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
        /// Force full sync instead of delta sync
        #[arg(long)]
        full_sync: bool,
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
    /// Clean up duplicate items in a project
    CleanDuplicates {
        /// GitHub Project number
        project: String,
        /// Actually delete duplicates (without this, just reports them)
        #[arg(long)]
        delete: bool,
    },
    /// Create a new GitHub Project
    CreateProject {
        title: String,
        #[arg(long)]
        org: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        repository: Option<String>,
        #[arg(long)]
        public: bool,
    },
    /// Set up GitHub Project with required fields and QA workflow
    SetupProject {
        /// GitHub Project number
        project_number: i32,
        #[arg(long)]
        org: Option<String>,
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
            full_sync,
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
                    eprintln!("Failed to initialize sync engine: {e}");
                    std::process::exit(1);
                }
            };

            // Set up sync options
            let options = task_master_sync::sync::SyncOptions {
                dry_run,
                force: full_sync,
                direction: task_master_sync::sync::SyncDirection::ToGitHub,
                batch_size: 50,
                include_archived: false,
                use_delta_sync: !full_sync, // Use delta sync unless full sync is forced
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
                            eprintln!("     - {error}");
                        }
                    }

                    if dry_run {
                        println!("\n🔍 This was a dry run - no changes were made to GitHub");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Sync failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Watch {
            tag,
            project,
            debounce,
        } => {
            let _ = (tag, project); // Ignore unused for now
            tracing::info!("Watching for changes with {}ms debounce", debounce);
            // TODO: Implement watch command
            println!("Watch command not yet implemented");
        }
        Commands::Status { project } => {
            let _ = project; // Ignore unused for now
                             // TODO: Implement status command
            println!("Status command not yet implemented");
        }
        Commands::ListTags => {
            // TODO: Implement list-tags command
            println!("List tags command not yet implemented");
        }
        Commands::Configure { project, tag } => {
            let _ = (project, tag); // Ignore unused for now
                                    // TODO: Implement configure command
            println!("Configure command not yet implemented");
        }
        Commands::CleanDuplicates { project, delete } => {
            use task_master_sync::github::GitHubAPI;

            let project_number: i32 = match project.parse() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("Invalid project number: {project}");
                    std::process::exit(1);
                }
            };

            let github_api = GitHubAPI::new("5dlabs".to_string());

            // Get project
            let project = match github_api.get_project(project_number).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to get project: {e}");
                    std::process::exit(1);
                }
            };

            // Get all items
            let items = match github_api.list_project_items(&project.id).await {
                Ok(items) => items,
                Err(e) => {
                    eprintln!("Failed to list project items: {e}");
                    std::process::exit(1);
                }
            };

            // Group by TM_ID and title
            let mut tm_id_groups: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
            let mut title_groups: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
            let mut no_tm_id_items = Vec::new();

            for item in &items {
                // Extract TM_ID
                let mut tm_id = None;
                for field_value in &item.field_values {
                    if field_value.field.name == "TM_ID" {
                        if let task_master_sync::models::github::FieldValueContent::Text(id) =
                            &field_value.value
                        {
                            tm_id = Some(id.clone());
                            break;
                        }
                    }
                }

                if let Some(id) = tm_id {
                    tm_id_groups.entry(id).or_insert_with(Vec::new).push(item);
                } else {
                    no_tm_id_items.push(item);
                }

                // Also group by title
                title_groups
                    .entry(item.title.clone())
                    .or_insert_with(Vec::new)
                    .push(item);
            }

            println!("\n📊 Duplicate Analysis for Project #{project_number}");
            println!("Total items: {}", items.len());
            println!("Items without TM_ID: {}", no_tm_id_items.len());

            // Find duplicates by TM_ID
            let mut duplicates_found = false;
            println!("\n🔍 Duplicates by TM_ID:");
            for (tm_id, items) in &tm_id_groups {
                if items.len() > 1 {
                    duplicates_found = true;
                    println!("  {} - {} copies", tm_id, items.len());
                }
            }

            // Find duplicates by title
            println!("\n🔍 Duplicates by Title:");
            for (title, items) in &title_groups {
                if items.len() > 1 {
                    duplicates_found = true;
                    println!("  '{}' - {} copies", title, items.len());
                }
            }

            if !duplicates_found && no_tm_id_items.is_empty() {
                println!("\n✅ No duplicates found!");
            } else if delete {
                println!("\n🗑️  Cleaning up duplicates...");

                // Delete duplicates by TM_ID (keep the first one)
                for (tm_id, items) in &tm_id_groups {
                    if items.len() > 1 {
                        for item in items.iter().skip(1) {
                            println!("  Deleting duplicate of {tm_id}");
                            if let Err(e) =
                                github_api.delete_project_item(&project.id, &item.id).await
                            {
                                eprintln!("    Failed to delete: {e}");
                            }
                        }
                    }
                }

                // Delete items without TM_ID that have title duplicates
                for item in &no_tm_id_items {
                    if let Some(title_items) = title_groups.get(&item.title) {
                        // If there's another item with the same title that has a TM_ID, delete this one
                        let has_tm_id_version = title_items.iter().any(|i| {
                                i.field_values.iter().any(|fv| {
                                    fv.field.name == "TM_ID" &&
                                    matches!(&fv.value, task_master_sync::models::github::FieldValueContent::Text(t) if !t.is_empty())
                                })
                            });

                        if has_tm_id_version {
                            println!("  Deleting item without TM_ID: '{}'", item.title);
                            if let Err(e) =
                                github_api.delete_project_item(&project.id, &item.id).await
                            {
                                eprintln!("    Failed to delete: {e}");
                            }
                        }
                    }
                }

                println!("\n✅ Cleanup complete!");
            } else {
                println!("\n💡 Run with --delete to remove these duplicates");
            }
        }
        Commands::CreateProject {
            title,
            org,
            description,
            repository,
            public: _,
        } => {
            use task_master_sync::github::GitHubAPI;

            let org_name = org.unwrap_or_else(|| "5dlabs".to_string());
            let github_api = GitHubAPI::new(org_name.clone());

            // Use provided repository or default to taskmaster-sync
            let repo = repository.unwrap_or_else(|| format!("{org_name}/taskmaster-sync"));

            match github_api
                .create_project(&title, description.as_deref(), Some(&repo))
                .await
            {
                Ok(project) => {
                    println!(
                        "✅ Successfully created project: {} (#{}) linked to {}",
                        project.title, project.number, repo
                    );
                    println!("🔗 Project URL: {}", project.url);
                    println!("📋 Project ID: {}", project.id);

                    // Update config to include the new project
                    println!("\n💡 Next steps:");
                    println!("1. Run: task-master-sync setup-project {}", project.number);
                    println!(
                        "2. Then sync: task-master-sync sync master {}",
                        project.number
                    );

                    println!("\n💡 Or add to your .taskmaster/sync-config.json:");
                    println!("{{");
                    println!("  \"version\": \"1.0.0\",");
                    println!("  \"organization\": \"{org_name}\",");
                    println!("  \"project_mappings\": {{");
                    println!("    \"master\": {{");
                    println!("      \"project_number\": {},", project.number);
                    println!("      \"project_id\": \"{}\",", project.id);
                    println!("      \"repository\": \"{repo}\",");
                    println!("      \"subtask_mode\": \"nested\"");
                    println!("    }}");
                    println!("  }}");
                    println!("}}");
                }
                Err(e) => {
                    eprintln!("❌ Failed to create project: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::SetupProject {
            project_number,
            org,
        } => {
            use task_master_sync::fields::FieldManager;
            use task_master_sync::github::GitHubAPI;

            let org_name = org.unwrap_or_else(|| "5dlabs".to_string());
            let github_api = GitHubAPI::new(org_name.clone());

            // Get project details
            let project = match github_api.get_project(project_number).await {
                Ok(project) => {
                    println!(
                        "🔗 Setting up project: {} (#{})",
                        project.title, project.number
                    );
                    project
                }
                Err(e) => {
                    eprintln!("❌ Failed to get project #{project_number}: {e}");
                    std::process::exit(1);
                }
            };

            // Initialize field manager and sync required fields
            let field_manager = FieldManager::new();

            println!("🔄 Creating required custom fields...");
            match field_manager
                .sync_fields_to_github(&github_api, &project.id)
                .await
            {
                Ok(_) => println!("✅ Required fields created successfully"),
                Err(e) => {
                    eprintln!("❌ Failed to create required fields: {e}");
                    std::process::exit(1);
                }
            }

            // Get updated fields list
            let fields = match github_api.get_project_fields(&project.id).await {
                Ok(fields) => fields,
                Err(e) => {
                    eprintln!("❌ Failed to get project fields: {e}");
                    std::process::exit(1);
                }
            };

            // Check if QA Review option exists in Status field
            if let Some(status_field) = fields.iter().find(|f| f.name == "Status") {
                let has_qa_review = status_field
                    .options
                    .as_ref()
                    .map(|opts| opts.iter().any(|opt| opt.name == "QA Review"))
                    .unwrap_or(false);

                if !has_qa_review {
                    println!("🔄 Adding QA Review status option...");
                    match github_api
                        .create_field_option(&project.id, &status_field.id, "QA Review", "BLUE")
                        .await
                    {
                        Ok(_) => println!("✅ QA Review status option added successfully"),
                        Err(e) => {
                            println!(
                                "⚠️  Warning: Could not add QA Review option automatically: {e}"
                            );
                            println!(
                                "   Please add 'QA Review' status option manually in GitHub UI"
                            );
                        }
                    }
                } else {
                    println!("✅ QA Review status option already exists");
                }
            } else {
                println!("⚠️  Status field not found - this shouldn't happen");
            }

            println!("\n✅ Project setup complete!");
            println!("🚀 Ready to sync: task-master-sync sync master {project_number}");
        }
    }

    Ok(())
}
