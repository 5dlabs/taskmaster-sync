# Task Master Sync - Claude Code Configuration

## Project Overview

Task Master Sync is a high-performance Rust CLI tool that synchronizes Taskmaster tasks with GitHub Projects for visual project management. Built with production-grade standards including comprehensive error handling, async I/O, and zero-dependency file operations.

### Key Technologies

- **Language**: Rust (latest stable)
- **CLI Framework**: Clap for command-line interface
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest for GitHub API
- **Serialization**: Serde (JSON)
- **Error Handling**: Anyhow + Thiserror
- **Testing**: Built-in test harness with integration tests
- **CI/CD**: GitHub Actions with cross-platform builds

## Development Context

### Architecture Decisions

- **CLI-First**: Single binary with subcommands
- **Type Safety**: Leverage Rust's type system for GitHub API interactions
- **Zero-Cost Abstractions**: Performance by default
- **Cross-Platform**: Native binaries for Linux and macOS

### Current Development Phase

- Core implementation in progress (Task 7+)
- Quality gates enforced at all stages
- Focus on developer productivity with AI assistance

### Important Authentication Note

**IMPORTANT**: Always use the Zsh profile with Jonathan Julian user to access the GitHub CLI. This ensures proper authentication for GitHub operations.

## Code Patterns & Standards

### Rust-Specific Patterns

#### Error Handling

```rust
// Use custom error types with thiserror
#[derive(thiserror::Error, Debug)]
pub enum TaskMasterError {
    #[error("GitHub API error: {0}")]
    GitHub(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Invalid task format: {0}")]
    InvalidTaskFormat(String),
}

// Propagate with ?
async fn sync_tasks() -> Result<()> {
    let tasks = load_tasks().await?;
    let github_client = create_github_client().await?;
    sync_to_github(&github_client, &tasks).await?;
    Ok(())
}
```

#### Async Patterns

- Use structured concurrency with `futures::future::try_join_all`
- Always handle timeouts with `tokio::time::timeout`
- Design for cancellation safety
- Use `tokio::spawn` for background tasks

#### Type State Pattern for GitHub Client

```rust
pub struct Unauthenticated;
pub struct Authenticated;

pub struct GitHubClient<State = Unauthenticated> {
    client: reqwest::Client,
    base_url: String,
    _state: PhantomData<State>,
}

impl GitHubClient<Unauthenticated> {
    pub async fn authenticate(self) -> Result<GitHubClient<Authenticated>> {
        // Verify GitHub CLI auth
        verify_gh_auth().await?;
        Ok(GitHubClient {
            client: self.client,
            base_url: self.base_url,
            _state: PhantomData,
        })
    }
}

// Only authenticated clients can make API calls
impl GitHubClient<Authenticated> {
    pub async fn create_project_item(&self, item: &ProjectItem) -> Result<String> {
        // Implementation
    }
}
```

### Testing Strategy

- **Unit Tests**: Required for all public functions
- **Integration Tests**: For GitHub API interactions (with real test projects)
- **High Coverage**: Target 90%+ coverage
- **Test Organization**: Group tests by functionality
- **Real Integration Testing**: Use actual GitHub API with test projects

#### Real Integration Testing

Integration tests use real GitHub Projects:

```rust
#[tokio::test]
async fn test_real_github_sync() {
    // Uses actual test GitHub project
    let client = GitHubClient::new().authenticate().await?;
    let test_project = get_test_project_id();
    let result = client.sync_tasks(&tasks, test_project).await?;
    assert!(result.synced_count > 0);
}
```

### Code Organization

``
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library root
├── models/           # Data structures
│   ├── task.rs       # Taskmaster task models
│   ├── github.rs     # GitHub API models
│   └── config.rs     # Configuration models
├── auth.rs           # GitHub authentication
├── config.rs         # Configuration management
├── sync.rs           # Core sync engine
├── github.rs         # GitHub API client
├── taskmaster.rs     # Taskmaster file reader
├── fields.rs         # Field mapping logic
├── subtasks.rs       # Subtask handling
├── progress.rs       # Progress tracking
├── watcher.rs        # File watching
├── error.rs          # Error types
└── state.rs          # Sync state tracking

```

## Key Files & Structure

### Important Files

- `Cargo.toml` - Dependencies and project metadata
- `src/main.rs` - CLI interface with clap
- `src/models/` - Type-safe data models
- `.github/workflows/` - CI/CD pipelines
- `tests/` - Integration tests

### Configuration

- Configuration files for runtime config
- Environment variables for GitHub authentication
- Cross-platform binary distribution

### GitHub CLI Authentication Note

**Important**: When working with GitHub CLI operations in this project, use Jonathan Julian's zsh profile for authentication. This ensures proper GitHub CLI access for all operations.

## Development Workflow

### Testing Philosophy

**IMPORTANT: Always complete regression testing and manual testing before marking any feature as complete.**

**GitHub CLI Authentication**: Always ensure proper authentication by using the correct Zsh profile with Jonathan Julian user before running any GitHub operations.

Before declaring a feature done:
1. **Unit Tests**: Ensure all unit tests pass
2. **Integration Tests**: Run relevant integration tests
3. **Manual Testing**: Test the feature manually with real data
4. **Regression Testing**: Verify existing functionality still works
5. **Edge Cases**: Test error conditions and edge cases

### Testing Commands

```bash
# Run all tests
cargo test --all-features

# Run with coverage
cargo tarpaulin --out Html --target-dir target/tarpaulin

# View coverage report
open target/tarpaulin/tarpaulin-report.html

# Run specific test
cargo test test_name

# Run integration tests only
cargo test --test '*'
```

### Quality Checks

```bash
# Format code
cargo fmt

# Run linter (pedantic level)
cargo clippy --all-targets --all-features -- -D clippy::pedantic -A clippy::module_name_repetitions

# Security audit
cargo audit

# Check for outdated dependencies
cargo outdated
```

### Build & Deploy

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Cross-compilation for distribution
cargo build --target x86_64-unknown-linux-gnu --release
cargo build --target x86_64-apple-darwin --release
cargo build --target aarch64-apple-darwin --release
```

## Current Focus Areas

### Active Priorities

1. Complete core sync functionality (Tasks 7-15)
2. Implement agent assignment system
3. Add comprehensive error handling
4. Optimize GitHub API performance

### Known Patterns

- Use `thiserror` for error types
- Use `anyhow` for application errors
- Prefer `tokio` for async runtime
- Use `tracing` for structured logging
- Use `clap` for CLI interface

## Context for AI Assistance

### Preferred Coding Style

- Explicit over implicit
- Early returns with `?`
- Builder pattern for complex types
- Newtype pattern for domain types

### Design Principles

1. **Type Safety First**: Make invalid states unrepresentable
2. **Performance by Default**: Zero-cost abstractions
3. **Testability**: Design for testing from the start
4. **CLI UX**: Clear, helpful command-line interface

### Common Tasks

#### Adding a New CLI Command

1. Add command to `main.rs` clap configuration
2. Implement handler function
3. Add appropriate error handling
4. Add integration tests
5. Update documentation

#### Adding GitHub API Integration

1. Define models in `src/models/github.rs`
2. Add API methods to `GitHubClient`
3. Handle rate limiting and errors
4. Add comprehensive tests
5. Update field mappings

### Practical Workflows

#### Complete Feature Implementation (2-3 tool calls)

1. **Plan** (0 tool calls)
   - Design approach
   - Identify affected files
   - Plan test strategy

2. **Implement** (1-2 tool calls)

   ```rust
   // Implement feature + tests together
   // Focus on type safety and error handling
   ```

3. **Validate** (1 tool call)

   ```bash
   cargo fmt && cargo clippy && cargo test
   ```

#### Efficient Debugging Workflow

```bash
# Single call for comprehensive analysis
echo "=== Compilation Check ===" && \
cargo check 2>&1 | head -50 && \
echo "=== Test Results ===" && \
cargo test --lib -- --nocapture 2>&1 | grep -A5 -B5 "FAILED\|Error" && \
echo "=== Recent Changes ===" && \
git diff HEAD~1 --stat
```

### Areas Requiring Caution

- **GitHub API Rate Limits**: Always implement backoff
- **File System Operations**: Handle permissions and missing files
- **JSON Parsing**: Validate Taskmaster file format
- **CLI Error Messages**: Make them helpful for users

### Common Pitfalls & Solutions

#### GitHub API Error Handling

```rust
// ✅ DO: Handle rate limiting gracefully
async fn github_request_with_retry<T>(
    client: &GitHubClient,
    request: impl Fn() -> BoxFuture<'_, Result<T>>,
) -> Result<T> {
    let mut retries = 0;
    loop {
        match request().await {
            Ok(result) => return Ok(result),
            Err(e) if is_rate_limited(&e) && retries < 3 => {
                let delay = Duration::from_secs(2_u64.pow(retries));
                tokio::time::sleep(delay).await;
                retries += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

#### File System Safety

```rust
// ✅ DO: Validate file paths and handle errors
pub async fn read_tasks_file(path: &Path) -> Result<TaskmasterFile> {
    if !path.exists() {
        return Err(TaskMasterError::ConfigError {
            reason: format!("Tasks file not found at: {}", path.display()),
        });
    }

    let content = fs::read_to_string(path).await?;
    let tasks: TaskmasterFile = serde_json::from_str(&content)
        .map_err(|e| TaskMasterError::InvalidTaskFormat(e.to_string()))?;

    validate_tasks(&tasks)?;
    Ok(tasks)
}
```

## Tool Usage Efficiency & Cost Optimization

### Tool Call Budgets by Task Type

- **Bug Fix**: 1-2 tool calls maximum
- **New Feature**: 2-3 tool calls
- **CLI Command**: 2-3 tool calls
- **GitHub Integration**: 3-4 tool calls

### Batch Operation Examples

```bash
# ✅ EFFICIENT: Complete quality check in one call
cargo fmt && cargo clippy --all-targets --all-features && cargo test

# ❌ INEFFICIENT: Multiple separate calls
cargo fmt
cargo clippy
cargo test
```

### Background Execution Pattern

```bash
# ✅ DO: Run ALL commands in background mode
cargo build --release
cargo test --all-features
cargo clippy --all-targets --all-features
```

## Project-Specific Conventions

### Naming Conventions

- CLI Commands: `kebab-case` (e.g., `list-tags`, `sync-tasks`)
- Types: `PascalCase`
- Functions/Variables: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

### File Organization

- One major type per file
- Group related small types
- Tests in same file for unit tests
- Integration tests in `tests/` directory

### CLI Design Patterns

```rust
// ✅ DO: Provide helpful error messages
if !tasks_file.exists() {
    eprintln!("Error: Tasks file not found at {}", tasks_file.display());
    eprintln!("Hint: Make sure you're in a Taskmaster project directory");
    eprintln!("      or use --project-root to specify the location");
    std::process::exit(1);
}

// ✅ DO: Show progress for long operations
let pb = ProgressBar::new(tasks.len() as u64);
pb.set_style(ProgressStyle::default_bar()
    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
    .progress_chars("##-"));

for task in tasks {
    pb.set_message(format!("Syncing task {}", task.id));
    sync_task(&client, &task).await?;
    pb.inc(1);
}
pb.finish_with_message("Sync complete!");
```

## Performance & Security

### Optimization Guidelines

- Use connection pooling for GitHub API
- Batch GitHub API requests when possible
- Cache project metadata between syncs
- Use streaming for large task lists

### Security Best Practices

```rust
// ✅ DO: Sanitize data before logging
tracing::info!(task_id = %task.id, "Syncing task");
// Never log: GitHub tokens, sensitive task content

// ✅ DO: Validate all inputs
pub fn validate_project_number(num: i32) -> Result<i32> {
    if num <= 0 {
        return Err(TaskMasterError::ConfigError {
            reason: "Project number must be positive".to_string(),
        });
    }
    Ok(num)
}
```

### GitHub API Efficiency

```rust
// ✅ DO: Batch operations when possible
pub async fn sync_multiple_tasks(
    client: &GitHubClient,
    tasks: &[Task],
    project_id: &str,
) -> Result<SyncResult> {
    // Group tasks for batch operations
    let chunks: Vec<_> = tasks.chunks(10).collect();

    let results = futures::future::try_join_all(
        chunks.into_iter().map(|chunk| {
            sync_task_batch(client, chunk, project_id)
        })
    ).await?;

    Ok(SyncResult::from_batch_results(results))
}
```

## Current Implementation Status

### Completed Features ✅

- [x] Project structure and CLI framework
- [x] Taskmaster file reader with tagged format support
- [x] GitHub CLI authentication wrapper
- [x] GitHub GraphQL API client
- [x] Configuration management system
- [x] Field mapping infrastructure

### In Progress 🚧

- [ ] Core sync engine (Task 7)
- [ ] Agent assignment system
- [ ] File watching capability
- [ ] Progress tracking and reporting

### Planned Features 📋

- [ ] Real-time sync with file watching
- [ ] Advanced error recovery
- [ ] Performance optimization
- [ ] Comprehensive CLI help system

## Testing Requirements

### Integration Test Setup

```rust
// tests/github_integration.rs
#[tokio::test]
async fn test_full_sync_workflow() {
    let test_config = load_test_config();
    let client = GitHubClient::new().authenticate().await?;

    // Use real test GitHub project
    let project_id = test_config.test_project_id;
    let tasks = load_test_tasks();

    let result = sync_tasks(&client, &tasks, &project_id).await?;
    assert_eq!(result.success_count, tasks.len());
}
```

### Test Data Management

- Use real GitHub test projects
- Maintain test task files
- Clean up test data after runs
- Use environment variables for test configuration

## Monitoring & Observability

### Tracing Setup

```rust
use tracing::{info, instrument, warn};

// Initialize tracing for CLI
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .with_target(false)
    .init();

// Instrument key functions
#[instrument(skip(client), fields(project_id = %project_id))]
pub async fn sync_tasks(
    client: &GitHubClient,
    tasks: &[Task],
    project_id: &str,
) -> Result<SyncResult> {
    info!(task_count = tasks.len(), "Starting sync");
    // Function automatically traced
}
```

### CLI Progress Reporting

```rust
use indicatif::{ProgressBar, ProgressStyle};

pub fn create_sync_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-")
    );
    pb
}
```

## Task-Specific Implementation Guide

### Current Task Context (Task 7+)

We're implementing the core sync functionality. Key areas:

1. **State Tracking**: Track which tasks have been synced
2. **Conflict Resolution**: Handle GitHub vs Taskmaster differences
3. **Error Recovery**: Graceful handling of partial failures
4. **Performance**: Efficient GitHub API usage

### Implementation Priorities

1. Get basic sync working end-to-end
2. Add comprehensive error handling
3. Implement progress tracking
4. Add agent assignment logic
5. Optimize for performance

Remember: This project emphasizes reliability and user experience. Always provide clear error messages and progress feedback for CLI operations.

## GitHub Integration Lessons Learned

### GitHub CLI Authentication

**Critical Points for GitHub CLI Authentication:**

1. **Environment Variables vs Keyring**: 
   - The GITHUB_TOKEN environment variable takes precedence over keyring authentication
   - Always `unset GITHUB_TOKEN` before switching accounts with `gh auth switch`
   - The tool relies on GitHub CLI (`gh`) for authentication, not direct token usage

2. **Multiple Accounts**:
   - GitHub CLI can store multiple account credentials
   - Use `gh auth switch -u <username>` to switch between accounts
   - For this project, always use the JonathonJulian account for proper access

3. **Required Scopes**:
   - Project operations require the `project` scope
   - Repository operations require the `repo` scope
   - Use `gh auth refresh -h github.com -s project,repo` to update scopes

4. **Authentication Workflow**:
   ```bash
   # Clear any environment tokens
   unset GITHUB_TOKEN
   
   # Switch to correct account
   gh auth switch -u JonathonJulian
   
   # Verify authentication
   gh auth status
   
   # Refresh scopes if needed
   gh auth refresh -h github.com -s project,repo,read:project
   ```

### GitHub Projects Field Mapping

**Critical Discovery: Assignee vs Agent Field**

1. **The Problem**:
   - TaskMaster has an `assignee` field containing usernames like "swe-1-5dlabs"
   - Initially mapped this to a field called "Assignee" in GitHub Projects
   - This failed because GitHub Projects uses "Agent" for custom assignee fields

2. **GitHub Project Fields**:
   - Built-in fields: "Assignees" (for GitHub issue assignees)
   - Custom fields we create: "Agent" (for TaskMaster assignee mapping)
   - The confusion: "Assignees" is built-in, "Agent" is our custom field

3. **Correct Field Mapping**:
   ```rust
   // In fields.rs - INCORRECT
   self.field_mappings.insert(
       "assignee".to_string(),
       FieldMapping {
           taskmaster_field: "assignee".to_string(),
           github_field: "Assignee".to_string(),  // WRONG!
           field_type: GitHubFieldType::Text,
           transformer: None,
       },
   );
   
   // CORRECT - Should map to "Agent"
   self.field_mappings.insert(
       "assignee".to_string(),
       FieldMapping {
           taskmaster_field: "assignee".to_string(),
           github_field: "Agent".to_string(),  // CORRECT!
           field_type: GitHubFieldType::SingleSelect,
           transformer: None,
       },
   );
   ```

4. **Field Types Matter**:
   - "Agent" should be SingleSelect, not Text
   - This allows for dropdown selection in GitHub UI
   - Options must be created for each possible agent value

### Common GitHub API Errors and Solutions

1. **"Bad credentials" (401)**:
   - Usually means GITHUB_TOKEN is set but invalid
   - Solution: `unset GITHUB_TOKEN` and use keyring auth

2. **"Could not resolve to DraftIssue"**:
   - Happens when trying to update items created as real issues
   - DraftIssue mutations don't work on real repository issues
   - Need different update strategy for real issues vs draft issues

3. **"Field not found"**:
   - Check exact field names - they're case-sensitive
   - Use GraphQL to list available fields
   - Remember: built-in fields vs custom fields have different behaviors

4. **Rate Limiting**:
   - Implement exponential backoff
   - Batch operations when possible
   - Cache field IDs to reduce API calls

### Best Practices

1. **Always Test Field Mappings**:
   - List project fields before assuming field names
   - Verify field types match expected values
   - Test with small datasets first

2. **Handle Authentication Gracefully**:
   - Check gh auth status before operations
   - Provide clear error messages about auth issues
   - Document required GitHub CLI setup

3. **Debug GraphQL Errors**:
   - Use `gh api graphql` to test queries directly
   - Check node IDs are correct type (DraftIssue vs Issue)
   - Verify field IDs haven't changed

4. **State Management**:
   - Track whether items are draft issues or real issues
   - Store both project item ID and content ID
   - Handle different update paths accordingly
