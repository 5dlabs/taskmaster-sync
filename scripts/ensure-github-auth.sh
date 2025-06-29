#!/bin/bash
# Ensure proper GitHub CLI authentication for TaskMaster Sync
# This script handles the persistent GITHUB_TOKEN issue

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}Ensuring GitHub CLI authentication...${NC}"

# Aggressively clear all GitHub tokens
unset GITHUB_TOKEN
unset GH_TOKEN
export GITHUB_TOKEN=""
export GH_TOKEN=""

# Remove from shell config temporarily (non-destructive)
if [ -f ~/.zshrc ]; then
    # Check if GITHUB_TOKEN is set in zshrc
    if grep -q "export GITHUB_TOKEN" ~/.zshrc; then
        echo -e "${YELLOW}Note: GITHUB_TOKEN is set in ~/.zshrc${NC}"
        echo "This may interfere with GitHub CLI keyring authentication"
    fi
fi

# Switch to JonathonJulian
gh auth switch -u JonathonJulian 2>/dev/null || true

# Verify authentication works
if gh api user --jq .login 2>/dev/null | grep -q "JonathonJulian"; then
    echo -e "${GREEN}✅ Successfully authenticated as JonathonJulian${NC}"
    echo -e "${GREEN}GitHub API access verified!${NC}"
    exit 0
else
    echo "Authentication check failed. Attempting to fix..."
    
    # Try switching again
    gh auth switch -u JonathonJulian
    
    # Test again
    if gh api user --jq .login 2>/dev/null | grep -q "JonathonJulian"; then
        echo -e "${GREEN}✅ Authentication fixed!${NC}"
        exit 0
    else
        echo "❌ Authentication still failing"
        echo "Please run: gh auth login"
        exit 1
    fi
fi