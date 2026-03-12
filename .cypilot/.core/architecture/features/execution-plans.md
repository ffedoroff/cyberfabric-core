# Feature: Execution Plans


<!-- toc -->

- [1. Feature Context](#1-feature-context)
  - [1.1 Overview](#11-overview)
  - [1.2 Purpose](#12-purpose)
  - [1.3 Actors](#13-actors)
  - [1.4 References](#14-references)
- [2. Actor Flows (CDSL)](#2-actor-flows-cdsl)
  - [Generate Execution Plan](#generate-execution-plan)
  - [Execute Phase](#execute-phase)
  - [Check Plan Status](#check-plan-status)
- [3. Processes / Business Logic (CDSL)](#3-processes--business-logic-cdsl)
  - [Decompose Task](#decompose-task)
  - [Compile Phase File](#compile-phase-file)
  - [Enforce Line Budget](#enforce-line-budget)
- [4. States (CDSL)](#4-states-cdsl)
  - [Plan Lifecycle](#plan-lifecycle)
  - [Phase Lifecycle](#phase-lifecycle)
- [5. Definitions of Done](#5-definitions-of-done)
  - [Plan Workflow](#plan-workflow)
  - [Phase File Template](#phase-file-template)
  - [Decomposition Strategies](#decomposition-strategies)
  - [Plan Storage](#plan-storage)
- [6. Acceptance Criteria](#6-acceptance-criteria)

<!-- /toc -->

- [x] `p1` - **ID**: `cpt-cypilot-featstatus-execution-plans`
## 1. Feature Context

- [x] `p1` - `cpt-cypilot-feature-execution-plans`

### 1.1 Overview

Execution Plans decompose large agent tasks (artifact generation, validation, code implementation) into self-contained phase files that fit within a single LLM context window. Each phase file is a compiled prompt — all rules, constraints, conventions, and context are pre-resolved and inlined so that any AI agent can execute it without Cypilot knowledge.

### 1.2 Purpose

Context window overflow is the primary source of non-deterministic results in Cypilot workflows. A single generate or analyze invocation can load 3000+ lines of instructions (SKILL.md + execution-protocol.md + workflow + rules + template + checklist + example + constraints + project context) before the agent writes any output. This causes:

- **Attention drift**: different parts of instructions "win" attention on each run, producing inconsistent results
- **Partial completion**: agent runs out of context mid-task, requiring manual re-scoping
- **Manual decomposition**: users must figure out how to break tasks into manageable pieces

Execution Plans solve this by moving decomposition from the user to the tool. The plan workflow reads all relevant sources once, decomposes the task into phases, and "compiles" each phase into a focused instruction file (≤500 lines target, ≤1000 max) containing only what's needed for that specific sub-task.

**Requirements**: `cpt-cypilot-fr-core-workflows`, `cpt-cypilot-fr-core-execution-plans`

**Principles**: `cpt-cypilot-principle-determinism-first`, `cpt-cypilot-principle-occams-razor`

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cypilot-actor-user` | Invokes plan workflow, reviews generated phases, triggers phase execution, checks plan progress |
| `cpt-cypilot-actor-ai-agent` | Generates execution plans, compiles phase files, executes individual phases |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md) — `cpt-cypilot-fr-core-workflows`, `cpt-cypilot-fr-core-execution-plans`
- **Design**: [DESIGN.md](../DESIGN.md) — `cpt-cypilot-component-agent-generator`
- **Dependencies**: `cpt-cypilot-feature-agent-integration` (builds on generate/analyze workflows)

## 2. Actor Flows (CDSL)

### Generate Execution Plan

- [x] `p1` - **ID**: `cpt-cypilot-flow-execution-plans-generate-plan`

**Actor**: `cpt-cypilot-actor-user`

**Success Scenarios**:
- User requests a large task → agent produces a plan manifest + phase files in `.plans/` directory
- User requests plan for specific artifact → agent decomposes by template sections

**Error Scenarios**:
- Task is small enough for single context → agent skips plan, executes directly via generate/analyze
- Kit dependencies missing → agent reports missing deps and stops
- `.plans/` directory cannot be created → agent reports filesystem error

**Steps**:
1. [x] - `p1` - User requests task via plan workflow (e.g., "plan generate PRD", "plan analyze DESIGN") - `inst-user-request`
2. [x] - `p1` - Agent loads task context: identify task type (generate/analyze/implement), target artifact kind, and kit - `inst-load-context`
3. [x] - `p1` - Agent loads all kit dependencies for target kind: template, rules, checklist, example, constraints - `inst-load-deps`
4. [x] - `p1` - Agent runs decomposition algorithm `cpt-cypilot-algo-execution-plans-decompose` to split task into phases - `inst-decompose`
5. [x] - `p1` - Agent creates `.plans/` directory in `{cypilot_path}` if not exists - `inst-create-dir`
6. [ ] - `p1` - **IF** `.plans/` not in `.gitignore` → agent adds it - `inst-gitignore`  *(not implemented — only `.archive/` is gitignored)*
7. [x] - `p1` - Agent creates plan directory: `{cypilot_path}/.plans/{task-slug}/` - `inst-create-plan-dir`
8. [x] - `p1` - **FOR EACH** phase in decomposition result - `inst-loop-phases`
   1. [x] - `p1` - Agent runs compile algorithm `cpt-cypilot-algo-execution-plans-compile-phase` to produce phase file content - `inst-compile`
   2. [x] - `p1` - Agent runs budget enforcement `cpt-cypilot-algo-execution-plans-enforce-budget` on compiled content - `inst-budget`
   3. [x] - `p1` - Agent writes phase file: `phase-{NN}-{slug}.md` - `inst-write-phase`
9. [x] - `p1` - Agent writes plan manifest: `plan.toml` with all phase metadata - `inst-write-manifest`
10. [x] - `p1` - Agent reports plan summary: total phases, estimated lines per phase, execution order - `inst-report`

### Execute Phase

- [x] `p1` - **ID**: `cpt-cypilot-flow-execution-plans-execute-phase`

**Actor**: `cpt-cypilot-actor-user`

**Success Scenarios**:
- User asks to execute next phase → agent reads phase file, follows instructions, produces output
- All acceptance criteria pass → phase marked done in manifest

**Error Scenarios**:
- Phase depends on incomplete phase → agent reports dependency and stops
- Acceptance criteria fail → phase marked failed, agent reports specifics
- Phase file missing or corrupted → agent reports error

**Steps**:
1. [x] - `p1` - User requests phase execution (next phase or specific phase number) - `inst-user-exec`
2. [x] - `p1` - Agent reads `plan.toml` manifest to determine target phase - `inst-read-manifest`
3. [x] - `p1` - **IF** target phase has unmet dependencies → **RETURN** error with dependency list - `inst-check-deps`
4. [x] - `p1` - Agent updates phase status to `in_progress` in manifest - `inst-update-status-start`
5. [x] - `p1` - Agent reads phase file content (self-contained instructions) - `inst-read-phase`
6. [x] - `p1` - Agent follows phase instructions exactly (the phase file contains ALL needed context) - `inst-execute`
7. [x] - `p1` - Agent self-checks against acceptance criteria in phase file - `inst-self-check`
8. [x] - `p1` - **IF** all acceptance criteria pass - `inst-check-pass`
   1. [x] - `p1` - Agent updates phase status to `done` in manifest - `inst-mark-done`
   2. [x] - `p1` - Agent reports phase completion and next phase - `inst-report-done`
9. [x] - `p1` - **ELSE** - `inst-check-fail`
   1. [x] - `p1` - Agent updates phase status to `failed` in manifest with details - `inst-mark-failed`
   2. [x] - `p1` - Agent reports failed criteria - `inst-report-failed`

### Check Plan Status

- [x] `p2` - **ID**: `cpt-cypilot-flow-execution-plans-check-status`

**Actor**: `cpt-cypilot-actor-user`

**Success Scenarios**:
- User asks for plan status → agent reads manifest and reports phase progress

**Error Scenarios**:
- No active plan found → agent reports no plan

**Steps**:
1. [x] - `p2` - User requests plan status - `inst-user-status`
2. [x] - `p2` - Agent reads `plan.toml` manifest - `inst-read-manifest-status`
3. [x] - `p2` - Agent reports: plan name, total phases, completed/pending/failed counts, next actionable phase - `inst-report-status`

## 3. Processes / Business Logic (CDSL)

### Decompose Task

- [x] `p1` - **ID**: `cpt-cypilot-algo-execution-plans-decompose`

**Input**: Task type (generate/analyze/implement), target artifact kind, kit dependencies (template, checklist, rules)

**Output**: Ordered list of phases, each with: title, scope description, relevant template sections, relevant checklist items, relevant rules subset, dependency list

**Steps**:
1. [x] - `p1` - Determine decomposition strategy based on task type - `inst-determine-strategy`
2. [x] - `p1` - **IF** task type is `generate` (artifact creation) - `inst-strategy-generate`
   1. [x] - `p1` - Parse template into logical section groups (2-4 sections per phase) - `inst-parse-template`
   2. [x] - `p1` - Assign each section group to a phase in template order - `inst-assign-sections`
   3. [x] - `p1` - For each phase, extract only the rules applicable to its sections - `inst-extract-rules`
   4. [x] - `p1` - For each phase, extract only the checklist items applicable to its sections - `inst-extract-checklist`
3. [x] - `p1` - **IF** task type is `analyze` (validation/review) - `inst-strategy-analyze`
   1. [x] - `p1` - Parse checklist into category groups (structural, semantic, cross-reference, traceability) - `inst-parse-checklist`
   2. [x] - `p1` - Assign each category group to a phase - `inst-assign-categories`
   3. [x] - `p1` - Add synthesis phase at end (aggregate results, final verdict) - `inst-add-synthesis`
4. [x] - `p1` - **IF** task type is `implement` (code from FEATURE) - `inst-strategy-implement`
   1. [x] - `p1` - Parse FEATURE CDSL blocks (flows, algorithms, states) - `inst-parse-cdsl`
   2. [x] - `p1` - Assign each CDSL block + its tests to a phase - `inst-assign-cdsl`
   3. [x] - `p1` - Order phases by CDSL dependency graph - `inst-order-by-deps`
5. [x] - `p1` - Set phase dependencies: each phase depends on all prior phases that produce content it references - `inst-set-deps`
6. [x] - `p1` - **RETURN** ordered phase list with metadata - `inst-return-phases`

### Compile Phase File

- [x] `p1` - **ID**: `cpt-cypilot-algo-execution-plans-compile-phase`

**Input**: Phase metadata (from decompose), full kit dependencies, project context

**Output**: Self-contained phase file content (markdown) following `plan-template.md` structure

**Steps**:
1. [x] - `p1` - Generate TOML frontmatter: plan ID, phase number, total, type, status, dependencies, input/output paths - `inst-gen-frontmatter`
2. [x] - `p1` - Write "What" section: 2-3 sentences describing this phase's scope and its place in the plan - `inst-write-what`
3. [x] - `p1` - Write "Prior Context" section: summary of what previous phases produced (or "First phase" if phase 1) - `inst-write-prior`
4. [x] - `p1` - Write "Rules" section: inline ONLY rules applicable to THIS phase's scope - `inst-write-rules`
   1. [x] - `p1` - Extract structural rules relevant to phase's template sections - `inst-extract-structural`
   2. [x] - `p1` - Extract content rules relevant to phase's scope - `inst-extract-content`
   3. [x] - `p1` - Extract quality rules (always included, condensed) - `inst-extract-quality`
5. [x] - `p1` - Write "Input" section: pre-resolve all file paths, inline project context needed for this phase - `inst-write-input`
6. [x] - `p1` - Write "Task" section: numbered step-by-step instructions specific to this phase - `inst-write-task`
7. [x] - `p1` - Write "Acceptance Criteria" section: binary pass/fail checklist for this phase - `inst-write-criteria`
8. [x] - `p1` - Write "Output Format" section: exact expected output format and completion report template - `inst-write-output`
9. [x] - `p1` - Resolve ALL template variables (`{variable}` → absolute paths) in the compiled content - `inst-resolve-vars`
10. [x] - `p1` - **RETURN** compiled phase file content - `inst-return-compiled`

### Enforce Line Budget

- [x] `p1` - **ID**: `cpt-cypilot-algo-execution-plans-enforce-budget`

**Input**: Compiled phase file content, target budget (500 lines), maximum budget (1000 lines)

**Output**: Budget-compliant phase file content, or split recommendation

**Steps**:
1. [x] - `p1` - Count lines in compiled content - `inst-count-lines`
2. [x] - `p1` - **IF** lines ≤ target budget (500) → **RETURN** content as-is - `inst-under-target`
3. [x] - `p1` - **IF** lines > target but ≤ maximum (1000) - `inst-over-target`
   1. [x] - `p1` - ~~Trim rules section: remove rules not directly applicable to phase scope~~ — **SUPERSEDED** by "Kit Rules Are Law" constraint: rules are NEVER trimmed, phases are split instead - `inst-trim-rules`
   2. [x] - `p1` - ~~Condense quality rules to bullet points~~ — **SUPERSEDED** by "Kit Rules Are Law" constraint - `inst-condense-quality`
   3. [x] - `p1` - **IF** still > target → accept (within maximum budget) - `inst-accept-over`
4. [x] - `p1` - **IF** lines > maximum (1000) - `inst-over-max`
   1. [x] - `p1` - **RETURN** split recommendation: suggest splitting this phase into N sub-phases with proposed scope boundaries - `inst-recommend-split`

## 4. States (CDSL)

### Plan Lifecycle

- [x] `p1` - **ID**: `cpt-cypilot-state-execution-plans-plan-lifecycle`

**States**: pending, in_progress, done, failed

**Initial State**: pending

**Transitions**:
1. [x] - `p1` - **FROM** pending **TO** in_progress **WHEN** first phase execution starts - `inst-plan-start`
2. [x] - `p1` - **FROM** in_progress **TO** done **WHEN** all phases are done - `inst-plan-done`
3. [x] - `p1` - **FROM** in_progress **TO** failed **WHEN** any phase fails and user does not retry - `inst-plan-failed`

### Phase Lifecycle

- [x] `p1` - **ID**: `cpt-cypilot-state-execution-plans-phase-lifecycle`

**States**: pending, in_progress, done, failed

**Initial State**: pending

**Transitions**:
1. [x] - `p1` - **FROM** pending **TO** in_progress **WHEN** agent begins executing phase - `inst-phase-start`
2. [x] - `p1` - **FROM** in_progress **TO** done **WHEN** all acceptance criteria pass - `inst-phase-done`
3. [x] - `p1` - **FROM** in_progress **TO** failed **WHEN** acceptance criteria fail - `inst-phase-failed`
4. [x] - `p1` - **FROM** failed **TO** in_progress **WHEN** user retries phase - `inst-phase-retry`

## 5. Definitions of Done

### Plan Workflow

- [x] `p1` - **ID**: `cpt-cypilot-dod-execution-plans-workflow`

The system MUST provide a `plan.md` workflow file that instructs AI agents how to decompose tasks into phases and generate self-contained phase files. The workflow MUST follow the same structure as existing `generate.md` and `analyze.md` workflows.

**Implements**:
- `cpt-cypilot-flow-execution-plans-generate-plan`
- `cpt-cypilot-flow-execution-plans-execute-phase`
- `cpt-cypilot-flow-execution-plans-check-status`

**Constraints**: `cpt-cypilot-constraint-markdown-contract`

**Touches**:
- File: `workflows/plan.md` (new)
- File: `{cypilot_path}/.core/workflows/plan.md` (synced copy)

### Phase File Template

- [x] `p1` - **ID**: `cpt-cypilot-dod-execution-plans-template`

The system MUST provide a `plan-template.md` requirement file that defines the strict structure for generated phase files. The template MUST enforce:
- TOML frontmatter with plan/phase metadata
- Self-contained preamble ("Any AI agent can execute this file")
- Sections: What, Prior Context, Rules (inlined), Input (pre-resolved), Task (step-by-step), Acceptance Criteria (binary), Output Format
- No unresolved template variables
- No external file references that require Cypilot knowledge

**Implements**:
- `cpt-cypilot-algo-execution-plans-compile-phase`

**Constraints**: `cpt-cypilot-constraint-markdown-contract`

**Touches**:
- File: `requirements/plan-template.md` (new)
- File: `{cypilot_path}/.core/requirements/plan-template.md` (synced copy)

### Decomposition Strategies

- [x] `p1` - **ID**: `cpt-cypilot-dod-execution-plans-decomposition`

The system MUST provide a `plan-decomposition.md` requirement file that defines decomposition strategies for each task type:
- **Generate**: split by template section groups (2-4 sections per phase)
- **Analyze**: split by checklist category groups (structural → semantic → cross-ref → traceability → synthesis)
- **Implement**: split by CDSL blocks (each flow/algorithm/state + its tests = 1 phase)

The file MUST include budget enforcement rules (500-line target, 1000-line max) and phase dependency resolution.

**Implements**:
- `cpt-cypilot-algo-execution-plans-decompose`
- `cpt-cypilot-algo-execution-plans-enforce-budget`

**Constraints**: `cpt-cypilot-constraint-markdown-contract`

**Touches**:
- File: `requirements/plan-decomposition.md` (new)
- File: `{cypilot_path}/.core/requirements/plan-decomposition.md` (synced copy)

### Plan Storage

- [x] `p1` - **ID**: `cpt-cypilot-dod-execution-plans-storage`

The system MUST store execution plans in `{cypilot_path}/.plans/{task-slug}/` directory. The directory MUST be added to `.gitignore` automatically on first use. Each plan directory contains:
- `plan.toml` — manifest with phase metadata and status tracking
- `phase-{NN}-{slug}.md` — self-contained phase files

**Implements**:
- `cpt-cypilot-flow-execution-plans-generate-plan`
- `cpt-cypilot-state-execution-plans-plan-lifecycle`
- `cpt-cypilot-state-execution-plans-phase-lifecycle`

**Touches**:
- Directory: `{cypilot_path}/.plans/` (new, git-ignored)

## 6. Acceptance Criteria

- [x] Plan workflow file (`workflows/plan.md`) exists and follows workflow structure conventions
- [x] Phase template file (`requirements/plan-template.md`) exists with all required sections
- [x] Decomposition strategies file (`requirements/plan-decomposition.md`) exists with strategies for generate/analyze/implement
- [x] Generated phase files are self-contained: zero unresolved `{variable}` references, zero "open file X" instructions
- [x] Generated phase files respect line budget: ≤500 lines target, ≤1000 lines maximum
- [x] Phase files can be executed by any AI agent without Cypilot context or tools
- [x] Plan manifest (`plan.toml`) correctly tracks phase status across executions
- [ ] `.plans/` directory is automatically git-ignored *(only `.archive/` is currently gitignored)*
