# Task Master Sync

A high-performance Rust CLI tool that synchronizes [Taskmaster](https://github.com/taskmaster-ai/taskmaster) tasks with GitHub Projects for visual project management and team collaboration.

## ✨ Features

- 🚀 **High Performance** - Written in Rust for blazing-fast synchronization
- 🔄 **Real-time Sync** - File watching with automatic synchronization
- 📊 **GitHub Projects Integration** - Full support for GitHub Projects v2
- 🏷️ **Multi-tag Support** - Map different Taskmaster tags to different projects
- 👥 **Agent Assignment** - Automatically assign tasks to team members based on service ownership
- 📈 **Progress Tracking** - Visual progress bars and detailed status updates
- 🎯 **Flexible Subtask Handling** - Display subtasks as checklists or separate items
- 💾 **Zero Runtime Dependencies** - Single binary executable, no Node.js required
- 🛡️ **Memory Safe** - Rust's ownership system prevents common bugs
- ⚡ **Cross Platform** - Works on macOS, Linux, and Windows

## 🚧 Current Status

**This project is currently under active development.** The Rust implementation provides a solid foundation with:

- ✅ Complete CLI interface and command structure
- ✅ Modular architecture ready for implementation
- ✅ Proper error handling and logging framework
- ✅ File watching infrastructure
- ✅ GitHub API integration scaffolding
- 🚧 Core sync functionality (in progress)
- 🚧 Agent assignment system (in progress)

## 📋 Prerequisites

- **GitHub CLI** (`gh`) installed and authenticated
- **Taskmaster** project with `.taskmaster/tasks/tasks.json`
- **GitHub Projects v2** access in your organization
- **Rust** (for building from source)

## 🛠️ Installation

### Option 1: Build from Source (Current)

Since this is an active development project, building from source is currently the primary installation method:

```bash
# Clone the repository
git clone https://github.com/yourusername/task-master-sync
cd task-master-sync

# Build release binary
cargo build --release

# The binary will be available at:
./target/release/task-master-sync

# Optionally install globally
sudo cp target/release/task-master-sync /usr/local/bin/
```

### Option 2: Future Release Binaries

Pre-built binaries will be available for download once the implementation is complete:

```bash
# macOS (Intel)
curl -L https://github.com/yourusername/task-master-sync/releases/latest/download/task-master-sync-macos-x64 -o task-master-sync

# macOS (Apple Silicon)
curl -L https://github.com/yourusername/task-master-sync/releases/latest/download/task-master-sync-macos-arm64 -o task-master-sync

# Linux
curl -L https://github.com/yourusername/task-master-sync/releases/latest/download/task-master-sync-linux-x64 -o task-master-sync

# Make executable and install
chmod +x task-master-sync
sudo mv task-master-sync /usr/local/bin/
```

## 🚀 Quick Start

1. **Authenticate with GitHub**:
   ```bash
   gh auth login
   ```

2. **Navigate to your Taskmaster project**:
   ```bash
   cd your-taskmaster-project
   ls .taskmaster/tasks/tasks.json  # Verify tasks file exists
   ```

3. **Sync tasks to GitHub Project**:
   ```bash
   task-master-sync sync master 123  # Replace 123 with your project number
   ```

4. **Enable automatic sync** (recommended):
   ```bash
   task-master-sync watch master 123
   ```

## 📖 Usage

### Core Commands

```bash
# Sync specific tag to GitHub Project
task-master-sync sync <TAG> <PROJECT_NUMBER>

# Watch for changes and auto-sync
task-master-sync watch <TAG> <PROJECT_NUMBER> [--debounce <MS>]

# Show current sync status
task-master-sync status [--project <PROJECT_NUMBER>]

# List available Taskmaster tags
task-master-sync list-tags

# Configure project mappings
task-master-sync configure --project <PROJECT_NUMBER> --tag <TAG>

# Create new GitHub Project
task-master-sync create-project <TITLE> [--org <ORG>] [--description <DESC>] [--public]
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

## 🔗 Related Projects

- [Taskmaster](https://github.com/taskmaster-ai/taskmaster) - AI-powered task management
- [GitHub CLI](https://cli.github.com/) - GitHub command line tool
- [GitHub Projects](https://docs.github.com/en/issues/planning-and-tracking-with-projects) - Project management on GitHub

---

**Status**: 🚧 Under active development | **Version**: 0.1.0 | **Language**: Rust 🦀