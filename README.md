# Task Master Sync

A high-performance Rust CLI tool that synchronizes [Taskmaster](https://github.com/taskmaster-ai/taskmaster) tasks with GitHub Projects for visual project management and team collaboration.

## ✨ Features

- 🚀 **High Performance** - Written in Rust for blazing-fast synchronization
- 🔄 **Delta Sync** - Intelligent sync with state tracking to minimize API calls
- 📊 **GitHub Projects Integration** - Full support for GitHub Projects v2 with custom fields
- 🏷️ **Multi-tag Support** - Map different Taskmaster tags to different projects
- 👥 **Agent Assignment** - Automatically assign tasks to team members
- 📈 **Progress Tracking** - Visual progress bars and detailed sync statistics
- 🎯 **Flexible Subtask Handling** - Preserves task hierarchies in GitHub
- 💾 **Zero Runtime Dependencies** - Single binary executable
- 🛡️ **Memory Safe** - Rust's ownership system ensures reliability
- ⚡ **Cross Platform** - Native binaries for macOS and Linux

## 🎉 Current Status

**Version 0.0.1 Released!** The first release provides core functionality:

- ✅ **Complete sync engine** with delta sync support
- ✅ **GitHub Projects V2 API** integration
- ✅ **Custom field mapping** (TM_ID, Agent, Status, Priority, etc.)
- ✅ **Subtask hierarchy** support
- ✅ **Configuration management** with project mappings
- ✅ **Progress tracking** with detailed statistics
- ✅ **Comprehensive error handling** and retry logic
- ✅ **Project setup automation** with field creation
- ✅ **Duplicate detection** and cleanup utilities
- 🚧 Real-time file watching (coming in v0.1.0)
- 🚧 Advanced agent assignment rules (coming in v0.1.0)

## 📋 Prerequisites

- **GitHub CLI** (`gh`) installed and authenticated
- **Taskmaster** project with `.taskmaster/tasks/tasks.json`
- **GitHub Projects v2** access in your organization
- **Rust** (for building from source)

## 🛠️ Installation

### Option 1: As a GitHub Action (Recommended for CI/CD)

The easiest way to use taskmaster-sync in your projects is as a GitHub Action:

```yaml
- uses: 5dlabs/taskmaster-sync@v1
  with:
    direction: to-github
    delta: true
```

See [.github/workflows/example-usage.yml.example](.github/workflows/example-usage.yml.example) for complete workflow examples.

### Option 2: Download Pre-built Binary (For Local Use)

Download the latest release from [GitHub Releases](https://github.com/5dlabs/taskmaster-sync/releases/latest):

```bash
# macOS (Intel)
curl -L https://github.com/5dlabs/taskmaster-sync/releases/download/v0.0.1/task-master-sync-darwin-x86_64.tar.gz | tar xz
chmod +x task-master-sync
sudo mv task-master-sync /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/5dlabs/taskmaster-sync/releases/download/v0.0.1/task-master-sync-darwin-aarch64.tar.gz | tar xz
chmod +x task-master-sync
sudo mv task-master-sync /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/5dlabs/taskmaster-sync/releases/download/v0.0.1/task-master-sync-linux-x86_64.tar.gz | tar xz
chmod +x task-master-sync
sudo mv task-master-sync /usr/local/bin/
```

### Option 3: Build from Source

```bash
# Clone the repository
git clone https://github.com/5dlabs/taskmaster-sync
cd taskmaster-sync

# Build release binary
cargo build --release

# Install globally
sudo cp target/release/task-master-sync /usr/local/bin/

# Or run directly
./target/release/task-master-sync --help
```

## 🚀 Quick Start

1. **Authenticate with GitHub**:
   ```bash
   gh auth login
   ```

2. **Create and set up a GitHub Project**:
   ```bash
   # Create a new project
   task-master-sync create-project "My TaskMaster Project" --org your-org

   # Set up required fields (replace 123 with your project number)
   task-master-sync setup-project 123
   ```

3. **Navigate to your Taskmaster project**:
   ```bash
   cd your-taskmaster-project
   ls .taskmaster/tasks/tasks.json  # Verify tasks file exists
   ```

4. **Sync tasks to GitHub Project**:
   ```bash
   task-master-sync sync master 123  # Replace 123 with your project number
   ```

5. **Monitor sync progress**:
   ```bash
   # Check sync status
   task-master-sync status

   # Clean up any duplicates
   task-master-sync clean-duplicates 123 --delete
   ```

## 📖 Usage

### Core Commands

```bash
# Sync specific tag to GitHub Project
task-master-sync sync <TAG> <PROJECT_NUMBER> [--dry-run] [--full-sync]

# Create and set up a new GitHub Project
task-master-sync create-project <TITLE> [--org <ORG>] [--description <DESC>]
task-master-sync setup-project <PROJECT_NUMBER>

# Clean up duplicate items in a project
task-master-sync clean-duplicates <PROJECT_NUMBER> [--delete]

# Show current sync status (coming in v0.1.0)
task-master-sync status [--project <PROJECT_NUMBER>]

# Watch for changes and auto-sync (coming in v0.1.0)
task-master-sync watch <TAG> <PROJECT_NUMBER> [--debounce <MS>]

# List available Taskmaster tags (coming in v0.1.0)
task-master-sync list-tags

# Configure project mappings (coming in v0.1.0)
task-master-sync configure --project <PROJECT_NUMBER> --tag <TAG>
```

### Sync Options

```bash
# Preview changes without applying them
task-master-sync sync master 123 --dry-run

# Create subtasks as separate GitHub items
task-master-sync sync master 123 --subtasks-as-items

# Include subtasks in parent task description (default)
task-master-sync sync master 123 --subtasks-in-body

# Enable verbose logging
task-master-sync sync master 123 --verbose
```

### Watch Mode

```bash
# Watch with custom debounce time (default: 1000ms)
task-master-sync watch master 123 --debounce 2000

# Watch with verbose logging
task-master-sync watch master 123 --verbose
```

## ⚙️ Configuration

Task Master Sync stores configuration in `.taskmaster/sync-config.json`:

```json
{
  "version": "1.0.0",
  "organization": "your-org",
  "project_mappings": {
    "master": {
      "project_number": 123,
      "project_id": "PVT_kwDOAM8-ec4AKnVx",
      "subtask_mode": "nested",
      "last_sync": "2024-01-15T10:30:00Z",
      "field_mappings": {
        "status": "Status",
        "priority": "Priority",
        "assignee": "Assignee",
        "dependencies": "Dependencies"
      }
    },
    "feature-auth": {
      "project_number": 124,
      "project_id": "PVT_kwDOAM8-ec4AKnVy",
      "subtask_mode": "separate"
    }
  },
  "agent_mapping": {
    "swe-1": {
      "github_username": "swe-1-5dlabs",
      "services": ["taskmaster-sync", "copy-trader", "live-trader", "analytics"],
      "role": "Senior Software Engineer"
    },
    "swe-2": {
      "github_username": "SWE-2-5dlabs",
      "services": ["paper-trader", "portfolio-manager", "risk-manager"],
      "role": "Senior Software Engineer"
    },
    "qa": {
      "github_username": "qa0-5dlabs",
      "services": ["*"],
      "role": "Quality Assurance Engineer"
    }
  }
}
```

## 🗂️ Field Mappings

Task Master Sync automatically creates and maps these fields in GitHub Projects:

| Taskmaster Field | GitHub Project Field | Type | Description |
|-----------------|---------------------|------|-------------|
| `id` | TM_ID | Text | Unique Taskmaster task identifier |
| `status` | Status | Single Select | Task status (pending, in-progress, done, etc.) |
| `priority` | Priority | Single Select | Task priority (high, medium, low) |
| `assignee` | Assignee | Text | GitHub username of assigned team member |
| `dependencies` | Dependencies | Text | Comma-separated list of dependent task IDs |
| `testStrategy` | Test Strategy | Text | Testing approach and validation steps |

## 👥 Agent Assignment

Configure automatic task assignment based on service ownership and task patterns:

```json
{
  "assignmentRules": {
    "rules": [
      {
        "name": "Service-based Assignment",
        "condition": "task.service || task.tags?.includes('service:') || task.title.includes('[')",
        "logic": "serviceMapping",
        "priority": 1
      },
      {
        "name": "QA and Testing Tasks",
        "condition": "task.title.toLowerCase().includes('test')",
        "assignTo": "qa",
        "priority": 2
      }
    ]
  },
  "serviceMapping": {
    "services": {
      "taskmaster-sync": { "owner": "swe-1" },
      "copy-trader": { "owner": "swe-1" },
      "paper-trader": { "owner": "swe-2" },
      "portfolio-manager": { "owner": "swe-2" }
    }
  }
}
```

Tasks are automatically assigned based on:
- **Service ownership** - Tasks tagged with `[service-name]` or `service:name`
- **Task type** - Testing tasks go to QA, project tasks go to PM
- **Pattern matching** - Custom rules based on task content

## 🏗️ Architecture

Task Master Sync uses a modular Rust architecture:

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── models/              # Data structures
│   ├── task.rs          # Taskmaster task models
│   ├── github.rs        # GitHub API models
│   └── config.rs        # Configuration models
├── auth.rs              # GitHub authentication
├── config.rs            # Configuration management
├── sync.rs              # Core sync engine
├── github.rs            # GitHub API client
├── taskmaster.rs        # Taskmaster file reader
├── fields.rs            # Field mapping logic
├── subtasks.rs          # Subtask handling
├── progress.rs          # Progress tracking
├── watcher.rs           # File watching
└── error.rs             # Error types
```

**Key Design Principles:**
- **Async I/O** - Tokio runtime for concurrent operations
- **Type Safety** - Rust's type system prevents runtime errors
- **Error Handling** - Comprehensive error context with `anyhow`
- **Structured Logging** - `tracing` for observability
- **Zero-copy** - Efficient string and data handling

## 📊 Performance

Expected performance characteristics:

- **Sync Speed** - 100+ tasks in under 30 seconds
- **Memory Usage** - < 50MB RAM footprint
- **Binary Size** - ~2.2MB release binary
- **Startup Time** - < 100ms cold start
- **API Efficiency** - Batched operations and concurrent requests

## 🔧 Development

### Building

```bash
# Development build (faster compilation, larger binary)
cargo build

# Release build (optimized, smaller binary)
cargo build --release

# Run with logging
RUST_LOG=debug cargo run -- sync master 123
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for security vulnerabilities
cargo audit
```

### Auto-Formatting Setup

**Automatic CI Formatting:**
- GitHub Actions automatically formats code on every PR
- Commits formatting changes back to the PR if needed
- Uses `[skip ci]` to avoid infinite loops

**Local Pre-Commit Hook:**
```bash
# Install pre-commit hook for automatic formatting
./scripts/setup-hooks.sh

# The hook will automatically:
# • Format code with cargo fmt
# • Run clippy linting
# • Run tests
# • Add formatted files to commit

# To bypass temporarily (not recommended):
git commit --no-verify
```

### Debugging

```bash
# Enable debug logging
RUST_LOG=debug task-master-sync sync master 123

# Enable trace logging (very verbose)
RUST_LOG=trace task-master-sync sync master 123

# Preview operations without executing
task-master-sync sync master 123 --dry-run
```

## 🐛 Troubleshooting

### Authentication Issues

```bash
# Check GitHub CLI authentication
gh auth status

# Re-authenticate if needed
gh auth login --web
```

### File Not Found Errors

```bash
# Verify Taskmaster project structure
ls .taskmaster/tasks/tasks.json

# Check if in correct directory
pwd

# Initialize Taskmaster if needed
task-master init
```

### Permission Issues

```bash
# Verify GitHub permissions
gh api user

# Check organization access
gh api orgs/your-org/projects

# Verify project access
gh project view 123
```

### Performance Issues

```bash
# Check file sizes
ls -lh .taskmaster/tasks/tasks.json

# Monitor API rate limits
gh api rate_limit

# Use dry-run to test without API calls
task-master-sync sync master 123 --dry-run
```

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Fork** the repository
2. **Create** your feature branch (`git checkout -b feature/amazing-feature`)
3. **Make** your changes
4. **Test** thoroughly (`cargo test`)
5. **Format** code (`cargo fmt`)
6. **Lint** code (`cargo clippy`)
7. **Commit** changes (`git commit -m 'Add amazing feature'`)
8. **Push** to branch (`git push origin feature/amazing-feature`)
9. **Open** a Pull Request

### Development Guidelines

- Follow Rust naming conventions
- Add tests for new functionality
- Update documentation for API changes
- Use `cargo clippy` to catch common issues
- Ensure `cargo fmt` is run before committing

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built for [Taskmaster](https://github.com/taskmaster-ai/taskmaster) users
- Powered by the GitHub GraphQL API v4
- Written in Rust for reliability and performance
- Inspired by the need for visual project management in AI-driven development

## 📝 Release History

### v0.0.1 (2025-06-29)
- 🎉 **Initial Release**
- ✅ Core sync engine with delta sync support
- ✅ GitHub Projects V2 API integration
- ✅ Custom field mapping (TM_ID, Agent, Status, Priority)
- ✅ Subtask hierarchy preservation
- ✅ Project creation and setup automation
- ✅ Duplicate detection and cleanup
- ✅ Comprehensive error handling
- ✅ Cross-platform binaries (Linux, macOS)

### Roadmap for v0.1.0
- 🚧 Real-time file watching with auto-sync
- 🚧 Advanced agent assignment rules
- 🚧 Status command for sync monitoring
- 🚧 Tag listing and management
- 🚧 Interactive configuration wizard

## 🔗 Related Projects

- [Taskmaster](https://github.com/taskmaster-ai/taskmaster) - AI-powered task management
- [GitHub CLI](https://cli.github.com/) - GitHub command line tool
- [GitHub Projects](https://docs.github.com/en/issues/planning-and-tracking-with-projects) - Project management on GitHub

---

**Status**: ✅ Released | **Version**: 0.0.1 | **Language**: Rust 🦀 | **License**: MIT