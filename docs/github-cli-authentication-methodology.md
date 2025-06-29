# GitHub CLI Authentication Methodology for TaskMaster Sync

## Overview
This document provides a standardized methodology for ensuring reliable GitHub CLI authentication for the TaskMaster Sync project. The project relies exclusively on GitHub CLI (`gh`) for all GitHub operations.

## Prerequisites
- GitHub CLI (`gh`) must be installed
- JonathonJulian account credentials must be available in the system keyring
- Required scopes: `project`, `repo`, `workflow`, `admin:org`, `admin:public_key`, `gist`

## Authentication Methodology

### 1. Pre-Authentication Checks

Always perform these checks before any GitHub operations:

```bash
# Check if GITHUB_TOKEN is set (it interferes with keyring auth)
if [ -n "$GITHUB_TOKEN" ]; then
    echo "WARNING: GITHUB_TOKEN is set. Unsetting to use keyring authentication..."
    unset GITHUB_TOKEN
fi

# Verify gh is installed
if ! command -v gh &> /dev/null; then
    echo "ERROR: GitHub CLI (gh) is not installed"
    exit 1
fi
```

### 2. Switch to Correct Account

```bash
# Switch to JonathonJulian account (required for this project)
gh auth switch -u JonathonJulian

# Verify the switch was successful
if ! gh auth status | grep -q "Active account: true.*JonathonJulian"; then
    echo "ERROR: Failed to switch to JonathonJulian account"
    exit 1
fi
```

### 3. Verify Authentication and Scopes

```bash
# Check authentication status
gh auth status

# Verify required scopes are available
REQUIRED_SCOPES=("project" "repo" "workflow")
CURRENT_SCOPES=$(gh auth status | grep "Token scopes:" | head -1)

for scope in "${REQUIRED_SCOPES[@]}"; do
    if [[ ! "$CURRENT_SCOPES" =~ "$scope" ]]; then
        echo "WARNING: Missing required scope: $scope"
        echo "Run: gh auth refresh -h github.com -s project,repo,workflow"
    fi
done
```

### 4. Test Authentication

```bash
# Test with a simple API call
if ! gh api user --jq .login | grep -q "JonathonJulian"; then
    echo "ERROR: Authentication test failed"
    exit 1
fi
```

## Complete Authentication Script

```bash
#!/bin/bash
# github-auth-check.sh - Ensure proper GitHub CLI authentication

set -e

echo "🔐 Checking GitHub CLI authentication..."

# Clear any environment token
unset GITHUB_TOKEN

# Check gh installation
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) is not installed"
    echo "Install with: brew install gh"
    exit 1
fi

# Switch to correct account
echo "📝 Switching to JonathonJulian account..."
gh auth switch -u JonathonJulian

# Verify authentication
if gh auth status | grep -q "Active account: true.*JonathonJulian"; then
    echo "✅ Successfully authenticated as JonathonJulian"
else
    echo "❌ Authentication failed"
    echo "Run: gh auth login"
    exit 1
fi

# Check scopes
echo "🔍 Verifying OAuth scopes..."
SCOPES=$(gh auth status | grep -A1 "JonathonJulian" | grep "Token scopes:" | head -1)
if [[ "$SCOPES" =~ "project" ]] && [[ "$SCOPES" =~ "repo" ]]; then
    echo "✅ Required scopes are present"
else
    echo "⚠️  Missing required scopes"
    echo "Run: gh auth refresh -h github.com -s project,repo,workflow"
fi

echo "🚀 GitHub CLI authentication verified!"
```

## Important Note: Environment Token Interference

**GITHUB_TOKEN in Shell Configuration**: If GITHUB_TOKEN is set in your shell configuration files (e.g., ~/.zshrc, ~/.bashrc), it will persistently interfere with GitHub CLI keyring authentication. The token set in environment variables always takes precedence over keyring authentication.

**Solution**: Either:
1. Remove or comment out `export GITHUB_TOKEN=...` from your shell config files
2. Always run `unset GITHUB_TOKEN` before TaskMaster Sync operations
3. Use the provided wrapper scripts that handle this automatically

## Troubleshooting

### Common Issues and Solutions

1. **"Bad credentials" error**
   - Cause: GITHUB_TOKEN environment variable is set with invalid token
   - Solution: `unset GITHUB_TOKEN`

2. **"Not authenticated" error**
   - Cause: No active GitHub CLI session
   - Solution: `gh auth login`

3. **Wrong account active**
   - Cause: Different account is set as active
   - Solution: `gh auth switch -u JonathonJulian`

4. **Missing scopes**
   - Cause: Token doesn't have required permissions
   - Solution: `gh auth refresh -h github.com -s project,repo,workflow,admin:org,admin:public_key,gist`

5. **Multiple authentication methods conflict**
   - Cause: Both GITHUB_TOKEN and keyring auth present
   - Solution: Always `unset GITHUB_TOKEN` before operations

## Integration with TaskMaster Sync

The authentication check is integrated into the Rust code via `src/auth.rs`:

```rust
// The verify_gh_auth() function handles all authentication checks
pub async fn verify_gh_auth() -> Result<()> {
    // 1. Check gh installation
    // 2. Verify authentication status
    // 3. Extract username and validate it's JonathonJulian
    // 4. Check OAuth scopes
}
```

## Best Practices

1. **Always unset GITHUB_TOKEN**: Environment tokens override keyring auth
2. **Use keyring authentication**: More secure than environment variables
3. **Verify before operations**: Check auth status before any GitHub API calls
4. **Handle auth errors gracefully**: Provide clear error messages to users
5. **Document requirements**: Make auth requirements clear in README/docs

## Quick Reference

```bash
# One-liner to ensure proper auth
unset GITHUB_TOKEN && gh auth switch -u JonathonJulian && gh auth status
```