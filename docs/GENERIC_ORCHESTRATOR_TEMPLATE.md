# Generic Multi-Agent Orchestrator Template

## Overview
This template enables orchestration of multiple AI agents working in parallel on any codebase. Agents work in isolated environments with no shared context.

## Core Principles

1. **Isolation**: Each agent gets a fresh workspace with no knowledge of other agents
2. **Parallelization**: Identify tasks that can run simultaneously  
3. **Dependency Management**: Ensure proper sequencing when tasks depend on each other
4. **Quality Assurance**: Adversarial QA by different agents
5. **Production Standards**: All code must be production-ready

## Initial Setup

### 1. Configure Repository

```yaml
# Add agent workspaces to gitignore
mcp_github_create_or_update_file:
  repository: "owner/repo"
  path: ".gitignore"
  content: |
    # Agent workspaces
    .agent-*/
  message: "Add agent workspaces to gitignore"
  branch: "main"
```

### 2. Create Isolated Workspaces

```bash
#!/bin/bash
# Create isolated workspace for each agent
REMOTE_URL=$(git remote get-url origin)

for i in {1..9}; do
  rsync -av --exclude='target/' \
            --exclude='*.lock' \
            --exclude='node_modules/' \
            --exclude='.git/' \
            --exclude='.agent-*/' \
            --filter=':- .gitignore' \
            ./ .agent-$i/
            
  cd .agent-$i
  git init
  git remote add origin $REMOTE_URL
  git add -A
  git commit -m "Agent $i workspace initialized"
  cd ..
done
```

## Task Decomposition

### How to Break Down Work

1. **Single Responsibility**: One clear objective per agent
2. **File Boundaries**: Tasks modifying same files cannot run in parallel
3. **Clear Success Criteria**: Measurable "done" state
4. **Appropriate Scope**: 4-8 hour chunks

### Example Task Breakdown

```yaml
# Too Large ❌
task: "Implement authentication system"

# Right-Sized ✅
task_1:
  description: "Add password hashing utilities"
  files: ["src/auth/crypto.js"]
  success: "Hash/verify functions with tests"

task_2:
  description: "Create login endpoint"
  files: ["src/api/login.js"]
  success: "POST /login returns JWT token"

task_3:
  description: "Add auth middleware"
  files: ["src/middleware/auth.js"]
  success: "Routes protected by token validation"
```

## Agent Assignment Template

```yaml
agent_task:
  agent_id: 1
  workspace: ".agent-1/"
  branch: "feature/task-name"
  
  context_provided:
    - specification_files: ["SPEC.md", "API.md"]
    - task_description: "Implement [specific feature]"
    - success_criteria:
      - "Feature works as specified"
      - "All edge cases handled"
      - "Tests pass"
    
  forbidden_context:
    - No other agent's code
    - No implementation hints
    - No "see how Agent X did it"
```

## GitHub MCP Commands Reference

### Check Repository State

```yaml
# List directory contents
mcp_github_list_directory:
  repository: "owner/repo"
  path: "src/components"
  branch: "main"

# Check file existence
mcp_github_get_file_contents:
  repository: "owner/repo"
  path: "README.md"

# Search codebase
mcp_github_search_code:
  repository: "owner/repo"
  query: "function authenticate"
```

### Branch Operations

```yaml
# List branches
mcp_github_list_branches:
  repository: "owner/repo"

# Create feature branch
mcp_github_create_branch:
  repository: "owner/repo"
  branch: "feature/new-task"
  from: "main"
```

### Pull Request Workflow

```yaml
# Create PR
mcp_github_create_pull_request:
  repository: "owner/repo"
  title: "✅ Task: Description"
  body: "Implements [feature] according to spec"
  base: "main"
  head: "feature/task"

# Monitor PRs
mcp_github_list_pull_requests:
  repository: "owner/repo"
  state: "open"

# Review PR
mcp_github_create_pull_request_review:
  repository: "owner/repo"
  pr_number: 123
  event: "APPROVE"
  body: "Tests pass, meets requirements"

# Merge PR
mcp_github_merge_pull_request:
  repository: "owner/repo"
  pr_number: 123
  merge_method: "squash"
```

### Tag Completed Work

```yaml
mcp_github_create_tag:
  repository: "owner/repo"
  tag: "task-1-complete"
  target: "abc123"  # commit SHA
  message: "Task 1 implemented and tested"
```

## Dependency Management

### Identifying Dependencies

```yaml
task_analysis:
  task_a:
    modifies: ["auth/login.js", "auth/utils.js"]
    exports: ["loginUser()", "validateToken()"]
  
  task_b:
    modifies: ["api/profile.js"]
    imports: ["validateToken from auth/utils.js"]
    
  conclusion: "Task B depends on Task A"
```

### Execution Waves

```yaml
wave_1:  # Can run in parallel
  - task: "Add utility functions"
    files: ["utils/hash.js", "utils/validate.js"]
  - task: "Create UI components"
    files: ["components/Login.jsx", "components/Profile.jsx"]

wave_2:  # Depends on wave_1
  - task: "Wire up authentication"
    needs: ["utility functions", "UI components"]
```

## Quality Assurance Protocol

### Adversarial QA Assignment

```yaml
qa_assignment:
  never: "Agent reviews own code"
  always: "Different agent does QA"
  
  qa_agent_gets:
    - Original specification
    - PR number to review
    - No implementation context
    
  qa_mission: "Find bugs and spec violations"
```

### QA Process

1. **Read Spec First** - Build own mental model
2. **Review Code** - Compare to expectations
3. **Write Breaking Tests** - Try to find bugs
4. **Report Findings** - Create issues for bugs

## Production Code Standards

### Mandatory Requirements

```yaml
production_standards:
  forbidden_phrases:
    - "TODO: implement later"
    - "In production we would..."
    - "This is just for demo"
    - "Temporary workaround"
    
  required_quality:
    - Error handling for all cases
    - Input validation
    - Meaningful error messages
    - Tests with real assertions
```

### Code Review Checklist

- ✅ Does it actually work?
- ✅ Are all requirements met?
- ✅ Do tests fail when code is broken?
- ✅ Is it maintainable?

## Orchestration Patterns

### Pattern 1: Feature Development

```yaml
orchestration:
  phase_1: "Utility functions by different agents"
  phase_2: "Components using utilities"
  phase_3: "Integration layer"
  phase_4: "QA by fresh agents"
```

### Pattern 2: Bug Fix Sprint

```yaml
orchestration:
  assign: "One bug per agent"
  isolate: "Each in own workspace"
  qa: "Cross-review by other agents"
  merge: "As each fix is verified"
```

### Pattern 3: Refactoring

```yaml
orchestration:
  analyze: "Identify independent modules"
  assign: "One module per agent"
  constraint: "Maintain public interfaces"
  verify: "Integration tests still pass"
```

## Monitoring & Coordination

### Progress Tracking

```yaml
# Check PR status
mcp_github_list_pull_requests:
  repository: "owner/repo"
  state: "open"

# Check CI status
mcp_github_list_workflow_runs:
  repository: "owner/repo"
  workflow_id: "ci.yml"

# Track issues
mcp_github_search_issues:
  repository: "owner/repo"
  query: "is:issue is:open label:bug"
```

### Success Metrics

- All PRs pass CI
- QA agents approve
- No merge conflicts
- Meets deadline

## Common Pitfalls

1. **Sharing Context**: Agents discussing implementation pollutes isolation
2. **Assuming File State**: Always verify current state
3. **Skipping QA**: Every PR needs fresh eyes
4. **Incomplete Tasks**: No TODOs in PRs

## Customization Points

- Adjust number of agents based on work volume
- Modify workspace creation for your tech stack
- Define project-specific quality standards
- Add custom GitHub labels for tracking

Be Useful. Not Thorough.