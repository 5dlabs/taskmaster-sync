#!/bin/bash
# Automated regression test for TaskMaster Sync
# Validates that GitHub project state exactly matches TaskMaster data

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🔍 TaskMaster Sync Regression Test${NC}"
echo "=================================="

# Configuration
TAG="master"
PROJECT_NUMBER="18"
TASKMASTER_FILE=".taskmaster/tasks/tasks.json"

# Ensure authentication
./scripts/ensure-github-auth.sh > /dev/null 2>&1

echo -e "\n${YELLOW}1. Extracting TaskMaster expectations...${NC}"

# Extract expected data from TaskMaster
EXPECTED_TASKS=$(cat "$TASKMASTER_FILE" | jq -r ".${TAG}.tasks[] | @base64")
EXPECTED_COUNT=$(cat "$TASKMASTER_FILE" | jq ".${TAG}.tasks | length")

echo "Expected tasks: $EXPECTED_COUNT"

# Create temporary files for comparison
EXPECTED_FILE="/tmp/taskmaster_expected.json"
ACTUAL_FILE="/tmp/github_actual.json"

echo "[]" > "$EXPECTED_FILE"
echo "[]" > "$ACTUAL_FILE"

# Build expected data structure
echo "$EXPECTED_TASKS" | while IFS= read -r task_data; do
    task=$(echo "$task_data" | base64 --decode)
    
    id=$(echo "$task" | jq -r '.id')
    title=$(echo "$task" | jq -r '.title')
    assignee=$(echo "$task" | jq -r '.assignee // empty')
    priority=$(echo "$task" | jq -r '.priority')
    status=$(echo "$task" | jq -r '.status')
    
    # Add to expected file
    jq --arg id "$id" --arg title "$title" --arg assignee "$assignee" --arg priority "$priority" --arg status "$status" \
       '. += [{id: $id, title: $title, assignee: $assignee, priority: $priority, status: $status}]' \
       "$EXPECTED_FILE" > "${EXPECTED_FILE}.tmp" && mv "${EXPECTED_FILE}.tmp" "$EXPECTED_FILE"
done

echo -e "\n${YELLOW}2. Running sync to ensure up-to-date state...${NC}"

# Run sync to ensure current state
cargo run -- sync "$TAG" "$PROJECT_NUMBER" --verbose > /dev/null 2>&1

echo -e "\n${YELLOW}3. Querying GitHub project state...${NC}"

# Query GitHub project using GraphQL
PROJECT_DATA=$(gh api graphql -f query="
{
  organization(login: \"5dlabs\") {
    projectV2(number: $PROJECT_NUMBER) {
      id
      title
      items(first: 100) {
        totalCount
        nodes {
          id
          fieldValues(first: 20) {
            nodes {
              ... on ProjectV2ItemFieldTextValue {
                text
                field {
                  ... on ProjectV2Field {
                    name
                  }
                }
              }
              ... on ProjectV2ItemFieldSingleSelectValue {
                name
                field {
                  ... on ProjectV2SingleSelectField {
                    name
                  }
                }
              }
            }
          }
          content {
            ... on DraftIssue {
              title
              body
            }
            ... on Issue {
              title
              body
              number
            }
          }
        }
      }
    }
  }
}")

ACTUAL_COUNT=$(echo "$PROJECT_DATA" | jq '.data.organization.projectV2.items.totalCount')
echo "Actual items in GitHub: $ACTUAL_COUNT"

# Extract actual data from GitHub
echo "$PROJECT_DATA" | jq -r '.data.organization.projectV2.items.nodes[] | @base64' | while IFS= read -r item_data; do
    item=$(echo "$item_data" | base64 --decode)
    
    title=$(echo "$item" | jq -r '.content.title')
    
    # Extract field values
    tm_id=""
    agent=""
    status=""
    priority=""
    
    echo "$item" | jq -r '.fieldValues.nodes[] | @base64' | while IFS= read -r field_data; do
        field=$(echo "$field_data" | base64 --decode)
        field_name=$(echo "$field" | jq -r '.field.name // empty')
        
        case "$field_name" in
            "TM_ID")
                tm_id=$(echo "$field" | jq -r '.text // empty')
                ;;
            "Agent")
                agent=$(echo "$field" | jq -r '.name // empty')
                ;;
            "Status")
                status=$(echo "$field" | jq -r '.name // empty')
                ;;
            "Priority")
                priority=$(echo "$field" | jq -r '.name // empty')
                ;;
        esac
    done
    
    # Only process items with TM_ID (our synced tasks)
    if [[ -n "$tm_id" ]]; then
        # Map GitHub status back to TaskMaster status
        case "$status" in
            "QA Review") tm_status="done" ;;
            "In Progress") tm_status="in-progress" ;;
            "Todo") tm_status="pending" ;;
            *) tm_status="$status" ;;
        esac
        
        # Add to actual file
        jq --arg id "$tm_id" --arg title "$title" --arg assignee "$agent" --arg priority "$priority" --arg status "$tm_status" \
           '. += [{id: $id, title: $title, assignee: $assignee, priority: $priority, status: $status}]' \
           "$ACTUAL_FILE" > "${ACTUAL_FILE}.tmp" && mv "${ACTUAL_FILE}.tmp" "$ACTUAL_FILE"
    fi
done

echo -e "\n${YELLOW}4. Comparing TaskMaster vs GitHub...${NC}"

# Sort both files for comparison
jq 'sort_by(.id)' "$EXPECTED_FILE" > "${EXPECTED_FILE}.sorted"
jq 'sort_by(.id)' "$ACTUAL_FILE" > "${ACTUAL_FILE}.sorted"

# Compare the files
DIFFERENCES=0

# Check each expected task
cat "${EXPECTED_FILE}.sorted" | jq -r '.[] | @base64' | while IFS= read -r expected_data; do
    expected=$(echo "$expected_data" | base64 --decode)
    id=$(echo "$expected" | jq -r '.id')
    
    # Find corresponding actual task
    actual=$(cat "${ACTUAL_FILE}.sorted" | jq --arg id "$id" '.[] | select(.id == $id)')
    
    if [[ -z "$actual" ]]; then
        echo -e "${RED}❌ Missing task in GitHub: $id${NC}"
        DIFFERENCES=$((DIFFERENCES + 1))
        continue
    fi
    
    # Compare each field
    exp_title=$(echo "$expected" | jq -r '.title')
    act_title=$(echo "$actual" | jq -r '.title')
    if [[ "$exp_title" != "$act_title" ]]; then
        echo -e "${RED}❌ Title mismatch for $id: expected '$exp_title', got '$act_title'${NC}"
        DIFFERENCES=$((DIFFERENCES + 1))
    fi
    
    exp_assignee=$(echo "$expected" | jq -r '.assignee')
    act_assignee=$(echo "$actual" | jq -r '.assignee')
    if [[ "$exp_assignee" != "$act_assignee" ]]; then
        echo -e "${RED}❌ Assignee mismatch for $id: expected '$exp_assignee', got '$act_assignee'${NC}"
        DIFFERENCES=$((DIFFERENCES + 1))
    fi
    
    exp_priority=$(echo "$expected" | jq -r '.priority')
    act_priority=$(echo "$actual" | jq -r '.priority')
    if [[ "$exp_priority" != "$act_priority" ]]; then
        echo -e "${RED}❌ Priority mismatch for $id: expected '$exp_priority', got '$act_priority'${NC}"
        DIFFERENCES=$((DIFFERENCES + 1))
    fi
    
    exp_status=$(echo "$expected" | jq -r '.status')
    act_status=$(echo "$actual" | jq -r '.status')
    if [[ "$exp_status" != "$act_status" ]]; then
        echo -e "${RED}❌ Status mismatch for $id: expected '$exp_status', got '$act_status'${NC}"
        DIFFERENCES=$((DIFFERENCES + 1))
    fi
    
    if [[ "$exp_title" == "$act_title" && "$exp_assignee" == "$act_assignee" && "$exp_priority" == "$act_priority" && "$exp_status" == "$act_status" ]]; then
        echo -e "${GREEN}✅ $id: All fields match${NC}"
    fi
done

# Check for extra tasks in GitHub
cat "${ACTUAL_FILE}.sorted" | jq -r '.[] | @base64' | while IFS= read -r actual_data; do
    actual=$(echo "$actual_data" | base64 --decode)
    id=$(echo "$actual" | jq -r '.id')
    
    expected=$(cat "${EXPECTED_FILE}.sorted" | jq --arg id "$id" '.[] | select(.id == $id)')
    if [[ -z "$expected" ]]; then
        echo -e "${RED}❌ Extra task in GitHub: $id${NC}"
        DIFFERENCES=$((DIFFERENCES + 1))
    fi
done

echo -e "\n${YELLOW}5. Summary${NC}"
echo "Expected tasks: $EXPECTED_COUNT"
echo "Actual tasks: $(cat "${ACTUAL_FILE}.sorted" | jq 'length')"

# Cleanup
rm -f "$EXPECTED_FILE" "$ACTUAL_FILE" "${EXPECTED_FILE}.sorted" "${ACTUAL_FILE}.sorted" "${EXPECTED_FILE}.tmp" "${ACTUAL_FILE}.tmp"

if [[ $DIFFERENCES -eq 0 ]]; then
    echo -e "\n${GREEN}🎉 REGRESSION TEST PASSED!${NC}"
    echo "TaskMaster data perfectly matches GitHub project state."
    exit 0
else
    echo -e "\n${RED}💥 REGRESSION TEST FAILED!${NC}"
    echo "Found $DIFFERENCES mismatches between TaskMaster and GitHub."
    exit 1
fi