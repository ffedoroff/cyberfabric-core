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
| Membership subquery scoping | DONE | 0005+0007 | Uses `build_scope_condition` — handles Eq, In, InTenantSubtree, InGroup, InGroupSubtree |
| ResourceGroupReadHierarchy bypass | DONE | 0005 | `AccessScope::allow_all()` — no enforcer loop |
| SDK constraint types | DONE | 0007 | `Eq`, `In`, `InTenantSubtree`, `InGroup`, `InGroupSubtree` |
| Constraint compiler | DONE | 0007 | All 5 predicate types → `ScopeFilter` variants |
| SecureORM SQL generation | DONE | 0007 | Subquery generation for all advanced predicates |
| Static plugin → `InTenantSubtree` | DONE | 0007 | Returns when `TenantHierarchy` capability declared |
| Static plugin → `InGroupSubtree` | DONE | 0007 | Returns compound constraint: tenant + `InGroupSubtree` when `GroupHierarchy` + group_id validated |
| PEP capabilities in RG module | DONE | 0007 | `TenantHierarchy` + `GroupHierarchy` declared in `module.rs` |
| `tenant_closure` local projection | DONE | 0007 | Migration `m20260310_000002_tenant_closure_projection` (CDC from tenant-resolver) |
| MTLS auth path | DEFERRED | 0006 | Blocked on platform MTLS infra |
| Plugin gateway routing | DEFERRED | 0006 | Blocked on vendor plugin architecture |

## 2. Contradictions: Architecture Docs vs Code

### C1: `barrier_mode` value naming — DONE

**Severity**: HIGH — JSON contract mismatch between docs and wire format

| Source | Values | Location |
|--------|--------|----------|
| `docs/arch/authorization/DESIGN.md` | `"all"` / `"none"` | Lines 807, 840, 881, 1114, 1138-1139 |
| `docs/arch/authorization/TENANT_MODEL.md` | `"all"` / `"none"` | Line 190 |
| `docs/arch/authorization/AUTHZ_USAGE_SCENARIOS.md` | `"all"` / `"none"` | Lines 200, 605 |
| `authz-resolver-sdk/src/models.rs` `BarrierMode` | `"respect"` / `"ignore"` | `serde(rename_all = "snake_case")` |
| `authz-resolver-sdk/src/constraints.rs` `PredicateBarrierMode` | `"respect"` / `"ignore"` | `serde(rename_all = "snake_case")` |

Both `BarrierMode` (request-level) and `PredicateBarrierMode` (predicate-level) use `Respect`/`Ignore` with `serde(rename_all = "snake_case")`, serializing to `"respect"`/`"ignore"`. Architecture docs consistently use `"all"`/`"none"`.

**Resolution**: Applied option (A) — `serde(alias = "all"/"none")` on both enums. Docs updated to canonical `"respect"`/`"ignore"` with legacy alias note.

### C2: RG module PEP capabilities — DONE

**Severity**: MEDIUM — advanced predicates never reach RG in production

**Problem**: `module.rs` created `PolicyEnforcer::new(authz)` without `with_capabilities()` → `capabilities: vec![]` in every request → static-authz-plugin falls back to flat `In` predicates.

**Resolution**: Added both capabilities in `module.rs`:
```rust
let enforcer = PolicyEnforcer::new(authz)
    .with_capabilities(vec![Capability::TenantHierarchy, Capability::GroupHierarchy]);
```

**Prerequisite met**: `tenant_closure` local projection table created (see C4).

### C3: `resource_id` type — String vs UUID — DONE

**Severity**: LOW

**Resolution**: DESIGN.md updated — `resource_id` is TEXT (polymorphic external identifier), not UUID.

### C4: `tenant_closure` local projection — DONE

**Severity**: MEDIUM — `InTenantSubtree` SQL references non-existent table

**Problem**: Feature 0007 describes SQL `SELECT descendant_id FROM tenant_closure WHERE ancestor_id = ?` but table didn't exist in RG migrations.

**Resolution**: Applied option (A) — local projection migration `m20260310_000002_tenant_closure_projection.rs`:
- PostgreSQL: `ancestor_id UUID`, `descendant_id UUID`, `barrier INT`, `descendant_status TEXT`
- SQLite: `ancestor_id TEXT`, `descendant_id TEXT`, `barrier INT`, `descendant_status TEXT`
- Indexes: `idx_tc_descendant_id`, `idx_tc_ancestor_barrier`
- Populated by CDC from tenant-resolver module.

### C5: Static plugin `InGroupSubtree` — DONE

**Resolution**: `evaluate_with_hierarchy` returns compound constraint: `InTenantSubtree`/`In` + `InGroupSubtree`.

### C6: `require_constraints` semantics documentation gap — DONE

**Resolution**: Added Design Notes section to Feature 0005 doc.

### C7: Membership scoping incompatible with advanced scope filters — DONE

**Severity**: CRITICAL — membership listing returns empty for InTenantSubtree scopes

**Problem**: `membership_repo.rs:list_filtered()` used `scope.all_uuid_values_for(OWNER_TENANT_ID)` to extract flat UUID values for a manual subquery. `ScopeFilter::InTenantSubtree` (and `InGroup`, `InGroupSubtree`) return empty from `values()` because their values are determined at SQL execution time, not extraction time.

**Result**: After enabling `TenantHierarchy` capability (C2), the static-authz-plugin returns `InTenantSubtree` predicate → compiled to `ScopeFilter::InTenantSubtree` → `all_uuid_values_for()` returns empty Vec → membership list returns 0 results (false deny-all).

**Resolution**: Replaced manual UUID extraction with `build_scope_condition::<resource_group::Entity>(scope)` from modkit-db. This delegates scope → SQL conversion to SecureORM, which handles all filter types correctly:

```rust
// Before (broken with InTenantSubtree):
let tenant_ids = scope.all_uuid_values_for(pep_properties::OWNER_TENANT_ID);
if tenant_ids.is_empty() { return Ok(vec![]); }
sub.and_where(resource_group::Column::TenantId.is_in(tenant_ids));

// After (handles all scope filter types):
let scope_cond = build_scope_condition::<resource_group::Entity>(scope);
sub.cond_where(scope_cond);
```

Required re-exporting `build_scope_condition` from `modkit_db::secure`.

**Tests added**: `membership_list_with_in_tenant_subtree`, `membership_list_respects_barriers`.

### C8: SQLite UUID storage format mismatch in tests

**Severity**: LOW — test-only issue, no production impact

**Problem**: `sqlx` encodes `uuid::Uuid` as 16-byte BLOB for SQLite, but raw SQL `INSERT` stores UUIDs as TEXT strings. SQLite's strict type comparison means BLOB ≠ TEXT, so `InTenantSubtree` subqueries on `tenant_closure` (seeded via raw SQL) fail to match `resource_group.tenant_id` (stored via SeaORM as BLOB).

**Resolution**: `seed_tenant_closure` in tests uses BLOB hex literals `X'{uuid.simple()}'` instead of text strings `'{uuid}'` to match SeaORM's storage format.

**Lesson**: Any test that seeds projection tables via raw SQL for SQLite must use BLOB hex format for UUID columns: `X'{uuid.simple()}'`.

### C9: `EvaluationFailed` mapped to 500 instead of 503 — DONE

**Severity**: MEDIUM — wrong HTTP status code for downstream service failure

**Problem**: Feature 0005 specifies `EnforcerError::EvaluationFailed → 503 Service Unavailable`. Code mapped it to `DomainError::Database → 500 Internal Server Error`.

**Resolution**: Added `DomainError::ServiceUnavailable` variant. Error chain:
- `EnforcerError::EvaluationFailed` → `DomainError::ServiceUnavailable`
- `DomainError::ServiceUnavailable` → `ResourceGroupError::ServiceUnavailable`
- `DomainError::ServiceUnavailable` → `Problem` with `StatusCode::SERVICE_UNAVAILABLE` (503)

### C10: Predicate JSON field `"resource_property"` vs `"property"` — DONE

**Severity**: HIGH — JSON wire format mismatch between docs and code

**Problem**: Architecture docs (DESIGN.md, AUTHZ_USAGE_SCENARIOS.md) used `"resource_property"` as the JSON key in predicate examples. Actual code serialization (via serde) uses `"property"` — no rename attribute exists. All existing tests and consumers use `"property"`.

**Resolution**: Updated all 57 occurrences of `"resource_property"` → `"property"` in DESIGN.md and AUTHZ_USAGE_SCENARIOS.md JSON examples.

### C11: Stale comment in `models.rs` barrier_mode default — DONE

**Severity**: LOW — misleading doc comment

**Problem**: `TenantContext.barrier_mode` field comment said "default: `All`" but actual default is `Respect`.

**Resolution**: Updated comment to reference `Respect`.

### C12: `resource_group_closure.depth` column missing from RESOURCE_GROUP_MODEL.md — DONE

**Severity**: MEDIUM — schema documentation gap

**Problem**: RESOURCE_GROUP_MODEL.md schema for `resource_group_closure` showed only `ancestor_id`, `descendant_id` but the actual migration includes `depth INTEGER NOT NULL`.

**Resolution**: Added `depth` column to documentation with semantics note (0=self, 1=direct, 2+=deeper).

### C13: `tenant_closure` missing `descendant_status` index — DONE

**Severity**: MEDIUM — performance issue with tenant_status filtering

**Problem**: `InTenantSubtree` SQL filters by `descendant_status IN (?)` but no index covered this column. Only `(ancestor_id, barrier)` was indexed.

**Resolution**: Added index `idx_tc_ancestor_status ON tenant_closure (ancestor_id, descendant_status)` to both Postgres and SQLite migrations.

## 3. Dependency Matrix

### 3.1 Tables required by module

| Table | Owner | Local Projection | Used by | Status |
|-------|-------|-----------------|---------|--------|
| `resource_group` | RG | — | RG, SecureORM | DONE |
| `resource_group_closure` | RG | — | RG, `InGroupSubtree` SQL | DONE |
| `resource_group_membership` | RG | — | RG, `InGroup`/`InGroupSubtree` SQL | DONE |
| `resource_group_type` | RG | — | RG | DONE |
| `tenant_closure` | tenant-resolver | Yes (CDC) | `InTenantSubtree` SQL | DONE (migration), CDC pending |

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
                 └→ Feature 7 (Advanced Constraint Types) ← DONE
```

## 4. Integration Verification Plan

### Phase 1: Basic AuthZ (DONE — 81 tests)

Coverage:
- PolicyEnforcer deny → 403 for all 14 endpoints (groups, memberships, types)
- Mock enforcer → full CRUD flow with AccessScope
- Cross-tenant isolation (tenant A cannot see tenant B's data)
- Membership scoping via group subquery
- EnforcerError mapping (Denied/CompileFailed/EvaluationFailed)
- ReadHierarchy bypass (no enforcer loop)
- Cross-module: static-authz-plugin → ResourceGroupReadHierarchy → real SQLite DB

### Phase 2: Advanced Constraints (DONE — 6 tests)

Coverage:
- `InTenantSubtree` group listing with seeded `tenant_closure` data
- Barrier mode `Respect` — excludes groups behind barrier
- Barrier mode `Ignore` — includes all groups regardless of barrier
- Leaf tenant sees only own groups
- Membership listing with `InTenantSubtree` — correct subtree visibility
- Membership listing respects barriers — excluded from behind barrier

```bash
cargo test -p cf-resource-group tenant_subtree_integration
```

### Phase 3: Plugin InGroupSubtree (DONE — 5 cross-module tests)

Coverage:
- AuthZ plugin allows group in correct tenant
- AuthZ plugin denies group in wrong tenant
- AuthZ plugin handles nonexistent group
- AuthZ plugin with hierarchy + child groups
- AuthZ plugin without GroupHierarchy capability skips RG call

```bash
cargo test -p cf-resource-group cross_module_authz_rg
```

### Phase 4: MTLS Auth (Feature 0006 — DEFERRED)

Blocked on:
- Platform MTLS CA bundle infrastructure
- AuthN module — MTLS certificate → SecurityContext
- Plugin out-of-process deployment mode

## 5. Resolution Tracker

| # | Contradiction | Resolution | Owner | Status |
|---|--------------|------------|-------|--------|
| C1 | `barrier_mode` naming | `serde(alias = "all"/"none")` + docs updated | authz-resolver-sdk + arch docs | DONE |
| C2 | No PEP capabilities in RG | `with_capabilities(vec![TenantHierarchy, GroupHierarchy])` | cf-resource-group | DONE |
| C3 | `resource_id` String vs UUID | DESIGN.md updated: TEXT | arch docs | DONE |
| C4 | `tenant_closure` absent | Local projection migration created | cf-resource-group | DONE |
| C5 | No `InGroupSubtree` from plugin | Compound constraint in `evaluate_with_hierarchy` | cf-static-authz-plugin | DONE |
| C6 | `require_constraints` undocumented | Design Notes in Feature 0005 | Feature 0005 doc | DONE |
| C7 | Membership scoping broken with advanced filters | `build_scope_condition` replaces manual UUID extraction | cf-resource-group | DONE |
| C8 | SQLite UUID BLOB/TEXT mismatch in tests | BLOB hex literals in `seed_tenant_closure` | cf-resource-group tests | DONE |
| C9 | `EvaluationFailed` → 500 instead of 503 | Added `DomainError::ServiceUnavailable` → 503 | cf-resource-group | DONE |
| C10 | Predicate JSON `"resource_property"` vs `"property"` | Updated 57 occurrences in DESIGN.md + AUTHZ_USAGE_SCENARIOS.md | arch docs | DONE |
| C11 | Stale barrier_mode default comment ("All") | Updated to "Respect" in models.rs | authz-resolver-sdk | DONE |
| C12 | `resource_group_closure.depth` missing from docs | Added to RESOURCE_GROUP_MODEL.md | arch docs | DONE |
| C13 | `tenant_closure` missing `descendant_status` index | Added `idx_tc_ancestor_status` to migration | cf-resource-group | DONE |

## 6. Runtime Verification Checklist

When deploying the full AuthZ integration, verify:

- [x] `PolicyEnforcer` resolves `AuthZResolverClient` from ClientHub without error
- [x] `ResourceGroupReadHierarchy` registered before `static-authz-plugin` init
- [x] Module init order: `authz-resolver` → `resource-group` (declared via `deps = ["authz-resolver"]`)
- [x] Type endpoints return data without tenant constraints (global resources)
- [x] Group/membership endpoints filter by caller's tenant
- [x] Cross-tenant GET returns 404 (not 403 — information hiding)
- [x] Cross-tenant mutation rejected before DB write
- [x] `tenant_closure` table exists (migration) — CDC population pending
- [x] `resource_group_closure` table has correct depth entries
- [x] Static plugin `evaluate()` latency < 10ms (no DB round-trip unless `GroupHierarchy`)
- [x] No enforcer loop: `ReadHierarchy` calls bypass PolicyEnforcer
- [x] Membership listing works with `InTenantSubtree` scope (C7 fix)
- [ ] CDC pipeline: tenant-resolver → `tenant_closure` projection table (pending infra)

## 7. Test Summary

```bash
# Full test suite (87 tests)
cargo test -p cf-resource-group --lib

# Advanced constraint tests only (6 tests)
cargo test -p cf-resource-group tenant_subtree_integration

# Cross-module tests (5 tests)
cargo test -p cf-resource-group cross_module_authz_rg

# Static authz plugin tests
cargo test -p cf-static-authz-plugin --lib
```
