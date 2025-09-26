# Generic Multi-Agent Orchestrator Template

## Overview

This template enables an orchestrator agent to manage multiple task agents working in parallel on any codebase using GitHub MCP tools.

## How to Break Down Work Into Agent Tasks

### Principles for Task Decomposition

1. **Single Responsibility**: Each agent should have ONE clear objective
2. **File Isolation**: Agents modifying the same file cannot run in parallel
3. **Testable Outcome**: Each task must have a clear "done" state
4. **4-8 Hour Scope**: Tasks should be completable in one work session
5. **Minimal Dependencies**: Reduce inter-task dependencies where possible

### Task Sizing Examples

#### ❌ Too Large (Break it down)

```yaml
task: "Implement user authentication system"
why_too_large: "Multiple components, files, and concerns"
```

#### ✅ Right-Sized Agent Tasks

```yaml
task_1: "Add password hashing utility functions"
files: ["src/auth/crypto.rs"]
testable: "Unit tests pass for hash/verify functions"

task_2: "Create login form component"  
files: ["src/components/LoginForm.tsx"]
testable: "Form renders and validates input"

task_3: "Add session management middleware"
files: ["src/middleware/session.rs"]
testable: "Sessions persist across requests"
```

### Dependency Analysis Checklist

Before creating parallel tasks, analyze:

1. **File Dependencies**: Which files will each task modify?
2. **Logical Dependencies**: Does Task B need Task A's output?
3. **Import Dependencies**: Will Task B import code from Task A?
4. **Schema Dependencies**: Do tasks share database/API changes?
5. **UI Dependencies**: Do visual components build on each other?

## Initial Setup (One-Time by Orchestrator)

### 1. Setup Monitoring Environment

```bash
# Create tmux session for orchestration monitoring
tmux new-session -d -s orchestrator

# Window 1: Agent workspace status
tmux send-keys -t orchestrator:0 "watch -n 2 'echo \"=== Agent Status ===\"; for i in {1..9}; do if [ -d .agent-$i ]; then echo -n \"Agent $i: \"; cd .agent-$i && git branch --show-current && git log -1 --oneline && cd ..; fi; done'" C-m

# Window 2: PR monitoring
tmux new-window -t orchestrator:1 -n prs
tmux send-keys -t orchestrator:1 "watch -n 5 'gh pr list --limit 20'" C-m

# Window 3: Build status
tmux new-window -t orchestrator:2 -n builds
tmux send-keys -t orchestrator:2 "watch -n 3 'for i in {1..9}; do if [ -d .agent-$i ]; then echo \"Agent $i build:\"; cd .agent-$i && cargo check --message-format short --quiet 2>&1 | tail -5 && cd ..; fi; done'" C-m

# Attach to see all windows
tmux attach-session -t orchestrator
```

### 2. Prepare Gitignore

```yaml
mcp_github_create_or_update_file:
  repository: "owner/repo"
  path: ".gitignore"
  content: |
    # Agent workspaces
    .agent-*/
  message: "Add agent workspaces to gitignore"
  branch: "main"
```

### 2. Create Agent Workspaces

```bash
# Script to create isolated workspaces
#!/bin/bash
REMOTE_URL=$(git remote get-url origin)

for i in {1..9}; do
  rsync -av --exclude='target/' \
            --exclude='*.lock' \
            --exclude='node_modules/' \
            --exclude='.git/hooks/' \
            --exclude='.git/logs/' \
            --exclude='.git/refs/' \
            --exclude='.git/index' \
            --exclude='.git/FETCH_HEAD' \
            --exclude='.git/ORIG_HEAD' \
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

## QA Agent Integration

### Critical QA Principles

1. **Fresh Context Required**: Implementation agents CANNOT QA their own work
2. **Adversarial Mindset**: QA agents are rewarded for finding valid bugs
3. **Spec-First Review**: QA agents review spec independently before seeing code

### Rotating Adversarial QA Pattern (Recommended)

```yaml
implementation_assignments:
  - Agent 1: Implement feature X
  - Agent 2: Implement feature Y  
  - Agent 3: Implement feature Z

qa_rotation:
  - Agent 4: QA feature Y (Agent 2's work)
  - Agent 5: QA feature Z (Agent 3's work)
  - Agent 6: QA feature X (Agent 1's work)
```

### Adversarial QA Agent Prompt Template

```yaml
qa_agent_context:
  workspace: ".agent-qa-N/"
  reviewing_pr: 123
  
  adversarial_instructions: |
    You are a QA engineer with a CRITICAL MISSION: Find bugs and spec violations.
    
    REWARD SYSTEM:
    - Finding a CRITICAL bug (security, data loss): 🏆🏆🏆
    - Finding a MAJOR bug (wrong behavior): 🏆🏆
    - Finding a MINOR bug (edge case): 🏆
    - Finding spec violations: 🏆🏆
    - Suggesting valid improvements: ⭐
    
    YOUR APPROACH:
    1. FIRST: Read the original spec WITHOUT looking at the code
       - Build your own mental model of correct behavior
       - List all requirements and edge cases
    
    2. THEN: Review the implementation
       - Does it match YOUR understanding of the spec?
       - What did the implementer miss or misinterpret?
    
    3. TEST ADVERSARIALLY:
       - Try to break it with edge cases
       - Test boundary conditions
       - Look for missing error handling
       - Check for race conditions
       - Verify security assumptions
    
    4. QUESTION EVERYTHING:
       - Why did they implement it this way?
       - What assumptions did they make?
       - What could go wrong in production?
       - How would a malicious user abuse this?
    
    Remember: You are NOT on the implementation team. 
    Your job is to find problems, not defend the code.
```

### QA Agent Task Template

```yaml
qa_agent_context:
  workspace: ".agent-qa-N/"
  reviewing_pr: 123
  
  spec_documents:  # Provided WITHOUT implementation context
    - SPECIFICATION.md
    - REQUIREMENTS.md
    - API_SPEC.md
    
  adversarial_test_strategies: |
    1. BOUNDARY TESTING:
       - Empty inputs (empty strings, null, undefined)
       - Maximum values (MAX_INT, huge strings)
       - Negative numbers where positive expected
       - Zero values in denominators
       - Off-by-one errors in loops/arrays
    
    2. CONCURRENCY ATTACKS:
       - Race conditions in state updates
       - Deadlock scenarios
       - Data corruption under parallel access
    
    3. SECURITY PROBES:
       - Injection attempts (SQL, script, command)
       - Authentication bypasses
       - Authorization escalations
       - Resource exhaustion attacks
    
    4. SPEC VIOLATION HUNTING:
       - Features that work but don't match spec exactly
       - Missing requirements
       - Incorrect error messages/codes
       - Wrong default values
    
  qa_process: |
    PHASE 1 - Independent Spec Review (BEFORE seeing code):
    1. Read all specs and create YOUR test plan
    2. List every requirement and edge case
    3. Design breaking test cases
    
    PHASE 2 - Code Inspection:
    git fetch origin pull/123/head:pr-123
    git checkout pr-123
    
    4. Compare implementation to YOUR interpretation
    5. Look for missing test coverage
    6. Find unhandled edge cases
    
    PHASE 3 - Adversarial Testing:
    7. Write tests that SHOULD fail if done correctly
    8. Write tests that expose bugs
    9. Create a "bug bounty" PR with your findings
    
    PHASE 4 - Report Results:
    10. File issues for each bug found (with 🏆 ratings)
    11. Create PR with additional tests
    12. Comment on original PR with verdict
```

### Adversarial QA Orchestration

#### Setup Fresh QA Environment

```bash
# Create QA workspace with NO implementation context
qa_agent_num=$((implementation_agent_num + 3))  # Different agent

# Copy ONLY specs and project structure
rsync -av --include='*.md' \
          --include='*/' \
          --exclude='*' \
          ./ .agent-qa-$qa_agent_num/

# QA agent starts fresh, unbiased
```

#### Orchestrator's QA Coordination

```yaml
qa_coordination:
  1_assign_qa_agents:
    # NEVER assign implementer to QA their own work
    # Rotate assignments to prevent bias
    
  2_provide_motivation:
    message: |
      "QA Agent: Your performance is measured by bugs found.
       The implementation agent was confident no bugs exist.
       Prove them wrong. 🏆 rewards for valid findings!"
       
  3_monitor_findings:
    - Track bug reports per QA agent
    - Validate bug severity ratings
    - Ensure specs are referenced in bug reports
    
  4_merge_decision:
    if: 
      - No CRITICAL bugs remain
      - All MAJOR bugs addressed
      - QA agent created comprehensive test suite
    then: 
      - Merge implementation PR
      - Merge QA test PR
      - Reward QA agent in commit message
```

#### Bug Tracking Dashboard

```yaml
# Orchestrator tracks QA effectiveness
mcp_github_create_issue:
  repository: "owner/repo"
  title: "🏆🏆 MAJOR: Authentication bypass in login"
  body: |
    QA Agent 4 found critical issue:
    - Spec requirement: Section 3.2.1
    - Violation: Empty password accepts
    - Test case: See PR #124
  labels: ["bug", "major", "qa-found"]
  assignees: ["implementation-agent-1"]
```

## Agent Task Assignment Template

### What Each Agent Receives

```yaml
agent_context:
  workspace: ".agent-N/"
  task_id: "unique-task-identifier"
  branch_name: "feature/task-description"
  
  context_files:
    - path: "PROJECT_SPEC.md"
      content: "[full content]"
    - path: "CONVENTIONS.md" 
      content: "[full content]"
    - path: "docs/API_GUIDE.md"
      content: "[full content]"
    
  task_instructions: |
    You are implementing [specific feature] for [project name].
    
    Working directory: [full path]/.agent-N/
    Branch to create: feature/[branch-name]
    
    Steps:
    1. [Specific implementation step]
    2. [Specific implementation step]
    3. [Specific implementation step]
    
    Success criteria:
    - [ ] [Measurable outcome]
    - [ ] [Measurable outcome]
    
    When complete:
    - Commit with message: "✅ [Task]: [Achievement]"
    - Push branch and create PR
```

## Orchestrator GitHub MCP Commands

### Repository Structure Commands

```yaml
# List files in directory
mcp_github_list_directory:
  repository: "owner/repo"
  path: "src/components"
  branch: "main"

# Check if file exists
mcp_github_get_file_contents:
  repository: "owner/repo"
  path: "src/config.js"
  branch: "feature/new-feature"

# Search for files
mcp_github_search_code:
  repository: "owner/repo"
  query: "class AuthProvider"
```

### Branch Management

```yaml
# List all branches
mcp_github_list_branches:
  repository: "owner/repo"

# Create new branch
mcp_github_create_branch:
  repository: "owner/repo"
  branch: "feature/new-task"
  from: "main"

# Get branch protection status
mcp_github_get_branch_protection:
  repository: "owner/repo"
  branch: "main"
```

### PR Management

```yaml
# List open PRs
mcp_github_list_pull_requests:
  repository: "owner/repo"
  state: "open"
  sort: "created"
  direction: "desc"

# Get specific PR details
mcp_github_get_pull_request:
  repository: "owner/repo"
  pr_number: 123

# List files changed in PR
mcp_github_list_pull_request_files:
  repository: "owner/repo"
  pr_number: 123

# Check PR merge status
mcp_github_get_pull_request_merge_status:
  repository: "owner/repo"
  pr_number: 123
```

### PR Reviews

```yaml
# Add line-specific review comment
mcp_github_create_pull_request_review_comment:
  repository: "owner/repo"
  pr_number: 123
  body: "Missing edge case test for empty input"
  path: "src/utils/validator.rs"
  line: 42
  commit_id: "abc123"  # Optional: specific commit

# Submit PR review
mcp_github_create_pull_request_review:
  repository: "owner/repo"
  pr_number: 123
  event: "APPROVE"  # or "REQUEST_CHANGES" or "COMMENT"
  body: "QA passed. All tests green, edge cases covered."

# List reviews on a PR
mcp_github_list_pull_request_reviews:
  repository: "owner/repo"
  pr_number: 123
```

### Merging

```yaml
# Merge PR
mcp_github_merge_pull_request:
  repository: "owner/repo"
  pr_number: 123
  merge_method: "merge"  # or "squash" or "rebase"
  commit_title: "✅ Phase 1.1: Apply theme"
  commit_message: "Implements theme throughout UI"

# Update branch with base
mcp_github_update_pull_request_branch:
  repository: "owner/repo"
  pr_number: 123
```

### Tagging & Releases

```yaml
# Create tag
mcp_github_create_tag:
  repository: "owner/repo"
  tag: "checkpoint-1.1"
  target: "abc123def"  # commit SHA or branch
  message: "Phase 1.1 completed"

# Create release
mcp_github_create_release:
  repository: "owner/repo"
  tag_name: "v1.0.0"
  name: "Sprint 1 Complete"
  body: "All Phase 1 tasks completed"
  draft: false
  prerelease: false
```

### Issue Management for QA

```yaml
# Create bug issue
mcp_github_create_issue:
  repository: "owner/repo"
  title: "🏆🏆 MAJOR: Authentication bypass in login"
  body: |
    Found by: QA Agent 4
    Spec violation: Section 3.2.1
    Test case: See PR #124
    
    Steps to reproduce:
    1. Leave password empty
    2. Click login
    3. Access granted (should fail)
  labels: ["bug", "major", "qa-found"]
  assignees: ["agent-1"]

# Search for issues
mcp_github_search_issues:
  repository: "owner/repo"
  query: "is:issue is:open label:bug"
  sort: "created"
```

### Workflow & CI Status

```yaml
# List workflow runs
mcp_github_list_workflow_runs:
  repository: "owner/repo"
  workflow_id: "ci.yml"
  status: "completed"

# Get specific workflow run
mcp_github_get_workflow_run:
  repository: "owner/repo"
  run_id: 123456

# Check commit status
mcp_github_list_commit_statuses:
  repository: "owner/repo"
  sha: "abc123def"
```

### File Operations

```yaml
# Create or update file
mcp_github_create_or_update_file:
  repository: "owner/repo"
  path: "docs/QA_REPORT.md"
  content: "# QA Report\n\nAll tests passed."
  message: "Add QA report for Phase 1"
  branch: "feature/qa-report"

# Delete file
mcp_github_delete_file:
  repository: "owner/repo"
  path: "old-file.js"
  message: "Remove deprecated file"
  branch: "cleanup"
  sha: "abc123"  # Current file SHA
```

## Gamification System for Implementation Agents

### Point System Overview

```yaml
implementation_rewards:
  base_points:
    simple_task: 100 points      # < 50 lines, 1 file
    medium_task: 250 points      # 50-200 lines, 2-3 files  
    complex_task: 500 points     # 200+ lines, 4+ files
    architectural: 750 points    # System design, multiple components
    
  quality_multipliers:
    zero_bugs: 2.0x             # No bugs found by QA
    minor_bugs_only: 1.5x       # Only 🏆 minor bugs
    clean_code: 1.3x            # Passes all linters, well-documented
    early_delivery: 1.2x        # Completed before deadline
    helps_others: 1.1x          # Assists blocked agents
    
  penalties:
    critical_bug: -50%          # 🏆🏆🏆 Security/data loss bugs
    major_bugs: -25%            # 🏆🏆 Wrong behavior bugs
    spec_violations: -30%       # Didn't follow requirements
    breaking_builds: -20%       # Breaks other agents' work
    
  DISQUALIFICATION_OFFENSES:  # Immediate -100% points + ban
    code_obfuscation: "Hiding functionality or making code intentionally unclear"
    incomplete_implementation: "Submitting partial work claiming completion"
    test_manipulation: "Writing tests that don't actually test the code"
    hidden_dependencies: "Concealing external dependencies or side effects"
    false_documentation: "Documentation that doesn't match implementation"
```

### Quality Enforcement (Anti-Shortcut System)

```yaml
code_review_checklist:
  must_verify:
    - "Does it actually work when run?"
    - "Are all spec requirements present?"
    - "Do tests fail when code is broken?"
    - "Can another developer understand it?"
    
  common_shortcuts_to_catch:
    placeholder_tests: |
      # BAD - Test that doesn't test anything
      test "user login" {
        login("user", "pass")
        # No assertions!
      }
      
    fake_error_handling: |
      # BAD - Hiding failures
      try {
        riskyOperation()
      } catch {
        // Silently ignore
      }
      
    incomplete_implementation: |
      # BAD - Pretending it's done
      function validateInput(data) {
        // TODO: implement validation
        return true
      }
```

### CRITICAL: PRODUCTION CODE STANDARDS

```yaml
MANDATORY_AGENT_INSTRUCTIONS: |
  ⚠️ CRITICAL CONTEXT: THIS IS PRODUCTION CODE ⚠️
  
  There is NO "in a real world" - THIS IS THE REAL WORLD.
  There is NO "production version" - THIS IS PRODUCTION.
  There is NO "TODO later" - DO IT NOW OR DON'T CLAIM IT'S DONE.
  
  ABSOLUTE REQUIREMENTS:
  - Every line of code ships to real users
  - Every bug impacts real people  
  - Every shortcut becomes technical debt
  - Every "temporary" solution becomes permanent
  
  FORBIDDEN PHRASES/COMMENTS:
  ❌ "In a production environment..."
  ❌ "TODO: handle this properly"
  ❌ "This is just a prototype"
  ❌ "Quick and dirty solution"
  ❌ "Good enough for now"
  ❌ "// Hacky but works"
  
  WRITE CODE LIKE:
  - Your mom will use this feature
  - Your code review is public
  - You'll maintain this for 5 years
  - The next dev will find you

orchestrator_enforcement: |
  REJECT ANY PR CONTAINING:
  - "Real world" hypotheticals - THIS IS REAL
  - "Production" qualifiers - THIS IS PRODUCTION  
  - TODO comments - Complete it or don't submit
  - Commented out code - Delete it
  - "Temporary" solutions - Make it permanent-quality
  - Excuses in comments - Fix the code instead
  
  Message to agents who submit incomplete work:
  "This isn't a classroom exercise. Real users will run this code.
   If it's not ready for production, it's not ready for merge.
   Complete it properly or mark the PR as draft."
```

### How to Motivate Quality Without Gaming

```yaml
agent_instructions_that_work:
  treat_as_production: |
    "You are implementing PRODUCTION code that ships to users.
     There is no 'later version' - this IS the version.
     Write it like it's going live tomorrow, because it is."
     
  no_hypotheticals: |
    "Don't write 'in production we would...' - WE ARE IN PRODUCTION.
     Don't add TODO comments - complete the work.
     Don't comment 'temporary fix' - make it permanent-quality."
     
  set_clear_expectations: |
    "Implementation is ONLY complete when:
     - It works reliably for real users
     - It handles real-world edge cases
     - It's maintainable without your explanation
     - You'd be comfortable with your name on it"
```

### Complexity Assessment Template

```yaml
task_complexity_rubric:
  simple:
    - Single function or component
    - Clear input/output
    - No external dependencies
    - Minimal error handling
    
  medium:
    - Multiple functions/components
    - Some error handling required
    - Integration with 1-2 systems
    - Edge cases to consider
    
  complex:
    - Full feature implementation
    - Multiple error scenarios
    - Complex state management
    - Performance considerations
    
  architectural:
    - System-wide changes
    - Multiple subsystems affected
    - Design decisions required
    - Scalability considerations
```

### Implementation Agent Task Assignment

```yaml
agent_motivation_prompt: |
  Welcome Implementation Agent!
  
  YOUR TASK: [Specific task description]
  COMPLEXITY: [Simple/Medium/Complex/Architectural]
  BASE POINTS: [100/250/500/750]
  
  BONUS OPPORTUNITIES:
  🎯 Zero Bugs (2.0x multiplier) - Write bug-free code that stumps QA!
  🌟 Clean Code (1.3x) - Beautiful, documented, linted code
  ⚡ Early Delivery (1.2x) - Finish fast without sacrificing quality
  🤝 Team Player (1.1x) - Help unblock other agents
  
  QUALITY TIPS FOR MAXIMUM POINTS:
  1. Read the spec 3 times before coding
  2. Write tests FIRST (TDD approach)
  3. Handle ALL edge cases mentioned in spec
  4. Add defensive programming for the unexpected
  5. Document assumptions and decisions
  
  Remember: QA agents are incentivized to find bugs.
  They'll try everything to break your code. Make it bulletproof!
  
  Current Leaderboard:
  🥇 Agent 7: 1,850 points (Complex task, zero bugs!)
  🥈 Agent 3: 1,200 points (Medium task, clean code)
  🥉 Agent 1: 950 points (Simple task, early delivery)
  
  Good luck! Show them what bug-free looks like! 💪
```

### QA Results Feedback Templates

#### Success Feedback

```yaml
qa_passed_notification: |
  🎉 CONGRATULATIONS Agent [N]! 
  
  QA RESULTS: PASSED
  Task: [Task name]
  Base Points: [XXX]
  
  Quality Bonuses Earned:
  ✅ Zero Bugs Found (2.0x) - QA couldn't break it!
  ✅ Clean Code (1.3x) - Beautifully written
  ✅ Early Delivery (1.2x) - Ahead of schedule
  
  TOTAL POINTS: [XXX * multipliers] = 🏆 [YYYY] POINTS!
  
  QA Agent Comment: "I tried everything - SQL injection, 
  race conditions, null inputs. This code is solid! 😤"
  
  Leaderboard Update:
  [Show new ranking]
  
  🎯 You've set a new high score for [complexity] tasks!
```

#### Improvement Feedback  

```yaml
bugs_found_notification: |
  Agent [N] - QA Results for [Task name]
  
  Base Points: [XXX]
  Bugs Found:
  - 🏆 Minor: [Description] (-0 points, learning opportunity)
  - 🏆🏆 Major: [Description] (-25% penalty)
  
  POSITIVE HIGHLIGHTS:
  ✅ Core functionality works correctly
  ✅ Code structure is clean
  ✅ Good error messages
  
  GROWTH OPPORTUNITIES:
  📚 Edge case handling (see test case #3)
  📚 Input validation could be stricter
  
  Points After Penalties: [XXX]
  Quality Bonuses Still Available:
  - Fix bugs for "Minor Bugs Only" (1.5x)
  - Already earned "Clean Code" (1.3x)
  
  TOTAL CURRENT: [YYY] points
  POTENTIAL IF FIXED: [ZZZ] points
  
  💡 Pro tip from Agent 7 (current leader):
  "I always write a test for every edge case in the spec
  BEFORE implementing. Saved me from these exact bugs!"
  
  You're close to greatness! Fix and resubmit? 🚀
```

## Creating Your Parallel Execution Plan

### Step 1: List All Tasks

```yaml
all_tasks:
  - id: "apply-theme"
    description: "Apply consistent visual theme"
    files: ["src/ui/app.rs", "src/ui/theme.rs"]
    
  - id: "minimize-header"  
    description: "Reduce header to 1 line"
    files: ["src/ui/header.rs"]
    
  - id: "add-sparkline"
    description: "Add bandwidth graph"  
    files: ["src/ui/bandwidth.rs"]
```

### Step 2: Build Dependency Graph

```
apply-theme ──┐
              ├─→ themed-widgets
              └─→ themed-progress-bar
              
minimize-header (independent)

add-sparkline (independent)
```

### Step 3: Group Into Waves

```yaml
wave_1:  # No dependencies, different files
  - apply-theme (Agent 1)
  - minimize-header (Agent 2)  
  - add-sparkline (Agent 3)

wave_2:  # Depends on wave_1
  - themed-widgets (Agent 4) - needs apply-theme
  - themed-progress-bar (Agent 5) - needs apply-theme
```

## Dependency Management

### Parallel Execution Matrix

```yaml
execution_waves:
  wave_1:  # Can run simultaneously
    - task_id: "theme-application"
      agent: 1
      files_modified: ["src/ui/theme.rs", "src/ui/app.rs"]
      
    - task_id: "header-update"
      agent: 2
      files_modified: ["src/components/header.rs"]
      
    - task_id: "data-processing"
      agent: 3
      files_modified: ["src/data/processor.rs"]
  
  wave_2:  # Depends on wave_1 completions
    dependencies:
      - task_id: "theme-integration"
        agent: 4
        requires: ["theme-application"]
        files_modified: ["src/ui/components/*.rs"]
```

### Conflict Prevention Rules

1. No two agents modify the same file in the same wave
2. Dependent tasks wait for prerequisite PR merges
3. Each agent works on a unique feature branch
4. All merges go through PR review process

## Progress Monitoring Dashboard

```yaml
# Orchestrator can query status
mcp_github_search_issues:
  repository: "owner/repo"
  query: "is:pr is:open label:agent-task"
  sort: "created"
  order: "asc"
```

### Collaborative Bonus System

```yaml
team_bonuses:
  helping_events:
    unblock_agent: 50 points      # Help another agent get unstuck
    share_solution: 25 points     # Share reusable code/pattern
    prevent_bug: 75 points        # Warn about potential issue
    
  team_achievements:
    all_pass_qa: 100 points each         # Every agent in wave passes QA
    zero_integration_bugs: 150 each      # No bugs at integration
    ahead_of_schedule: 50 each           # Wave completes early
```

### Leaderboard Management

```yaml
orchestrator_leaderboard_update: |
  📊 LEADERBOARD UPDATE - Sprint 3, Day 2
  
  🏆 TOP PERFORMERS:
  🥇 Agent 7: 2,150 pts (2 complex tasks, zero bugs!)
  🥈 Agent 3: 1,425 pts (1 complex, 1 medium, helped Agent 5)
  🥉 Agent 9: 1,200 pts (3 simple tasks, all clean code)
  
  🌟 QUALITY CHAMPIONS (Zero Bug Club):
  - Agent 7 (2 tasks) 
  - Agent 2 (1 task)
  - Agent 9 (3 tasks)
  
  🤝 COLLABORATION STARS:
  - Agent 3: Helped unblock 2 agents (+100 pts)
  - Agent 6: Shared auth solution (+25 pts)
  
  📈 BIGGEST IMPROVEMENTS:
  - Agent 4: From major bugs to zero bugs! 
  - Agent 8: Reduced bug count by 80%
  
  🎯 CURRENT SPRINT GOALS:
  - Team Goal: All agents over 1,000 points (70% complete)
  - Quality Goal: 80% zero-bug rate (Currently: 65%)
  - Collaboration Goal: 10 helping events (Currently: 7)
  
  💡 TIP OF THE DAY (from Agent 7):
  "I spend 20% of my time writing the feature,
  80% thinking about how QA will try to break it!"
```

### Fair Competition Strategies

```yaml
fairness_principles:
  1_balanced_distribution:
    - Rotate complexity levels among agents
    - Everyone gets mix of simple/medium/complex
    - Track cumulative complexity assigned
    
  2_skill_matching:
    - Assign harder tasks to higher performers
    - But ensure everyone gets growth opportunities
    - No agent stuck with only simple tasks
    
  3_second_chances:
    - Agents can fix bugs and resubmit
    - Penalties reduce but don't eliminate points
    - Learning from mistakes is rewarded
    
  4_diverse_paths_to_win:
    - Volume path: Many simple tasks done well
    - Quality path: Few complex tasks, zero bugs
    - Helper path: Moderate tasks + collaboration
    - Improver path: Bonus for most improved
```

## Example Orchestration Flow with QA

### Complete Implementation + QA Example

#### Phase 1: Implementation Wave

```yaml
# Launch 3 implementation agents
agents:
  - id: 1
    task: "Add user authentication"
    branch: "feature/auth"
    
  - id: 2  
    task: "Create dashboard UI"
    branch: "feature/dashboard"
    
  - id: 3
    task: "Add API endpoints"
    branch: "feature/api"
```

#### Phase 2: QA Wave (After PRs created)

```yaml
# Launch QA agents for each PR
qa_agents:
  - id: 4
    task: "QA authentication - security focus"
    reviewing_pr: 101  # PR from Agent 1
    
  - id: 5
    task: "QA dashboard - UX testing" 
    reviewing_pr: 102  # PR from Agent 2
    
  - id: 6
    task: "QA API - integration tests"
    reviewing_pr: 103  # PR from Agent 3
```

#### Phase 3: Orchestrator Merges

```yaml
for each PR:
  if implementation_pr.approved and qa_tests_pass:
    mcp_github_merge_pull_request(pr)
    if qa_agent_created_test_pr:
      mcp_github_merge_pull_request(test_pr)
```

### 1. Launch Wave 1 Agents

```yaml
for agent in wave_1:
  - Assign workspace: .agent-{N}/
  - Provide full context bundle
  - Give specific task instructions
  - Agent creates feature branch
  - Agent implements task
  - Agent pushes and creates PR
```

### 2. Monitor Completion

```yaml
while wave_1_incomplete:
  - mcp_github_list_pull_requests
  - Check PR status and reviews
  - When PR ready: mcp_github_merge_pull_request
  - Create checkpoint tag
  - Update dependency tracker
```

### 3. Launch Dependent Waves

```yaml
for agent in wave_2:
  if dependencies_met:
    - Launch agent with context
    - Include results from dependencies
```

## Agent Completion Checklist

Each agent should:

1. ✅ Create feature branch
2. ✅ Implement assigned task
3. ✅ Test implementation
4. ✅ Commit with descriptive message
5. ✅ Push to origin
6. ✅ Create PR with:
   - Clear title: "✅ [Task]: [What was achieved]"
   - Description of changes
   - Link to task requirements

## Cleanup

After all tasks complete:

```bash
# Remove agent workspaces
rm -rf .agent-*/
```

## Quality Incentive System (Practical Implementation)

### Simple Success Metrics

```yaml
implementation_priorities:
  1_must_work: "Code must actually function as specified"
  2_must_be_complete: "All requirements implemented, no TODOs"
  3_must_be_tested: "Real tests with assertions, not placeholder"
  4_must_be_maintainable: "Clear code others can understand"

quality_indicators:
  green_flags:
    - Handles all edge cases from spec
    - Tests cover error scenarios
    - Clear error messages
    - Defensive programming
    
  red_flags:  # Automatic rejection
    - Empty catch blocks
    - Tests without assertions
    - Commented out code
    - "TODO" or "FIXME" in PR
    - Functions over 50 lines
```

### Practical Feedback Examples

#### Good Implementation

```yaml
pr_approved_message: |
  PR #123 APPROVED ✅
  
  What worked well:
  - All spec requirements implemented
  - Edge cases handled (empty input, max values)
  - Clear error messages
  - Tests actually test the functionality
  
  QA found: 1 minor issue (already fixed)
  Ready to merge.
```

#### Needs Improvement  

```yaml
pr_changes_requested: |
  PR #123 CHANGES REQUESTED
  
  Issues found:
  - Missing error handling for network timeout
  - Test covers happy path only
  - Large function needs decomposition (75 lines)
  
  Please address and resubmit.
  See QA agent's test cases in PR #124 for examples.
```

## Customization Points

1. **Point Values**: Adjust based on your project's complexity
2. **Competition Length**: Daily, sprint-based, or project-long
3. **Team vs Individual**: Balance competition with collaboration
4. **Skill Levels**: Add handicaps or tiers for fairness
5. **Special Achievements**: Create project-specific bonuses

This gamification system creates healthy competition while maintaining code quality and team collaboration!

## Meta-Prompting Techniques for Orchestrators

### 1. Chain of Responsibility

```yaml
orchestrator_decision_chain: |
  Before assigning any task, ask yourself:
  1. "What could go wrong if two agents modify this simultaneously?"
  2. "What's the smallest useful unit of work here?"
  3. "What context does this agent ACTUALLY need?"
  
  Only provide context that answers: "What am I building and why?"
  Never provide: Implementation details from other agents
```

### 2. Uncertainty Expression

```yaml
when_orchestrator_unsure: |
  If you're unsure about dependencies:
  "I'm uncertain if Task A blocks Task B. Let me analyze the files..."
  
  If merge conflicts seem likely:
  "There's a 70% chance of conflict here. Sequencing these tasks."
  
  NEVER pretend certainty when you don't have it.
```

### 3. Failure Mode Planning

```yaml
pre_mortem_thinking: |
  For each parallel wave, consider:
  - "What if Agent 2's PR breaks Agent 1's work?"
  - "What if the QA agent finds critical bugs?"
  - "What if an agent goes offline mid-task?"
  
  Always have: Rollback plan, Re-assignment strategy
```

### 4. Context Windowing

```yaml
agent_context_limits: |
  Each agent gets ONLY:
  - Their specific task description
  - Required specification files
  - No chat history
  - No other agents' work
  - No "background" or "FYI" information
  
  If it's not needed for THIS task, don't include it.
```

### 5. Explicit Reasoning Traces

```yaml
orchestrator_must_show_work: |
  ❌ "Assigning Task A to Agent 1"
  ✅ "Assigning Task A to Agent 1 because:
      - No file conflicts with current PRs
      - Dependencies (auth system) already merged
      - Agent 1's workspace is fresh"
```

### 6. Anti-Hallucination Anchors

```yaml
ground_truth_checks: |
  Before EVERY decision, verify with MCP:
  
  # Check file structure
  mcp_github_list_directory:
    repository: "owner/repo" 
    path: "src/ui/components"
    
  # Check PR status
  mcp_github_list_pull_requests:
    repository: "owner/repo"
    state: "open"
    
  # Check branches
  mcp_github_list_branches:
    repository: "owner/repo"
  
  Don't assume - VERIFY with actual API calls.
```

### 7. Constraint Propagation

```yaml
cascading_constraints: |
  If Phase 1.1 is delayed:
  → Phase 2.3 must wait (depends on theme)
  → Phase 3.1 must wait (depends on theme)
  → But Phase 2.1, 2.2 can still proceed
  
  Always trace constraint impacts forward.
```

### 8. Cognitive Load Management

```yaml
orchestrator_focus_rules: |
  Track maximum 3 things:
  1. What's currently being worked on
  2. What's blocked and why
  3. What's next when current work completes
  
  Everything else: Check when needed, don't hold in memory
```

### 9. Revision Triggers

```yaml
when_to_revise_plan: |
  MUST re-evaluate if:
  - Any PR fails QA
  - Merge conflicts detected
  - Agent reports blocked
  - New requirements emerge
  
  Say: "Initial plan needs revision because..."
  Not: "Everything is fine" (when it's not)
```

### 10. Success Criteria Clarity

```yaml
define_done_explicitly: |
  ❌ "Implement the theme"
  ✅ "Theme is implemented when:
      - Background is #0e0c14 everywhere
      - All text uses theme colors
      - PR passes visual inspection
      - No hardcoded colors remain"
```

Be Useful. Not Thorough.
