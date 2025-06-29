#!/bin/bash
# Full regression test script for TaskMaster Sync
# This script ensures proper GitHub authentication and runs all regression tests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🧪 TaskMaster Sync Full Regression Test Suite${NC}"
echo "================================================"

# Function to check command success
check_status() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ $1${NC}"
    else
        echo -e "${RED}❌ $1 failed${NC}"
        exit 1
    fi
}

# 1. GitHub Authentication Check
echo -e "\n${YELLOW}1️⃣ GitHub CLI Authentication${NC}"
echo "--------------------------------"

# Clear any environment token
unset GITHUB_TOKEN
export GITHUB_TOKEN=""
echo "Cleared GITHUB_TOKEN environment variable"

# Also clear GH_TOKEN if set
unset GH_TOKEN
export GH_TOKEN=""

# Check gh installation
if ! command -v gh &> /dev/null; then
    echo -e "${RED}❌ GitHub CLI (gh) is not installed${NC}"
    echo "Install with: brew install gh"
    exit 1
fi
echo -e "${GREEN}✅ GitHub CLI is installed${NC}"

# Switch to correct account
echo "Switching to JonathonJulian account..."
gh auth switch -u JonathonJulian
check_status "Account switch"

# Verify authentication (ignore token errors, check keyring auth)
AUTH_OUTPUT=$(gh auth status 2>&1 || true)
if echo "$AUTH_OUTPUT" | grep -q "Logged in to github.com account JonathonJulian (keyring)"; then
    # Check if JonathonJulian is the active account
    if echo "$AUTH_OUTPUT" | grep -A1 "JonathonJulian" | grep -q "Active account: true"; then
        echo -e "${GREEN}✅ Authenticated as JonathonJulian (active)${NC}"
    else
        # Need to re-switch since something changed the active account
        echo "Re-switching to JonathonJulian account..."
        gh auth switch -u JonathonJulian
        echo -e "${GREEN}✅ Authenticated as JonathonJulian${NC}"
    fi
else
    echo -e "${RED}❌ JonathonJulian account not found in keyring${NC}"
    echo "Run: gh auth login"
    exit 1
fi

# Check required scopes
echo "Verifying OAuth scopes..."
SCOPES=$(gh auth status 2>&1 | grep -A5 "JonathonJulian" | grep "Token scopes:" || true)
if [[ "$SCOPES" =~ "project" ]] && [[ "$SCOPES" =~ "repo" ]]; then
    echo -e "${GREEN}✅ Required scopes are present${NC}"
    echo "  Found scopes: $(echo "$SCOPES" | sed 's/.*Token scopes: //')"
else
    echo -e "${YELLOW}⚠️  Missing required scopes${NC}"
    echo "  Current scopes: $(echo "$SCOPES" | sed 's/.*Token scopes: //')"
    echo "Run: gh auth refresh -h github.com -s project,repo,workflow"
fi

# Test API access
echo "Testing GitHub API access..."
if gh api user --jq .login | grep -q "JonathonJulian"; then
    echo -e "${GREEN}✅ GitHub API access verified${NC}"
else
    echo -e "${RED}❌ GitHub API test failed${NC}"
    exit 1
fi

# 2. Clean Build Environment
echo -e "\n${YELLOW}2️⃣ Clean Build Environment${NC}"
echo "--------------------------------"
cargo clean
cargo build --release
check_status "Release build"

# 3. Code Quality Checks
echo -e "\n${YELLOW}3️⃣ Code Quality Checks${NC}"
echo "--------------------------------"

echo "Running formatter check..."
cargo fmt -- --check
check_status "Code formatting"

echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings
check_status "Clippy lints"

# 4. Unit Tests
echo -e "\n${YELLOW}4️⃣ Unit Tests${NC}"
echo "--------------------------------"
cargo test --lib --all-features -- --nocapture
check_status "Unit tests"

# 5. Integration Tests
echo -e "\n${YELLOW}5️⃣ Integration Tests${NC}"
echo "--------------------------------"
echo "Running all integration tests..."
cargo test --test '*' -- --nocapture
check_status "Integration tests"

# 6. Security Audit
echo -e "\n${YELLOW}6️⃣ Security Audit${NC}"
echo "--------------------------------"
if command -v cargo-audit &> /dev/null; then
    cargo audit
    check_status "Security audit"
else
    echo -e "${YELLOW}⚠️  cargo-audit not installed, skipping${NC}"
    echo "Install with: cargo install cargo-audit"
fi

# 7. Manual Test Commands
echo -e "\n${YELLOW}7️⃣ Manual Test Commands${NC}"
echo "--------------------------------"

echo "Testing CLI version..."
./target/release/taskmaster-sync --version
check_status "Version command"

echo "Testing auth verification..."
./target/release/taskmaster-sync auth
check_status "Auth command"

echo "Testing config display..."
./target/release/taskmaster-sync config
check_status "Config command"

echo "Testing dry-run sync..."
./target/release/taskmaster-sync sync --dry-run
check_status "Dry-run sync"

# 8. Test Coverage Report
echo -e "\n${YELLOW}8️⃣ Test Coverage (Optional)${NC}"
echo "--------------------------------"
if command -v cargo-tarpaulin &> /dev/null; then
    echo "Generating coverage report..."
    cargo tarpaulin --out Html --target-dir target/tarpaulin
    echo -e "${GREEN}✅ Coverage report generated at: target/tarpaulin/tarpaulin-report.html${NC}"
else
    echo -e "${YELLOW}⚠️  cargo-tarpaulin not installed, skipping coverage${NC}"
    echo "Install with: cargo install cargo-tarpaulin"
fi

# 9. Summary
echo -e "\n${GREEN}================================================${NC}"
echo -e "${GREEN}✅ ALL REGRESSION TESTS PASSED!${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""
echo "Next steps:"
echo "1. Review GitHub Project at: https://github.com/users/JonathonJulian/projects/10"
echo "2. Run full sync if all looks good: ./target/release/taskmaster-sync sync"
echo "3. Create PR when ready"
echo ""
echo -e "${YELLOW}Remember to:${NC}"
echo "- Test edge cases manually"
echo "- Verify field mappings in GitHub UI"
echo "- Check that Agent field (not Assignee) is properly populated"
echo "- Ensure state persistence works between runs"