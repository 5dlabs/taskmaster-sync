# Test Agent Prompt for Taskmaster-Sync GitHub Action

You are tasked with comprehensively testing the newly published `taskmaster-sync` GitHub Action by **creating a complete dummy project** and **simulating the entire multi-agent development lifecycle**. This action synchronizes TaskMaster tasks with GitHub Projects v2 in CI/CD workflows.

## Background Context

**What is taskmaster-sync?**
- A Rust CLI tool that syncs TaskMaster task files to GitHub Projects
- Designed for multi-agent development workflows where AI agents update TaskMaster files
- Triggers on PR creation/updates to sync task status changes to GitHub Projects
- One-way sync only: TaskMaster → GitHub Projects (GitHub Actions can't modify repo files)

**Key Multi-Agent Workflow:**
1. **PM Agent**: Assigns task (status: pending) → should sync to GitHub Projects as "Todo"
2. **Dev Agent**: Works on feature branch, updates status to "in-progress" → should sync as "In Progress"  
3. **Dev Agent**: Completes work, updates status to "done", creates PR → should sync as "QA Review"
4. **QA Agent**: Tests on feature branch, either approves (merge) or requests changes
5. **Integration**: Merge to main triggers final sync

## Mission: Complete Lifecycle Test

**You must:**
1. **Create a realistic dummy project** with multiple microservices
2. **Simulate the complete multi-agent development workflow** 
3. **Verify each stage syncs correctly** to GitHub Projects
4. **Test all edge cases and error scenarios**
5. **Create GitHub issues** for any failures or unexpected behaviors
6. **Follow regression testing methodology** from the docs

## Test Objectives

Create a comprehensive test that validates:

1. **Complete Development Lifecycle**
   - PM task assignment (pending → Todo)
   - Development work (in-progress → In Progress)  
   - Completion and PR creation (done → QA Review)
   - QA testing and approval workflow
   - Merge and final state sync

2. **Multi-Service Project Simulation**
   - Frontend service tasks
   - Backend API tasks  
   - Database migration tasks
   - Infrastructure/DevOps tasks
   - Cross-service dependencies

3. **GitHub Action Integration**
   - Action downloads correct binary for platform
   - Authenticates with GitHub properly
   - Reads TaskMaster configuration correctly
   - Creates/updates GitHub Project items accurately
   - Outputs proper JSON results for CI/CD

4. **Field Mapping Accuracy**
   - Status mapping: pending→Todo, in-progress→In Progress, done→QA Review
   - Agent assignment: TaskMaster assignee → GitHub Project Agent field
   - Priority mapping: high/medium/low → P0/P1/P2/P3
   - Dependencies and subtasks handling

5. **Delta Sync Performance**
   - Only changed items update on subsequent syncs
   - No duplicate items created
   - Proper state tracking between runs
   - Efficient GitHub API usage

6. **Error Handling & Edge Cases**
   - Missing configuration files
   - Invalid project numbers  
   - Authentication failures
   - Malformed TaskMaster files
   - Network connectivity issues
   - GitHub API rate limiting

## Test Repository Setup

**Create a new repository simulating a realistic multi-service project:**
```
test-taskmaster-sync/
├── .github/
│   └── workflows/
│       ├── sync-tasks.yml           # Main sync workflow
│       └── pr-validation.yml        # PR validation workflow
├── .taskmaster/
│   ├── sync-config.json             # Project configuration
│   ├── agent-github-mapping.json    # Agent to GitHub user mapping
│   └── tasks/
│       └── tasks.json               # Main task definitions
├── services/
│   ├── frontend/                    # React frontend service
│   │   ├── src/
│   │   └── package.json
│   ├── backend/                     # Node.js API service  
│   │   ├── src/
│   │   └── package.json
│   └── database/                    # Database migrations
│       └── migrations/
├── infrastructure/
│   ├── docker-compose.yml
│   └── terraform/
└── README.md
```

**This structure simulates:**
- A realistic microservices architecture
- Multiple agents working on different components
- Cross-service dependencies
- Infrastructure and DevOps tasks

## Required Test Files

### 1. `.github/workflows/sync-tasks.yml`
```yaml
name: Sync TaskMaster Tasks

on:
  pull_request:
    branches: [ main ]
  push:
    branches: [ main ]

jobs:
  sync-tasks:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Sync Tasks to GitHub Project
        id: sync
        uses: 5dlabs/taskmaster-sync@v1
        with:
          project-number: YOUR_PROJECT_NUMBER  # Replace with actual project #
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Show Results
        run: |
          echo "Created: ${{ steps.sync.outputs.created }}"
          echo "Updated: ${{ steps.sync.outputs.updated }}"
          echo "Deleted: ${{ steps.sync.outputs.deleted }}"
          echo "Skipped: ${{ steps.sync.outputs.skipped }}"
          echo "Errors: ${{ steps.sync.outputs.errors }}"
```

### 2. `.taskmaster/sync-config.json`
```json
{
  "version": "1.0.0",
  "organization": "YOUR_GITHUB_ORG",
  "project_mappings": {
    "main": {
      "project_number": YOUR_PROJECT_NUMBER,
      "project_id": "YOUR_PROJECT_ID",
      "repository": "YOUR_ORG/test-taskmaster-sync",
      "subtask_mode": "nested"
    }
  }
}
```

### 3. `.taskmaster/agent-github-mapping.json`
```json
{
  "agent_mappings": {
    "pm0-testorg": "ProjectManager",
    "swe-1-testorg": "Developer",
    "qa0-testorg": "QAEngineer"
  }
}
```

### 4. `.taskmaster/tasks/tasks.json` - Multi-Service Project Tasks
```json
{
  "version": "1.0.0",
  "main": {
    "tasks": [
      {
        "id": "setup-project-structure",
        "title": "Set up multi-service project structure",
        "status": "pending",
        "priority": "high",
        "assignee": "pm0-testorg",
        "dependencies": [],
        "subtasks": [
          "Create service directories",
          "Set up Docker configuration", 
          "Initialize Git repository"
        ]
      },
      {
        "id": "frontend-user-auth",
        "title": "Implement user authentication in frontend",
        "status": "pending", 
        "priority": "high",
        "assignee": "swe-frontend-testorg",
        "dependencies": ["setup-project-structure"],
        "subtasks": [
          "Create login component",
          "Add JWT token handling",
          "Implement protected routes"
        ]
      },
      {
        "id": "backend-auth-api", 
        "title": "Build authentication API endpoints",
        "status": "pending",
        "priority": "high", 
        "assignee": "swe-backend-testorg",
        "dependencies": ["setup-project-structure"],
        "subtasks": [
          "Create user registration endpoint",
          "Implement login endpoint", 
          "Add JWT middleware"
        ]
      },
      {
        "id": "database-user-schema",
        "title": "Design user database schema",
        "status": "pending",
        "priority": "medium",
        "assignee": "swe-database-testorg", 
        "dependencies": ["backend-auth-api"],
        "subtasks": [
          "Create user table migration",
          "Add user roles table",
          "Set up foreign key constraints"
        ]
      },
      {
        "id": "infrastructure-deployment",
        "title": "Set up CI/CD and deployment infrastructure", 
        "status": "pending",
        "priority": "medium",
        "assignee": "devops-testorg",
        "dependencies": ["frontend-user-auth", "backend-auth-api"],
        "subtasks": [
          "Configure GitHub Actions",
          "Set up Docker registry",
          "Create Kubernetes manifests"
        ]
      },
      {
        "id": "integration-testing",
        "title": "End-to-end integration testing",
        "status": "pending",
        "priority": "medium", 
        "assignee": "qa0-testorg",
        "dependencies": ["database-user-schema", "infrastructure-deployment"],
        "subtasks": [
          "Test user registration flow",
          "Verify authentication across services",
          "Load test API endpoints"
        ]
      }
    ]
  }
}
```

## GitHub Project Setup

**Before testing, create a GitHub Project:**

1. Go to your GitHub organization/repository
2. Create a new Project (Projects v2)
3. Add these custom fields:
   - **TM_ID** (Text) - for TaskMaster task IDs
   - **Agent** (Single Select) - for assignee mapping
   - **Priority** (Single Select) - High, Medium, Low
4. Add **QA Review** option to the Status field
5. Note the project number (visible in URL)

## Complete Development Lifecycle Simulation

**You must simulate a realistic 2-week development sprint** with multiple agents working on the multi-service project. Follow this exact sequence:

### Phase 1: Project Planning (PM Agent)
1. **Initial Setup**: All tasks start as "pending" status
2. **Push to main**: Trigger initial sync 
3. **Verify**: All 6 tasks appear in GitHub Project as "Todo" status
4. **Expected Results**: Created: 6, Updated: 0, Status: All "Todo"

### Phase 2: Development Kickoff (Multiple Agents)
1. **Frontend Dev starts**: Update `frontend-user-auth` status to "in-progress"
2. **Backend Dev starts**: Update `backend-auth-api` status to "in-progress"  
3. **Create feature branches**: `feature/frontend-auth`, `feature/backend-auth`
4. **Push changes**: Trigger sync on both branches
5. **Expected Results**: 2 tasks should show "In Progress" status

### Phase 3: Development Completion (Dev Agents)
1. **Frontend completion**: Update `frontend-user-auth` to "done", create PR
2. **Backend completion**: Update `backend-auth-api` to "done", create PR
3. **Verify PR sync**: Both tasks should show "QA Review" status
4. **Expected Results**: 2 tasks moved from "In Progress" → "QA Review"

### Phase 4: QA Testing (QA Agent)  
1. **QA starts testing**: Update `integration-testing` to "in-progress"
2. **Database work begins**: Update `database-user-schema` to "in-progress"
3. **Push updates**: Trigger sync
4. **Expected Results**: More tasks in "In Progress", dependencies respected

### Phase 5: Issue Discovery & Resolution
1. **QA finds bug**: Revert `frontend-user-auth` from "done" back to "in-progress"
2. **Bug fix**: Re-complete task, set back to "done"
3. **Verify state tracking**: Should update existing item, not create duplicate
4. **Expected Results**: Updated: 1, Created: 0 (delta sync working)

### Phase 6: Infrastructure & Deployment
1. **DevOps begins**: Update `infrastructure-deployment` to "in-progress"
2. **Complete infrastructure**: Update to "done"
3. **Final QA**: Complete `integration-testing`, set to "done"
4. **Expected Results**: All tasks eventually reach "QA Review" or "Done"

## Regression Testing Scenarios

### Test 1: Basic GitHub Action Functionality
```bash
# Verify action downloads and runs
# Check all required outputs are set
# Validate JSON output format
```

### Test 2: Field Mapping Accuracy
**Critical mappings to verify:**
- `pending` → "Todo" (exactly, case-sensitive)
- `in-progress` → "In Progress" (exactly, case-sensitive)  
- `done` → "QA Review" (exactly, case-sensitive)
- TaskMaster `assignee` → GitHub `Agent` field (not "Assignee")
- Priority mapping: high→P0, medium→P1, low→P2

### Test 3: Delta Sync Performance
1. **Initial sync**: Should create all tasks
2. **No-change sync**: Should show 0 created, 0 updated
3. **Single change**: Should update only 1 task
4. **Bulk changes**: Should update only changed tasks

### Test 4: Dependency Handling
- Verify subtasks sync correctly
- Check dependency links (if supported)
- Validate task ordering in project

### Test 5: Error Handling & Edge Cases
1. **Authentication failure**: Remove/invalid GITHUB_TOKEN
2. **Project not found**: Invalid project number
3. **Malformed JSON**: Break tasks.json syntax
4. **Missing fields**: Remove required TaskMaster fields
5. **Network issues**: Simulate connection failures
6. **Rate limiting**: Test with rapid sync requests

## Expected Results

### Success Criteria
- [ ] Action downloads and runs without errors
- [ ] Tasks appear in GitHub Project with correct fields
- [ ] Status mapping works: "done" → "QA Review"
- [ ] Agent mapping works: "swe-1-testorg" → "Developer"
- [ ] Delta sync prevents duplicates
- [ ] JSON outputs are properly set
- [ ] Branch name auto-detection works

### Output Validation
- Action should output something like:
```
Created: 2
Updated: 0  
Deleted: 0
Skipped: 0
Errors: 0
```

## Troubleshooting Checklist

If tests fail, check:
1. **Authentication**: Ensure GITHUB_TOKEN has `project` scope
2. **Project Fields**: Verify TM_ID, Agent, Priority fields exist
3. **Project Number**: Correct project number in config
4. **File Paths**: All TaskMaster files in correct locations
5. **JSON Syntax**: Valid JSON in all config files

## Issue Creation & Bug Reporting

**For any failures or unexpected behaviors, you MUST create detailed GitHub issues in the `5dlabs/taskmaster-sync` repository.**

### Issue Template for Failures:
```markdown
## Bug Report: [Brief Description]

### Environment
- **Action Version**: v1.0.1 (or specific version tested)
- **Runner OS**: ubuntu-latest/windows-latest/macos-latest
- **Test Repository**: [link to your test repo]
- **GitHub Project**: [project URL]

### Expected Behavior
[Describe what should have happened based on the lifecycle test]

### Actual Behavior  
[Describe what actually happened]

### Steps to Reproduce
1. [Step 1]
2. [Step 2] 
3. [Step 3]

### Action Logs
```
[Paste relevant GitHub Action logs]
```

### TaskMaster Data
```json
[Paste the tasks.json content that caused the issue]
```

### GitHub Project State
[Screenshot or description of actual project state]

### Additional Context
- Phase of testing: [Phase 1-6 from lifecycle simulation]
- Test scenario: [which specific test case]
- Related issues: [any dependencies or related bugs]

### Severity
- [ ] Critical: Action completely fails
- [ ] High: Major functionality broken  
- [ ] Medium: Minor feature issue
- [ ] Low: Enhancement/improvement

### Labels
Add appropriate labels: `bug`, `action`, `sync-engine`, `field-mapping`, `authentication`, etc.
```

### Regression Testing Validation

**Before creating issues, validate against known working state:**

1. **Compare with regression test expectations** from docs
2. **Check against field mapping documentation**  
3. **Verify authentication setup** (unset GITHUB_TOKEN, use keyring)
4. **Confirm project field configuration** (Agent vs Assignee, Status options)

### Issue Types to Watch For:

1. **Field Mapping Issues**
   - Status not mapping correctly
   - Agent field not populating
   - Priority values incorrect

2. **Delta Sync Problems**  
   - Duplicate items created
   - Items not updating
   - State file corruption

3. **Authentication Failures**
   - Token vs keyring conflicts
   - Permission scope issues
   - Organization access problems

4. **GitHub Action Issues**
   - Binary download failures
   - Platform detection problems
   - JSON output format errors

## Comprehensive Reporting

Document your findings with:

### 1. Executive Summary
- **Overall Status**: PASS/FAIL with reason
- **Total Test Cases**: X passed, Y failed out of Z
- **Critical Issues**: Number of blocking bugs found
- **Recommendations**: High-level next steps

### 2. Lifecycle Test Results
For each phase (1-6), document:
- **Phase Name**: e.g., "Phase 2: Development Kickoff"
- **Expected Results**: What should have happened
- **Actual Results**: What actually happened  
- **Status**: ✅ PASS / ❌ FAIL
- **Issues Created**: Links to any GitHub issues
- **Screenshots**: Before/after GitHub Project state

### 3. Regression Test Matrix
| Test Scenario | Expected | Actual | Status | Issues |
|---------------|----------|--------|---------|---------|
| Field Mapping | pending→Todo | pending→Todo | ✅ PASS | - |
| Status Workflow | done→QA Review | done→QA Review | ✅ PASS | - |
| Delta Sync | Update 1 item | Created duplicate | ❌ FAIL | #123 |
| Authentication | Success | 401 error | ❌ FAIL | #124 |

### 4. Performance Metrics
- **Sync Duration**: Time taken for each phase
- **API Calls**: Estimated GitHub API usage
- **Success Rate**: Percentage of successful syncs
- **Error Rate**: Frequency of failures

### 5. GitHub Action Validation
- **Binary Download**: ✅/❌ Correct platform binary downloaded  
- **Authentication**: ✅/❌ GitHub token authentication worked
- **JSON Output**: ✅/❌ Proper format for CI/CD parsing
- **Output Variables**: ✅/❌ All outputs (created, updated, etc.) set correctly

## Important Notes

- The action expects **project-number** as input (not project ID)
- Always use the latest version: `@v1` (which points to v1.0.1+)
- TaskMaster files must be in `.taskmaster/` directory
- GitHub Projects v2 is required (not classic projects)
- Agent mapping is case-sensitive

Create issues in the `5dlabs/taskmaster-sync` repository for any bugs found during testing.

## Final Deliverables

Upon completion of testing, provide:

### 1. Test Repository
- **Public GitHub repository** with complete multi-service project setup
- **README.md** documenting the test setup and findings
- **Working GitHub Actions** demonstrating the sync functionality
- **Complete TaskMaster configuration** with realistic multi-agent tasks

### 2. Test Report Document
- **Comprehensive markdown report** following the reporting template above
- **Executive summary** with overall PASS/FAIL assessment
- **Detailed phase-by-phase results** from the 6-phase lifecycle test
- **Screenshots and evidence** of GitHub Project states
- **Performance metrics** and timing data

### 3. GitHub Issues (if any failures)
- **Well-documented bug reports** using the provided template
- **Proper severity and labeling** for each issue
- **Clear reproduction steps** for developers to fix issues
- **Links between related issues** if multiple bugs are connected

### 4. Recommendations Document
- **Assessment of production readiness**
- **Suggested improvements** for user experience
- **Edge cases** that need additional handling
- **Documentation gaps** that should be addressed

### 5. Video Walkthrough (Optional but Recommended)
- **Screen recording** showing the complete lifecycle test
- **Narration** explaining what's happening at each phase
- **Demonstration** of both successful and failed scenarios
- **Upload to unlisted YouTube** or similar platform

## Success Metrics

The taskmaster-sync GitHub Action will be considered **PRODUCTION READY** if:

- ✅ **95%+ test cases pass** without critical failures
- ✅ **Complete development lifecycle** works end-to-end
- ✅ **Field mappings are accurate** (status, assignee, priority)
- ✅ **Delta sync performs efficiently** (no duplicates, only updates changed)
- ✅ **Authentication works reliably** across different environments
- ✅ **Error handling is graceful** with helpful error messages
- ✅ **GitHub Action integration is seamless** with proper outputs

## Post-Testing Next Steps

Based on your findings:

1. **If tests pass**: Document successful validation and recommend proceeding with broader rollout
2. **If critical issues found**: Work with development team to prioritize and fix blocking bugs
3. **If minor issues found**: Create enhancement backlog for future iterations
4. **Documentation needs**: Update user guides and troubleshooting docs based on learnings

**Your thorough testing will directly impact the success of multi-agent development workflows using taskmaster-sync!** 🚀