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
| MTLS auth path | DEFERRED | 0006 | Blocked on platform MTLS infra |
| Plugin gateway routing | DEFERRED | 0006 | Blocked on vendor plugin architecture |

## 2. Contradictions: Architecture Docs vs Code

### C2: RG module PEP capabilities — DONE

**Severity**: MEDIUM — advanced predicates never reach RG in production

**Problem**: `module.rs` created `PolicyEnforcer::new(authz)` without `with_capabilities()` → `capabilities: vec![]` in every request → static-authz-plugin falls back to flat `In` predicates.

**Resolution**: Added both capabilities in `module.rs`:
```rust
let enforcer = PolicyEnforcer::new(authz)
    .with_capabilities(vec![Capability::TenantHierarchy, Capability::GroupHierarchy]);
```

**Prerequisite met**: `resource_group_closure` table already exists with tenant hierarchy data (tenants are resource groups with `group_type='tenant'`).

### C3: `resource_id` type — String vs UUID — DONE

**Severity**: LOW

**Resolution**: DESIGN.md updated — `resource_id` is TEXT (polymorphic external identifier), not UUID.

### C4: `InTenantSubtree` uses `resource_group_closure` — DONE

**Resolution**: `InTenantSubtree` SQL uses `resource_group_closure` with a JOIN on `resource_group.group_type = 'tenant'`. No separate projection table needed — tenants ARE resource groups.

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

**Tests added**: `membership_list_with_in_tenant_subtree`.

### C8: Test data seeding — DONE

**Resolution**: Tests create actual resource group hierarchies using domain services. Closure table is auto-populated by hierarchy management code. No raw SQL seeding needed.

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

### C12: `resource_group_closure.depth` column missing from RESOURCE_GROUP_MODEL.md — DONE

**Severity**: MEDIUM — schema documentation gap

**Problem**: RESOURCE_GROUP_MODEL.md schema for `resource_group_closure` showed only `ancestor_id`, `descendant_id` but the actual migration includes `depth INTEGER NOT NULL`.

**Resolution**: Added `depth` column to documentation with semantics note (0=self, 1=direct, 2+=deeper).

### C13: Removed — N/A (no separate projection table)

## 3. Dependency Matrix

### 3.1 Tables required by module

| Table | Owner | Local Projection | Used by | Status |
|-------|-------|-----------------|---------|--------|
| `resource_group` | RG | — | RG, SecureORM | DONE |
| `resource_group_closure` | RG | — | RG, `InGroupSubtree` SQL | DONE |
| `resource_group_membership` | RG | — | RG, `InGroup`/`InGroupSubtree` SQL | DONE |
| `resource_group_type` | RG | — | RG | DONE |

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

### Phase 2: Advanced Constraints (DONE — 4 tests)

Coverage:
- `InTenantSubtree` group listing via `resource_group_closure` + `group_type='tenant'` JOIN
- Descendant tenant groups visible through subtree scope
- Leaf tenant sees only own groups
- Membership listing with `InTenantSubtree` — correct subtree visibility

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
| C2 | No PEP capabilities in RG | `with_capabilities(vec![TenantHierarchy, GroupHierarchy])` | cf-resource-group | DONE |
| C3 | `resource_id` String vs UUID | DESIGN.md updated: TEXT | arch docs | DONE |
| C4 | `InTenantSubtree` table | Uses `resource_group_closure` + `group_type='tenant'` JOIN | cf-resource-group | DONE |
| C5 | No `InGroupSubtree` from plugin | Compound constraint in `evaluate_with_hierarchy` | cf-static-authz-plugin | DONE |
| C6 | `require_constraints` undocumented | Design Notes in Feature 0005 | Feature 0005 doc | DONE |
| C7 | Membership scoping broken with advanced filters | `build_scope_condition` replaces manual UUID extraction | cf-resource-group | DONE |
| C8 | Test data seeding | Tests use domain services, auto-populated closure | cf-resource-group tests | DONE |
| C9 | `EvaluationFailed` → 500 instead of 503 | Added `DomainError::ServiceUnavailable` → 503 | cf-resource-group | DONE |
| C10 | Predicate JSON `"resource_property"` vs `"property"` | Updated 57 occurrences in DESIGN.md + AUTHZ_USAGE_SCENARIOS.md | arch docs | DONE |
| C12 | `resource_group_closure.depth` missing from docs | Added to RESOURCE_GROUP_MODEL.md | arch docs | DONE |

## 6. Runtime Verification Checklist

When deploying the full AuthZ integration, verify:

- [x] `PolicyEnforcer` resolves `AuthZResolverClient` from ClientHub without error
- [x] `ResourceGroupReadHierarchy` registered before `static-authz-plugin` init
- [x] Module init order: `authz-resolver` → `resource-group` (declared via `deps = ["authz-resolver"]`)
- [x] Type endpoints return data without tenant constraints (global resources)
- [x] Group/membership endpoints filter by caller's tenant
- [x] Cross-tenant GET returns 404 (not 403 — information hiding)
- [x] Cross-tenant mutation rejected before DB write
- [x] `resource_group_closure` table has correct depth entries
- [x] Static plugin `evaluate()` latency < 10ms (no DB round-trip unless `GroupHierarchy`)
- [x] No enforcer loop: `ReadHierarchy` calls bypass PolicyEnforcer
- [x] Membership listing works with `InTenantSubtree` scope (C7 fix)

## 7. Test Summary

```bash
# Full test suite (85 tests)
cargo test -p cf-resource-group --lib

# Advanced constraint tests only (4 tests)
cargo test -p cf-resource-group tenant_subtree_integration

# Cross-module tests (5 tests)
cargo test -p cf-resource-group cross_module_authz_rg

# Static authz plugin tests
cargo test -p cf-static-authz-plugin --lib
```

## 8. Related Documentation

| Document | Path | Content |
|----------|------|---------|
| Feature 0005: AuthZ Enforcement | `features/0005-cpt-cf-resource-group-feature-authz-enforcement.md` | PolicyEnforcer integration, AccessScope, error mapping |
| Feature 0006: MTLS & Plugin Gateway | `features/0006-cpt-cf-resource-group-feature-read-authz.md` | MTLS auth, plugin routing (DEFERRED) |
| Feature 0007: Advanced Constraints | `features/0007-cpt-cf-resource-group-feature-authz-constraint-types.md` | InTenantSubtree, InGroup, InGroupSubtree + design notes on resource_type semantics |
| ADR-0001: Graceful Degradation | `ADR/0001-graceful-degradation-authz-plugin-hierarchy.md` | Plugin fallback when group hierarchy unavailable |
| DECOMPOSITION | `DECOMPOSITION.md` | Feature dependency graph (7 features) |
