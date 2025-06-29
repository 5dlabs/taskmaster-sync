# TaskMaster Sync Regression Testing Process

## Overview
This document outlines the comprehensive regression testing process for TaskMaster Sync. All features must pass regression testing before being marked as complete or merged into main.

## Testing Philosophy
- **Complete Coverage**: Test all functionality, not just new changes
- **Real Integration**: Use actual GitHub test projects, not mocks
- **Manual Verification**: Automated tests plus manual validation
- **Edge Cases**: Test error conditions and boundary scenarios

## Pre-Testing Requirements

### 1. Environment Setup
```bash
# Ensure proper GitHub authentication
unset GITHUB_TOKEN && gh auth switch -u JonathonJulian

# Verify test project access
gh api graphql -f query='
  query {
    user(login: "JonathonJulian") {
      projectV2(number: 10) {
        id
        title
      }
    }
  }'

# Clean build environment
cargo clean
cargo build --release
```

### 2. Test Data Preparation
- Ensure `.taskmaster/tasks-tagged.json` contains valid test tasks
- Verify test project (ID: 10) is accessible and clean
- Check agent mappings in `.taskmaster/agent-mapping.json`

## Regression Testing Stages

### Stage 1: Unit Tests
```bash
# Run all unit tests with coverage
cargo test --lib --all-features -- --nocapture

# Generate coverage report
cargo tarpaulin --out Html --target-dir target/tarpaulin
open target/tarpaulin/tarpaulin-report.html

# Expected: All tests pass, >90% coverage
```

### Stage 2: Integration Tests
```bash
# Run all integration tests
cargo test --test '*' -- --nocapture

# Run specific integration test suites
cargo test --test github_integration -- --nocapture
cargo test --test sync_integration -- --nocapture
cargo test --test config_integration -- --nocapture
cargo test --test state_integration -- --nocapture
cargo test --test assignee_field_test -- --nocapture
```

### Stage 3: Linting and Code Quality
```bash
# Format check
cargo fmt -- --check

# Clippy with pedantic rules
cargo clippy --all-targets --all-features -- -D clippy::pedantic -A clippy::module_name_repetitions

# Security audit
cargo audit

# Check for outdated dependencies
cargo outdated
```

### Stage 4: Manual Functional Testing

#### 4.1 Configuration Commands
```bash
# Test init command
./target/release/taskmaster-sync init

# Test config display
./target/release/taskmaster-sync config

# Test auth verification
./target/release/taskmaster-sync auth
```

#### 4.2 Core Sync Functionality
```bash
# Dry run to preview changes
./target/release/taskmaster-sync sync --dry-run

# Full sync with progress
./target/release/taskmaster-sync sync

# Verify in GitHub UI
echo "Check: https://github.com/users/JonathonJulian/projects/10"
```

#### 4.3 Field Mapping Verification
Test each field mapping:
- Title → Title
- Description → Description
- Status → Status (todo/in_progress/done)
- Priority → Priority (P0/P1/P2/P3)
- Assignee → Agent (SingleSelect field)
- Tags → Labels
- Due dates → Due Date
- Subtasks → Subtasks field

#### 4.4 State Management
```bash
# Check sync state persists
cat .taskmaster/sync-state-master.json

# Modify a task and re-sync
# Verify only changed items update

# Test conflict resolution
# Manually modify in GitHub, then sync
```

#### 4.5 Error Handling
```bash
# Test with invalid project ID
./target/release/taskmaster-sync sync --project-number 99999

# Test with malformed tasks file
cp .taskmaster/tasks-tagged.json .taskmaster/tasks-tagged.json.backup
echo "invalid json" > .taskmaster/tasks-tagged.json
./target/release/taskmaster-sync sync
mv .taskmaster/tasks-tagged.json.backup .taskmaster/tasks-tagged.json

# Test without authentication
gh auth logout
./target/release/taskmaster-sync sync
gh auth login
```

### Stage 5: Performance Testing
```bash
# Time full sync operation
time ./target/release/taskmaster-sync sync

# Monitor memory usage
/usr/bin/time -l ./target/release/taskmaster-sync sync

# Test with large task sets (if available)
```

### Stage 6: Cross-Platform Testing
```bash
# Build for all targets
cargo build --target x86_64-unknown-linux-gnu --release
cargo build --target x86_64-apple-darwin --release
cargo build --target aarch64-apple-darwin --release

# Test on current platform
./target/release/taskmaster-sync --version
```

## Regression Test Checklist

### Core Functionality
- [ ] Authentication works with gh CLI
- [ ] Configuration initialization creates proper structure
- [ ] Tasks load correctly from tagged format
- [ ] GitHub project is accessible
- [ ] Field mappings work for all fields
- [ ] Assignee → Agent mapping works correctly
- [ ] Status mappings (todo/in_progress/done) work
- [ ] Priority mappings work
- [ ] Subtasks create properly
- [ ] Tags sync as labels
- [ ] Due dates sync correctly

### State Management
- [ ] Sync state persists between runs
- [ ] Only changed items update on subsequent syncs
- [ ] Deleted tasks are handled properly
- [ ] New tasks are created correctly
- [ ] Modified tasks update appropriately

### Error Handling
- [ ] Invalid authentication is caught
- [ ] Network errors are handled gracefully
- [ ] Malformed JSON is reported clearly
- [ ] Missing fields don't crash
- [ ] Rate limiting is respected
- [ ] Partial failures are recoverable

### Performance
- [ ] Sync completes in reasonable time (<30s for 100 tasks)
- [ ] Memory usage stays reasonable
- [ ] Progress bar shows accurate status
- [ ] No memory leaks detected

### User Experience
- [ ] Clear error messages
- [ ] Helpful progress indicators
- [ ] Dry-run mode works
- [ ] Help text is accurate
- [ ] CLI arguments work as documented

## Post-Testing Verification

### 1. GitHub Project Verification
Navigate to the GitHub project and verify:
- All tasks appear correctly
- Fields are populated appropriately
- Agent assignments show in dropdown
- Status columns contain correct items
- Subtasks are linked properly

### 2. Log Analysis
```bash
# Check for any warnings or errors
RUST_LOG=debug ./target/release/taskmaster-sync sync 2>&1 | grep -E "(WARN|ERROR)"

# Verify GraphQL queries are efficient
RUST_LOG=trace ./target/release/taskmaster-sync sync 2>&1 | grep "GraphQL"
```

### 3. State File Verification
```bash
# Ensure state files are valid JSON
jq . .taskmaster/sync-state-master.json
jq . .taskmaster/snapshots/master-snapshot.json

# Check no sensitive data is stored
grep -i "token\|password\|secret" .taskmaster/*.json
```

## Automated Regression Test Script

```bash
#!/bin/bash
# regression-test.sh - Complete regression test suite

set -e

echo "🧪 Starting TaskMaster Sync Regression Tests"

# 1. Environment check
echo "1️⃣ Checking environment..."
unset GITHUB_TOKEN
gh auth switch -u JonathonJulian
gh auth status

# 2. Clean build
echo "2️⃣ Clean build..."
cargo clean
cargo build --release

# 3. Unit tests
echo "3️⃣ Running unit tests..."
cargo test --lib --all-features

# 4. Integration tests
echo "4️⃣ Running integration tests..."
cargo test --test '*'

# 5. Code quality
echo "5️⃣ Code quality checks..."
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings

# 6. Functional tests
echo "6️⃣ Functional tests..."
./target/release/taskmaster-sync --version
./target/release/taskmaster-sync auth
./target/release/taskmaster-sync sync --dry-run

echo "✅ All regression tests passed!"
```

## Known Issues to Check

1. **Agent Field Mapping**: Ensure "assignee" maps to "Agent" (not "Assignee")
2. **Status Values**: Check status values match exactly (case-sensitive)
3. **Authentication**: GITHUB_TOKEN must be unset for keyring auth
4. **Field Types**: Agent field must be SingleSelect, not Text
5. **Draft Issues**: Different update logic for draft vs real issues

## Success Criteria

A PR is ready to merge when:
1. All automated tests pass
2. Manual testing checklist is complete
3. No regressions from previous functionality
4. Performance metrics are acceptable
5. GitHub project shows correct data
6. No new clippy warnings or formatting issues