# Taskmaster Sync - Monorepo Strategy

## Overview

This document outlines the recommended strategy for using taskmaster-sync in a large monorepo with multiple microservices, infrastructure components, and multiple AI agents.

## Architecture Pattern

```
monorepo/
├── services/
│   ├── api-gateway/
│   │   └── .taskmaster/tasks/tasks.json
│   ├── auth-service/
│   │   └── .taskmaster/tasks/tasks.json
│   └── data-service/
│       └── .taskmaster/tasks/tasks.json
├── infrastructure/
│   └── .taskmaster/tasks/tasks.json
├── shared/
│   └── .taskmaster/tasks/tasks.json
└── .github/
    └── workflows/
        └── taskmaster-sync.yml
```

## Multi-Service Sync Configuration

### 1. Service-Scoped Projects

Each major component can have its own GitHub Project:

```json
{
  "version": "1.0.0",
  "organization": "your-org",
  "project_mappings": {
    "api-gateway": {
      "project_number": 1,
      "project_id": "auto",
      "repository": "your-org/monorepo",
      "path_prefix": "services/api-gateway",
      "subtask_mode": "nested"
    },
    "auth-service": {
      "project_number": 2,
      "project_id": "auto",
      "repository": "your-org/monorepo",
      "path_prefix": "services/auth-service",
      "subtask_mode": "nested"
    }
  }
}
```

### 2. Unified Project with Labels

Alternatively, use one project with service labels:

```json
{
  "version": "1.0.0",
  "organization": "your-org",
  "project_mappings": {
    "main": {
      "project_number": 1,
      "project_id": "auto",
      "repository": "your-org/monorepo",
      "label_strategy": "path-based",
      "label_mapping": {
        "services/api-gateway": "API Gateway",
        "services/auth-service": "Auth Service",
        "infrastructure": "Infrastructure"
      }
    }
  }
}
```

## Agent Coordination Strategies

### 1. Service-Based Agent Assignment

```yaml
# Enhanced workflow with service detection
- name: Detect Changed Services
  id: changes
  run: |
    # Get changed files in PR
    CHANGED_FILES=$(git diff --name-only origin/main...HEAD)

    # Detect which services were modified
    SERVICES=""
    if echo "$CHANGED_FILES" | grep -q "services/api-gateway"; then
      SERVICES="$SERVICES api-gateway"
    fi
    if echo "$CHANGED_FILES" | grep -q "services/auth-service"; then
      SERVICES="$SERVICES auth-service"
    fi

    echo "services=$SERVICES" >> $GITHUB_OUTPUT

- name: Sync Changed Services
  run: |
    for service in ${{ steps.changes.outputs.services }}; do
      ./target/release/taskmaster-sync sync \
        --config sync-config.json \
        --tag "$service-${{ github.event.pull_request.head.ref }}" \
        --path "services/$service/.taskmaster/tasks/tasks.json" \
        --direction to-github \
        --delta
    done
```

### 2. Lock File Strategy

Prevent race conditions with distributed locking:

```rust
// In your taskmaster-sync code
pub struct DistributedLock {
    project_id: String,
    lock_item_id: Option<String>,
}

impl DistributedLock {
    pub async fn acquire(&mut self, github: &GitHubAPI, timeout: Duration) -> Result<()> {
        let lock_title = format!("🔒 SYNC_LOCK_{}", Utc::now().timestamp());
        // Create a special issue/item as a lock
        // Check for existing locks before proceeding
    }
}
```

### 3. Event-Driven Sync

Use GitHub webhooks for real-time sync:

```yaml
# Webhook handler workflow
name: Taskmaster Webhook Sync
on:
  repository_dispatch:
    types: [taskmaster-update]

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - name: Sync Specific Task
        run: |
          ./target/release/taskmaster-sync sync-item \
            --task-id "${{ github.event.client_payload.task_id }}" \
            --service "${{ github.event.client_payload.service }}" \
            --action "${{ github.event.client_payload.action }}"
```

## Best Practices for Multi-Agent Monorepos

### 1. Branch Naming Conventions

```
feature/api-gateway/add-auth
feature/auth-service/jwt-tokens
feature/infra/kubernetes-setup
```

This allows automatic service detection and proper sync targeting.

### 2. Task ID Namespacing

Prefix task IDs with service identifiers:

```json
{
  "tasks": [
    {
      "id": "api-gateway-001",
      "title": "Add rate limiting"
    },
    {
      "id": "auth-service-001",
      "title": "Implement JWT refresh"
    }
  ]
}
```

### 3. Status Transition Rules

```json
{
  "transition_rules": {
    "api-gateway": {
      "done": {
        "next_status": "QA Review",
        "assign_to": "qa-agent-1"
      }
    },
    "auth-service": {
      "done": {
        "next_status": "Security Review",
        "assign_to": "security-agent"
      }
    }
  }
}
```

### 4. Sync Optimization

For large monorepos, use path filters:

```rust
// Only sync tasks that match the changed paths
pub fn filter_tasks_by_paths(tasks: Vec<Task>, changed_paths: Vec<String>) -> Vec<Task> {
    tasks.into_iter()
        .filter(|task| {
            // Check if task relates to changed paths
            changed_paths.iter().any(|path| {
                task.id.contains(&path.replace("/", "-"))
                || task.metadata.get("service") == Some(path)
            })
        })
        .collect()
}
```

## Recommended Workflow

1. **PR Creation**: Implementation agent creates PR
   - Sync only affected service tasks
   - Status: `done` → `QA Review`
   - Assign to appropriate QA agent

2. **QA Testing**: QA agent tests on feature branch
   - No sync needed during testing
   - QA agent updates local status

3. **PR Approval**: QA approves
   - Sync status to `Approved`
   - Trigger merge workflow

4. **Post-Merge**: Cleanup and final sync
   - Bidirectional sync on main
   - Archive completed tasks
   - Update project metrics

## Performance Considerations

1. **Parallel Service Syncs**: Run syncs for different services in parallel
2. **Caching**: Cache project structure between syncs
3. **Incremental Updates**: Use delta sync for large task lists
4. **Batch Operations**: Group GraphQL mutations

## Example Full Configuration

```yaml
# .github/workflows/taskmaster-monorepo-sync.yml
name: Monorepo Taskmaster Sync

on:
  pull_request:
    types: [opened, synchronize, ready_for_review]

env:
  SERVICES: |
    api-gateway:1
    auth-service:2
    data-service:3
    infrastructure:4

jobs:
  detect-changes:
    runs-on: ubuntu-latest
    outputs:
      matrix: ${{ steps.set-matrix.outputs.matrix }}
    steps:
      - uses: actions/checkout@v4
      - id: set-matrix
        run: |
          # Detect which services changed
          # Output matrix for parallel jobs

  sync-services:
    needs: detect-changes
    runs-on: ubuntu-latest
    strategy:
      matrix: ${{ fromJson(needs.detect-changes.outputs.matrix) }}
    steps:
      - name: Sync Service Tasks
        run: |
          # Service-specific sync
```

This approach ensures smooth multi-agent collaboration in your monorepo while preventing conflicts and race conditions.