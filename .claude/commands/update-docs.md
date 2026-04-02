---
description: "After code/doc changes are tested and implemented, verify that all changes are reflected in documentation, code has @cpt-* markers, and Cypilot validation passes."
---

# /update-docs — Documentation Sync After Implementation

Run after the code is tested and passes `make check`.

## ⛔ BLOCKER: Validation Gate (MANDATORY before Step 1)

**STOP. Do not proceed to Step 1 until the gate passes.**

1. Run `make cypilot-validate` — check `Errors:` and `Code coverage:`. If `Errors > 0` OR code coverage < 100% → **STOP**, ask the user: "Cypilot validate: {N} errors, code coverage {X}/{Y}. Fix?"

**Do not continue until the user responds.** If the user says "no" / "skip" — record in the summary as a known gap and continue.

## Step 1: Collect changed files

Collect changes from **two sources**:

1. **Current chat** — review the entire conversation history, collect the list of files created or edited in this session
2. **git diff** — run `git diff --name-only` and `git diff --name-only --cached` (unstaged + staged) for changes made outside this chat but not yet committed

Merge both lists (deduplicated) and split into:
- **Code files**: `modules/**/src/**/*.rs`, `modules/**/tests/**/*.rs`
- **Doc files**: `modules/**/docs/features/*.md`
- **Other**: configs, migrations, Cargo.toml, etc.

## Step 2: For each changed code file — verify documentation

For each changed `.rs` file in `src/`:
1. Read the diff (`git diff`) to understand what changed
2. Determine which feature the change belongs to (by existing `@cpt-*` markers or module path)
3. Read the corresponding `docs/features/*.md` in the same module
4. Check: does the documentation describe the current behavior? If not — update:
   - Algorithm steps (CDSL Steps sections)
   - Definitions of Done
   - Acceptance Criteria
   - Format descriptions, field lists, error codes
5. **Checkbox sync** (⚠️ MANDATORY — for ALL docs, not just the affected ones):
   - Run `grep -rn '\[ \]' modules/**/docs/features/` — collect ALL unchecked IDs and inst-steps across ALL feature documents
   - For each unchecked algo/flow/dod ID check: is there a `@cpt-algo`/`@cpt-flow`/`@cpt-dod` marker with this ID in the code (`grep -r "cpt-{id}" modules/**/src/`). If the marker exists → mark `[x]`
   - For each unchecked inst-step check: is there a `@cpt-begin:...:{inst}` in the code. If yes → mark `[x]`
   - After all markings, check the featstatus of each document: if ALL nested items are `[x]` → restore featstatus `[x]`
   - Run `make cypilot-validate` — ensure that checkbox sync did not create errors

## Step 3: For each changed code file — verify @cpt-* markers

For each changed `.rs` file in `src/`:
1. Read the file and find new/changed functions or logic blocks
2. Check: do new blocks have `@cpt-begin`/`@cpt-end` markers referencing the corresponding algorithm in documentation?
3. Check: do new functions/impls have `@cpt-algo`, `@cpt-flow`, or `@cpt-dod` markers?
4. If markers are missing — add them using IDs from the corresponding algorithm in `docs/features/*.md`
5. Marker format (Rust comments):
   - Function/impl level: `// @cpt-algo:cpt-rg-algo-{name}:p{N}`
   - Block level: `// @cpt-begin:cpt-rg-algo-{name}:p{N}:inst-{step}` / `// @cpt-end:...`

## Step 4: For each changed doc file — verify code

For each changed document in `docs/features/*.md`:
1. Read the diff to understand what changed in the documentation
2. If new algorithm steps (inst-*) were added — verify that corresponding `@cpt-begin`/`@cpt-end` markers exist in the code
3. If steps were removed or renamed — verify that code markers are updated
4. If a checkbox was changed from `[ ]` to `[x]` — confirm the code is actually implemented
5. If a checkbox is `[x]` but the code was removed — reset to `[ ]`

## Step 5: Cypilot validation

Run `make cypilot-validate`. Pass criteria:
- `Errors: 0` — zero errors
- `Code coverage: N/N` — 100% (numerator == denominator)

If it does not pass — fix errors (orphan markers, missing pairs, etc.), re-run until fully passing.

## Step 6: Build and test

```bash
make check
```

Pass criteria:
- All tests green
- Clippy clean
- Cypilot validation passed (included in `check` target)

## Output

⚠️ **Numbers in the summary ONLY from command output.** Copy verbatim from `make check` and `make cypilot-validate` stdout. Do NOT substitute numbers from other runs, do NOT round, do NOT embellish.

Report with a summary:
```
## /update-docs summary

### Documentation updated:
- modules/.../docs/features/X.md — {what changed}

### Markers added:
- modules/.../src/domain/X.rs — {which markers}

### Validation:
- cypilot validate: PASS/FAIL (errors: N, warnings: N, code coverage: X/Y)
- make check: PASS/FAIL
```
