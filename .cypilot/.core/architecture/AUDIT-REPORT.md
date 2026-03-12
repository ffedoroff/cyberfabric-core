# Cypilot Cross-Validation Audit Report

**Date**: 2026-03-12
**Scope**: Full codebase + specifications audit
**Phases completed**: 11/11

## Executive Summary

The Cypilot project is in **good health** with strong architectural foundations. All 14 p1 functional requirements have code implementations, all 14 ADRs are reflected in the DESIGN, and both codebases (proxy and skill engine) comply with architectural constraints (stdlib-only, stateless proxy, no SDLC code in core). The primary systemic issue is **incomplete ADR-0001 propagation**: the blueprint system was correctly removed from code, but spec artifacts (CLISPEC, DECOMPOSITION, 3 FEATURE specs) still reference the removed system. This single root cause accounts for roughly one-third of all HIGH-severity findings across phases. Secondary concerns are PRD quality issues (implementation details embedded in requirements) and documentation gaps (CLISPEC not reflecting actual CLI surface). No blocking defects were found.

### Issue Summary

| Severity | Raw Count | After Dedup (est.) |
|----------|-----------|-------------------|
| CRITICAL | 6 | 6 |
| HIGH | 29 | ~15 |
| MEDIUM | 29 | ~20 |
| LOW | 30 | ~20 |
| **Total** | **94** | **~61** |

Major deduplication: the stale `blueprint-system` ID and ADR-0001 propagation failure account for ~15 raw findings across phases 2, 6, 8, 9, and 10.

### Top Findings

1. **ADR-0001 propagation failure** — Blueprint system removed from code but not from CLISPEC, DECOMPOSITION, or 3 FEATURE specs — Phases 2, 6, 8, 9, 10
2. **Implementation details in PRD** — TOML format, directory paths, VS Code UI specs embedded in requirements instead of deferred to DESIGN — Phase 4
3. **ADR-0013 filename/ID slug mismatch** — File says `extract-sdlc-kit-to-github`, ID says `extract-sdlc-kit`, breaking automated resolution — Phase 7

## Metrics Dashboard

| Metric | Value | Assessment |
|--------|-------|-----------|
| PRD → Code traceability | 15/21 FRs full chain (71%) | needs-work — 4 expected p2/p3 backlog, 1 partial, 1 broken DECOMP link |
| PRD → DESIGN coverage | 21/21 FRs (100%) | good |
| DESIGN → DECOMP coverage | 7/7 components (100%) | good |
| Spec quality (PRD) | 9 issues (3C, 3H, 3M) | needs-work — implementation details in requirements |
| Spec quality (DESIGN) | 7 issues (0C, 0H, 2M, 5L) | good |
| Spec quality (DECOMP+FEATURE) | 20 issues (2C, 8H, 7M, 3L) | needs-work — stale IDs, missing checkboxes |
| Spec quality (ADRs) | 13 issues (1C, 4H, 5M, 3L) | needs-work — status staleness, informal refs |
| ADR → DESIGN compliance | 12/14 fully reflected (86%) | good — 2 partial (ADR-0004 dual-mode, ADR-0012 marker format) |
| Code → Spec coverage (proxy) | 22/25 symbols traced (88%) | good |
| Code → Spec coverage (skill) | 22/22 commands traced (100%) | good |
| SDLC boundary compliance | 0 violations | good |
| Priority inversions | 0 | good |

## Findings by Category

### 1. Traceability Gaps

**Sources**: Phases 1, 2, 10

**Pattern**: The PRD→DESIGN link is solid (100% FR coverage). The DESIGN→DECOMPOSITION link is solid (100% component coverage). Gaps emerge at the DECOMPOSITION→Code level for p2/p3 items and in NFR allocation.

| ID | Finding | Severity | Phases |
|----|---------|----------|--------|
| T-01 | `fr-core-toc` unallocated in DECOMPOSITION despite being fully implemented | MEDIUM | 2, 10 |
| T-02 | 5 NFRs unallocated by NFR ID in DECOMPOSITION (addressed via Design Principle refs instead) | HIGH | 2 |
| T-03 | Interface ID divergence between PRD and DESIGN for CLI and GitHub interfaces | HIGH | 1 |
| T-04 | 2 FEATURE specs have weak backlinks (generic refs, not specific FR/component IDs) | MEDIUM | 2 |
| T-05 | Resource Diff Engine lacks formal component ID in DESIGN | LOW | 2 |
| T-06 | DESIGN Section 5 omits ADRs 0013 and 0014 from traceability list (listed in §1.2) | HIGH | 3 |
| T-07 | PRD acceptance criteria cover only 6/21 FRs | HIGH | 4 |
| T-08 | Missing use cases for 3 p2 FRs (`doctor`, `template-qa`, `vscode-plugin`) | HIGH | 4 |

### 2. Specification Quality Issues

**Sources**: Phases 4, 5, 6, 7

**Pattern**: The DESIGN document is the highest-quality spec (0 CRITICAL issues). The PRD has the most structural problems (implementation details leaking into requirements). FEATURE specs suffer from stale IDs and inconsistent implementation tracking.

| ID | Finding | Severity | Phase |
|----|---------|----------|-------|
| Q-01 | Implementation decisions embedded in PRD FRs (TOML format, directory paths, line budgets) | CRITICAL | 4 |
| Q-02 | Technical implementation details in PRD (filenames, VS Code UI specs) | CRITICAL | 4 |
| Q-03 | Architectural decision stated in PRD ("single-layer generic engine") | CRITICAL | 4 |
| Q-04 | `fr-core-kits` is monolithic — 9 sub-requirements spanning 4+ concerns | MEDIUM | 4 |
| Q-05 | Vague term "intuitive" in `nfr-simplicity` (untestable) | MEDIUM | 4 |
| Q-06 | VS Code Plugin FR contains spec-level design (10 UI behaviors) | HIGH | 4 |
| Q-07 | Acronyms (PRD, SDLC, CI/CD, etc.) not expanded on first use | MEDIUM | 4 |
| Q-08 | No system context diagram in DESIGN | MEDIUM | 5 |
| Q-09 | Module/package structure not documented in DESIGN | MEDIUM | 5 |
| Q-10 | v2-v3-migration.md missing standard "Implementation Modules" section | CRITICAL | 6 |
| Q-11 | Kit Management flow/algo checkbox inconsistency (flow unchecked, algo checked) | CRITICAL | 6 |
| Q-12 | 3 DECOMPOSITION features show ⏳ but have checked `[x]` feature checkboxes | MEDIUM | 6 |
| Q-13 | ~20 CDSL steps in `agent-integration.md` and `developer-experience.md` lack checkboxes | HIGH | 6 |
| Q-14 | ADR-0013 filename slug (`extract-sdlc-kit-to-github`) ≠ ID slug (`extract-sdlc-kit`) | CRITICAL | 7 |
| Q-15 | ADR-0001 and ADR-0009 lack explicit risk documentation in consequences | HIGH | 7 |
| Q-16 | 3 ADRs use informal traceability references instead of canonical `cpt-*` IDs | MEDIUM | 7 |
| Q-17 | ADR-0013 and ADR-0014 ID placement before TOC (inconsistent with ADRs 0001–0012) | MEDIUM | 7 |

### 3. Code-Specification Drift

**Sources**: Phases 8, 9, 10

**Pattern**: A single root cause — **ADR-0001 not propagated to spec artifacts** — produces most drift. Code was correctly updated when the blueprint system was removed, but CLISPEC, DECOMPOSITION, and FEATURE specs were not. Secondary drift: undocumented commands and flags in the actual CLI.

| ID | Finding | Severity | Phases |
|----|---------|----------|--------|
| D-01 | CLISPEC `update` description references removed blueprint system | HIGH | 8, 10 |
| D-02 | CLISPEC defines removed commands (`kit migrate`, `generate-resources`) | HIGH | 8, 10 |
| D-03 | DECOMPOSITION Feature 2.2 uses stale ID `cpt-cypilot-feature-blueprint-system` | HIGH | 2, 6, 10 |
| D-04 | 3 FEATURE specs declare stale `blueprint-system` dependency (F5, F7, F10) | HIGH | 2, 6, 10 |
| D-05 | `agents` command implemented but not in CLISPEC | MEDIUM | 9, 10 |
| D-06 | Proxy flags (`--source`, `--url`, `--no-cache`, `--version`) not in CLISPEC | MEDIUM | 8, 10 |
| D-07 | `self-check` routed as alias to `validate-kits` (CLISPEC implies standalone) | MEDIUM | 9, 10 |
| D-08 | "Background" version check is synchronous post-execution (wording drift) | LOW | 8 |
| D-09 | Stale blueprint references in `toc.py` docstrings | LOW | 9, 10 |
| D-10 | DECOMPOSITION F12 references non-existent `workflow-engine` component | MEDIUM | 2, 6 |
| D-11 | Fork URL support and local copy capability undocumented in specs | LOW | 8, 10 |
| D-12 | ADR-0013 status `proposed` but decision fully acted upon (SDLC kit extracted) | HIGH | 3, 7 |
| D-13 | ADR-0014 status `proposed` but DESIGN treats as accepted | HIGH | 3, 7 |
| D-14 | PRD checkboxes `[ ]` for 4 FRs that are fully implemented | LOW | 10 |

### 4. Architecture Compliance

**Sources**: Phases 3, 8, 9

**Pattern**: Architecture compliance is strong. All ADRs are reflected in DESIGN, code follows the stateless proxy pattern, stdlib-only constraint is met, and SDLC kit extraction is clean.

| ID | Finding | Severity | Phases |
|----|---------|----------|--------|
| A-01 | ADR-0004 dual-mode CLI output not fully captured in DESIGN | MEDIUM | 3 |
| A-02 | ADR-0012 git-style conflict marker format not specified in DESIGN | MEDIUM | 3 |
| A-03 | Missing `cypilot` console entry point (only `cpt` registered) | HIGH | 8, 10 |
| A-04 | `kit move-config` specified in DESIGN §3.3 + DECOMPOSITION but not implemented (p1 scope) | HIGH | 9, 10 |
| A-05 | 5 DESIGN §3.3 commands unimplemented: `doctor`, `config show`, `config system add/remove`, `hook install/uninstall` | MEDIUM | 9 |

### 5. Dead Code & Cleanup

**Sources**: Phases 8, 9

**Pattern**: Very little dead code. The codebase is clean. Remaining items are intentional deprecated stubs and minor documentation debt.

| ID | Finding | Severity | Phase |
|----|---------|----------|-------|
| C-01 | Legacy fallback scan in `find_install_dir` — searches for old directory names | LOW | 8 |
| C-02 | 2 intentional deprecated command stubs (`validate-code`, `validate-rules`) | LOW | 9 |
| C-03 | Init/Update special routing not documented in CDSL flow | LOW | 8 |

## Prioritized Recommendations

### Fix Now (CRITICAL)

| # | Recommendation | Findings | Files | Effort |
|---|---------------|----------|-------|--------|
| 1 | Rename `cpt-cypilot-feature-blueprint-system` → `kit-management` globally | D-03, D-04, Q-11 | `DECOMPOSITION.md`, `kit-management.md`, `agent-integration.md`, `version-config.md`, `v2-v3-migration.md` | S |
| 2 | Fix ADR-0013 filename to match ID slug | Q-14 | `architecture/ADR/0013-cpt-cypilot-adr-extract-sdlc-kit-to-github-v1.md` → rename | S |
| 3 | Add "Implementation Modules" section to `v2-v3-migration.md` | Q-10 | `architecture/features/v2-v3-migration.md` | S |
| 4 | Reconcile kit-management flow/algo checkbox mismatch | Q-11 | `architecture/features/kit-management.md` | S |
| 5 | Extract implementation details from PRD FRs to DESIGN | Q-01, Q-02 | `architecture/PRD.md` | M |
| 6 | Reframe "single-layer generic engine" as product concept | Q-03 | `architecture/PRD.md` §1.1 | S |

### Fix Soon (HIGH)

| # | Recommendation | Findings | Files | Effort |
|---|---------------|----------|-------|--------|
| 7 | Propagate ADR-0001 to CLISPEC: remove `generate-resources`, deprecate `kit migrate`, fix `update` description | D-01, D-02 | `skills/cypilot/cypilot.clispec` | S |
| 8 | Add `agents` command and proxy flags to CLISPEC | D-05, D-06 | `skills/cypilot/cypilot.clispec` | S |
| 9 | Add `cypilot` console entry point | A-03 | `pyproject.toml` | S |
| 10 | Update ADR-0013 and ADR-0014 status to `accepted` | D-12, D-13 | `architecture/ADR/0013-*.md`, `architecture/ADR/0014-*.md` | S |
| 11 | Add ADRs 0013/0014 to DESIGN Section 5 traceability list | T-06 | `architecture/DESIGN.md` §5 | S |
| 12 | Add acceptance criteria for remaining 15 FRs in PRD | T-07 | `architecture/PRD.md` §9 | M |
| 13 | Implement `kit move-config` command (p1 FR scope) | A-04 | `skills/cypilot/scripts/cypilot/commands/` | M |
| 14 | Add checkbox markers to ~20 CDSL steps in `agent-integration.md` + `developer-experience.md` | Q-13 | `architecture/features/agent-integration.md`, `developer-experience.md` | S |
| 15 | Add risk documentation to ADR-0001 and ADR-0009 consequences | Q-15 | `architecture/ADR/0001-*.md`, `architecture/ADR/0009-*.md` | S |

### Improve Later (MEDIUM/LOW)

| # | Recommendation | Findings | Files | Effort |
|---|---------------|----------|-------|--------|
| 16 | Add `fr-core-toc` to F8 "Requirements Covered" in DECOMPOSITION | T-01 | `architecture/DECOMPOSITION.md` §2.8 | S |
| 17 | Fix DECOMP F12 component ref: `workflow-engine` → `component-agent-generator` | D-10 | `architecture/DECOMPOSITION.md` §2.12 | S |
| 18 | Update DECOMPOSITION status indicators (3 features show ⏳ but are checked) | Q-12 | `architecture/DECOMPOSITION.md` | S |
| 19 | Add system context diagram to DESIGN | Q-08 | `architecture/DESIGN.md` §1.1 | M |
| 20 | Document module/package structure mapping in DESIGN | Q-09 | `architecture/DESIGN.md` | M |
| 21 | Use canonical `cpt-*` IDs in ADR traceability sections | Q-16 | `architecture/ADR/0006-*.md`, `0009-*.md`, `0012-*.md` | S |
| 22 | Update PRD checkboxes for implemented FRs still marked `[ ]` | D-14 | `architecture/PRD.md` | S |
| 23 | Expand acronyms on first use in PRD | Q-07 | `architecture/PRD.md` | S |
| 24 | Mark unimplemented p2/p3 commands as `planned` in DESIGN §3.3 | A-05 | `architecture/DESIGN.md` §3.3 | S |
| 25 | Clean stale blueprint docstrings in `toc.py` | D-09 | `skills/cypilot/scripts/cypilot/utils/toc.py` | S |
| 26 | Move VS Code Plugin detailed behaviors from PRD to FEATURE spec | Q-06 | `architecture/PRD.md`, new FEATURE spec | M |
| 27 | Capture ADR-0004 dual-mode and ADR-0012 marker format in DESIGN | A-01, A-02 | `architecture/DESIGN.md` | S |

## Phase Completion Log

| Phase | Title | Status | Issues |
|-------|-------|--------|--------|
| 1 | PRD ↔ DESIGN Cross-Reference | PASS | 6 (0C, 2H, 2M, 2L) |
| 2 | DESIGN ↔ DECOMPOSITION ↔ FEATURE Chain | PASS | 8 (0C, 3H, 3M, 2L) |
| 3 | ADR ↔ DESIGN Consistency | PASS | 6 (0C, 2H, 2M, 2L) |
| 4 | PRD Quality Review | PASS | 9 (3C, 3H, 3M, 0L) |
| 5 | DESIGN Quality Review | PASS | 7 (0C, 0H, 2M, 5L) |
| 6 | DECOMPOSITION + FEATURE Quality Review | PASS | 20 (2C, 8H, 7M, 3L) |
| 7 | ADR Batch Quality Review | PASS | 13 (1C, 4H, 5M, 3L) |
| 8 | Proxy Codebase vs Specs | PASS | 12 (0C, 3H, 1M, 8L) |
| 9 | Skill Engine vs Specs | PASS | 4 (0C, 0H, 2M, 2L) |
| 10 | Specs vs Codebase Gap Analysis | PASS | 9 (0C, 4H, 2M, 3L) |
| 11 | Consolidated Audit Report | PASS | — |
| **Total** | | **ALL PASS** | **94 raw (6C, 29H, 29M, 30L)** |

---

*Report generated as part of the `analyze-full-audit-cypilot` execution plan. Phase findings are stored in `.bootstrap/.plans/analyze-full-audit-cypilot/out/`.*
