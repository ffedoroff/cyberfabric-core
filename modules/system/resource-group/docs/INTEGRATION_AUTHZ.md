# Integration Checklist: Resource Group ↔ AuthZ

**Module**: `cf-resource-group` + `cf-static-authz-plugin` + `authz-resolver-sdk`
**Scope**: Features 0005, 0006, 0007
**Last updated**: 2026-03-10

## 1. Current Integration State

| Component | Status | Feature | Notes |
|-----------|--------|---------|-------|
| PolicyEnforcer in RG handlers | DONE | 0005 | All endpoints call `access_scope()` / `access_scope_with()` |
| AccessScope propagation to repos | DONE | 0005 | SecureORM filters via `tenant_id` column |
| ResourceType descriptors | DONE | 0005 | `RESOURCE_GROUP` (tenant-scoped), `RESOURCE_GROUP_TYPE` (global) |
| EnforcerError → HTTP mapping | DONE | 0005 | Denied→403, CompileFailed→500, EvaluationFailed→503 |
| Cross-tenant isolation | DONE | 0005 | E2E tests: groups, memberships, mutations |
| Membership subquery scoping | DONE | 0005 | `group_id IN (SELECT id FROM resource_group WHERE tenant_id IN (?))` |
| ResourceGroupReadHierarchy bypass | DONE | 0005 | `AccessScope::allow_all()` — no enforcer loop |
| SDK constraint types | DONE | 0007 | `Eq`, `In`, `InTenantSubtree`, `InGroup`, `InGroupSubtree` |
| Constraint compiler | DONE | 0007 | All 5 predicate types → `ScopeFilter` variants |
| SecureORM SQL generation | DONE | 0007 | Subquery generation for all advanced predicates |
| Static plugin → `InTenantSubtree` | DONE | 0007 | Returns when `TenantHierarchy` capability declared |
| Static plugin → `InGroupSubtree` | NOT DONE | 0007 | Plugin does not yet return `InGroupSubtree` predicates |
| PEP capabilities in RG module | PARTIAL | 0007 | `GroupHierarchy` declared; `TenantHierarchy` deferred (needs `tenant_closure`) |
| MTLS auth path | DEFERRED | 0006 | Blocked on platform MTLS infra |
| Plugin gateway routing | DEFERRED | 0006 | Blocked on vendor plugin architecture |
| `tenant_closure` local projection | NOT DONE | 0007 | Table does not exist in RG migrations |

## 2. Contradictions: Architecture Docs vs Code

### C1: `barrier_mode` value naming

**Severity**: HIGH — JSON contract mismatch between docs and wire format

| Source | Values | Location |
|--------|--------|----------|
| `docs/arch/authorization/DESIGN.md` | `"all"` / `"none"` | Lines 807, 840, 881, 1114, 1138-1139 |
| `docs/arch/authorization/TENANT_MODEL.md` | `"all"` / `"none"` | Line 190 |
| `docs/arch/authorization/AUTHZ_USAGE_SCENARIOS.md` | `"all"` / `"none"` | Lines 200, 605 |
| `authz-resolver-sdk/src/models.rs` `BarrierMode` | `"respect"` / `"ignore"` | `serde(rename_all = "snake_case")` |
| `authz-resolver-sdk/src/constraints.rs` `PredicateBarrierMode` | `"respect"` / `"ignore"` | `serde(rename_all = "snake_case")` |

Both `BarrierMode` (request-level) and `PredicateBarrierMode` (predicate-level) use `Respect`/`Ignore` with `serde(rename_all = "snake_case")`, serializing to `"respect"`/`"ignore"`. Architecture docs consistently use `"all"`/`"none"`.

**Resolution options**:
1. **(A) Code adapts**: Add `#[serde(alias = "all")]` on `Respect`, `#[serde(alias = "none")]` on `Ignore` for backward compat; update docs to document both forms
2. **(B) Docs adapt**: Update all arch docs from `"all"`/`"none"` to `"respect"`/`"ignore"`
3. **(Recommended)**: Option (A) — accept both on deserialization, canonicalize docs to `"respect"`/`"ignore"` with note about legacy `"all"`/`"none"` alias

### C2: RG module does not declare PEP capabilities

**Severity**: MEDIUM — advanced predicates never reach RG in production

`module.rs:69`: `PolicyEnforcer::new(authz)` — no `with_capabilities()` call.

Result: `capabilities: vec![]` in every `EvaluationRequest`. The static-authz-plugin checks `request.context.capabilities.contains(&Capability::TenantHierarchy)` and falls back to flat `In` predicates when empty.

Current behavior is correct (flat `In` works) but prevents `InTenantSubtree` from being used, making the Feature 0007 compiler+SecureORM code unreachable in production.

Test `cross_module_authz_rg` manually sets `capabilities: vec![Capability::GroupHierarchy]` — passes, but doesn't reflect the production path.

**Resolution**: Add capabilities when creating PolicyEnforcer in `module.rs`:
```rust
let enforcer = PolicyEnforcer::new(authz)
    .with_capabilities(vec![
        Capability::TenantHierarchy,
        Capability::GroupHierarchy,
    ]);
```

**Prerequisite**: `tenant_closure` local projection must exist before enabling `TenantHierarchy` (otherwise `InTenantSubtree` SQL will fail at runtime).

### C3: `resource_id` type — `String` vs `UUID`

**Severity**: LOW (SQLite) / MEDIUM (PostgreSQL)

| Source | Type | Location |
|--------|------|----------|
| `docs/arch/authorization/DESIGN.md` | `UUID` | Line 756: "resource_group_membership keys: UUID, resource_id, group_id" |
| `resource-group-sdk/models.rs` | `String` | Line 86: `pub resource_id: String` |
| DB entity `resource_group_membership.rs` | `String` | Line 14: `pub resource_id: String` |

`InGroupSubtree` SQL from DESIGN.md: `col IN (SELECT resource_id FROM resource_group_membership WHERE ...)` assumes UUID join. Actual `resource_id` is `String` (stores URN/external identifier of any format).

**Resolution**: Update DESIGN.md table schema — `resource_id` is `TEXT/VARCHAR`, not `UUID`. The `InGroupSubtree` SQL still works because the column type doesn't affect the `IN` subquery semantics; only the join column (`group_id UUID`) matters for correctness.

### C4: `tenant_closure` table absent from RG migrations

**Severity**: MEDIUM — `InTenantSubtree` SQL references non-existent table

Feature 0007 doc describes SQL: `SELECT descendant_id FROM tenant_closure WHERE ancestor_id = ?`

`tenant_closure` is owned by the tenant-resolver module. RG migrations contain `resource_group`, `resource_group_closure`, `resource_group_membership`, `resource_group_type` — but not `tenant_closure`.

DECOMPOSITION.md line 407 mentions "Local projections: `tenant_closure`, `resource_group_closure`, `resource_group_membership`" — implying RG should have a local projection, but it's not implemented.

**Resolution options**:
1. **(A) Local projection migration**: Add a `tenant_closure` migration in RG that creates a local projection table, populated by tenant-resolver events (CDC/sync)
2. **(B) Cross-module query**: Use `tenant-resolver` SDK to resolve descendant IDs before querying RG — avoids local projection but adds latency
3. **(C) Shared schema**: `tenant_closure` lives in a shared schema accessible to all modules
4. **(Recommended)**: Option (A) — aligns with DESIGN.md's local projection strategy. Requires CDC pipeline from tenant-resolver → RG.

### C5: Static plugin `InGroupSubtree` not implemented

**Severity**: LOW (future work)

Feature 0007 doc line 137: "Static AuthZ plugin **SHOULD** return `InGroupSubtree` when `GroupHierarchy` capability is declared with a group_id."

Current static plugin behavior with `GroupHierarchy` capability:
- Validates group belongs to caller's tenant via `ResourceGroupReadHierarchy`
- Returns flat `In(OWNER_TENANT_ID, [tid])` or `InTenantSubtree` — NOT `InGroupSubtree`

The SDK types and compiler for `InGroupSubtree` are implemented, but the plugin doesn't produce them yet.

**Resolution**: Feature 0007 plugin enhancement task (already tracked in DECOMPOSITION.md line 380). Not a contradiction — it's planned but not yet done.

### C6: `require_constraints` semantics documentation gap

**Severity**: LOW — code is correct, docs incomplete

Feature 0005 documents that type operations use `require_constraints(false)` because types are global resources. But the relationship between `require_constraints`, `ResourceType.supported_properties`, and PDP behavior is not explicitly documented anywhere.

The implicit contract:
- `RESOURCE_GROUP_TYPE` has `supported_properties: &[RESOURCE_ID]` (no `OWNER_TENANT_ID`)
- PDP returns empty constraints for resources without tenant property
- `require_constraints(true)` + empty constraints = `ConstraintsRequiredButAbsent` error
- Therefore type operations MUST use `require_constraints(false)`

**Resolution**: Add a note in DESIGN.md or Feature 0005 doc explaining this contract explicitly.

## 3. Dependency Matrix

### 3.1 Tables required by module

| Table | Owner | Used by | Required for |
|-------|-------|---------|-------------|
| `resource_group` | RG | RG, SecureORM | Group CRUD, tenant scoping |
| `resource_group_closure` | RG | RG, `InGroupSubtree` SQL | Hierarchy, group subtree queries |
| `resource_group_membership` | RG | RG, `InGroup`/`InGroupSubtree` SQL | Membership CRUD, group-based access |
| `resource_group_type` | RG | RG | Type CRUD |
| `tenant_closure` | tenant-resolver | `InTenantSubtree` SQL | Tenant subtree queries (NOT YET IN RG) |

### 3.2 SDK trait dependencies

| Trait | Provider | Consumer | Registration |
|-------|----------|----------|-------------|
| `ResourceGroupClient` | `RgService` | Any module via ClientHub | `module.rs` init phase |
| `ResourceGroupReadHierarchy` | `RgService` | `static-authz-plugin` | `module.rs` init phase |
| `AuthZResolverClient` | `authz-resolver` module | RG's PolicyEnforcer | Resolved from ClientHub |

### 3.3 Feature dependency chain

```
Feature 1 (Domain Foundation)
  └→ Feature 2 (Type Management)
  └→ Feature 3 (Entity & Hierarchy)
       └→ Feature 4 (Membership)
            └→ Feature 5 (AuthZ Enforcement) ← DONE
                 ├→ Feature 6 (MTLS/Plugin Gateway) ← DEFERRED
                 └→ Feature 7 (Advanced Constraint Types) ← SDK/compiler DONE, plugin/projection NOT DONE
```

## 4. Integration Verification Plan

### Phase 1: Current State Verification (DONE — 81 tests)

```bash
# All RG + plugin tests
cargo test -p cf-resource-group -p cf-static-authz-plugin --lib
```

Coverage:
- PolicyEnforcer deny → 403 for all 14 endpoints (groups, memberships, types)
- Mock enforcer → full CRUD flow with AccessScope
- Cross-tenant isolation (tenant A cannot see tenant B's data)
- Membership scoping via group subquery
- EnforcerError mapping (Denied/CompileFailed/EvaluationFailed)
- ReadHierarchy bypass (no enforcer loop)
- Cross-module: static-authz-plugin → ResourceGroupReadHierarchy → real SQLite DB

### Phase 2: Enable Capabilities (next step)

Prerequisites:
1. Resolve C1 (barrier_mode naming)
2. Resolve C4 (tenant_closure table — at least stub migration)

Actions:
1. Add `with_capabilities(vec![TenantHierarchy, GroupHierarchy])` in `module.rs`
2. Add integration test: RG with real enforcer → plugin returns `InTenantSubtree` → SecureORM generates correct SQL
3. Add test: `barrier_mode: Respect` vs `Ignore` in predicate → SQL `AND barrier = 0` present/absent

New tests needed:
| Test | What it verifies |
|------|-----------------|
| `enforcer_with_capabilities_receives_in_tenant_subtree` | Full path: RG → enforcer → plugin → `InTenantSubtree` → compiler → `ScopeFilter` → SQL |
| `barrier_mode_respect_adds_barrier_clause` | `PredicateBarrierMode::Respect` → `AND barrier = 0` in generated SQL |
| `barrier_mode_ignore_omits_barrier_clause` | `PredicateBarrierMode::Ignore` → no barrier clause |
| `tenant_closure_subquery_integration` | `InTenantSubtree` → `SELECT descendant_id FROM tenant_closure WHERE ancestor_id = ?` |

### Phase 3: Plugin InGroupSubtree (Feature 0007 completion)

Actions:
1. Extend static-authz-plugin to return `InGroupSubtree` when `GroupHierarchy` capability + `group_id` present
2. Add `resource_group_closure` awareness in `InGroupSubtree` SQL generation
3. Integration test: group subtree access through full pipeline

New tests needed:
| Test | What it verifies |
|------|-----------------|
| `plugin_returns_in_group_subtree_with_group_hierarchy_capability` | Plugin generates `InGroupSubtree` predicate |
| `in_group_subtree_sql_uses_closure_and_membership` | SQL: `resource_id IN (SELECT resource_id FROM membership WHERE group_id IN (SELECT descendant_id FROM closure WHERE ancestor_id = ?))` |
| `in_group_flat_vs_subtree` | `InGroup` returns direct members, `InGroupSubtree` returns subtree members |

### Phase 4: MTLS Auth (Feature 0006 — DEFERRED)

Blocked on:
- Platform MTLS CA bundle infrastructure
- AuthN module — MTLS certificate → SecurityContext
- Plugin out-of-process deployment mode

## 5. Resolution Tracker

| # | Contradiction | Resolution | Owner | Status |
|---|--------------|------------|-------|--------|
| C1 | `barrier_mode` naming | Added `serde(alias = "all"/"none")` + updated all arch docs | authz-resolver-sdk + arch docs | DONE |
| C2 | No PEP capabilities in RG | Added `with_capabilities(vec![GroupHierarchy])` in `module.rs`. `TenantHierarchy` deferred until C4 resolved. | cf-resource-group | PARTIAL |
| C3 | `resource_id` String vs UUID | Updated DESIGN.md: `resource_id` is TEXT (polymorphic external identifier) | arch docs | DONE |
| C4 | `tenant_closure` absent | Add local projection migration + CDC | cf-resource-group | TODO |
| C5 | No `InGroupSubtree` from plugin | Plugin enhancement | cf-static-authz-plugin | PLANNED (Feature 0007) |
| C6 | `require_constraints` undocumented | Added "Design Notes" section to Feature 0005 doc | Feature 0005 doc | DONE |

## 6. Runtime Verification Checklist

When deploying the full AuthZ integration, verify:

- [ ] `PolicyEnforcer` resolves `AuthZResolverClient` from ClientHub without error
- [ ] `ResourceGroupReadHierarchy` registered before `static-authz-plugin` init
- [ ] Module init order: `authz-resolver` → `resource-group` (declared via `deps = ["authz-resolver"]`)
- [ ] Type endpoints return data without tenant constraints (global resources)
- [ ] Group/membership endpoints filter by caller's tenant (check `X-Tenant-Id` header propagation)
- [ ] Cross-tenant GET returns 404 (not 403 — information hiding)
- [ ] Cross-tenant mutation rejected before DB write
- [ ] `tenant_closure` table exists and populated (if `TenantHierarchy` capability enabled)
- [ ] `resource_group_closure` table has correct depth entries (if `GroupHierarchy` capability enabled)
- [ ] Static plugin `evaluate()` latency < 10ms for typical request (no DB round-trip unless `GroupHierarchy`)
- [ ] No enforcer loop: `ReadHierarchy` calls bypass PolicyEnforcer
