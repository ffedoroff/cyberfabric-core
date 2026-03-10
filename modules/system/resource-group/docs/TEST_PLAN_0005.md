# Test Plan: Feature 0005 — AuthZ Enforcement

**Feature**: `cpt-cf-resource-group-feature-authz-enforcement`
**Module**: `cf-resource-group` + `cf-static-authz-plugin`
**Status**: IMPLEMENTED

## 1. Test Matrix

### 1.1 Unit Tests — resource-group (`service_test.rs`)

| # | Test Name | AC Coverage | Status |
|---|-----------|-------------|--------|
| 1 | `authz_denying_enforcer_blocks_create_group` | Group endpoints reject unauthorized | PASS |
| 2 | `authz_denying_enforcer_blocks_list_groups` | Group endpoints reject unauthorized | PASS |
| 3 | `authz_denying_enforcer_blocks_get_group` | Group endpoints reject unauthorized | PASS |
| 4 | `authz_denying_enforcer_blocks_update_group` | Group endpoints reject unauthorized | PASS |
| 5 | `authz_denying_enforcer_blocks_delete_group` | Group endpoints reject unauthorized | PASS |
| 6 | `authz_denying_enforcer_blocks_list_group_depth` | Group endpoints reject unauthorized | PASS |
| 7 | `authz_denying_enforcer_blocks_add_membership` | Membership endpoints reject unauthorized | PASS |
| 8 | `authz_denying_enforcer_blocks_remove_membership` | Membership endpoints reject unauthorized | PASS |
| 9 | `authz_denying_enforcer_blocks_list_memberships` | Membership endpoints reject unauthorized | PASS |
| 10 | `authz_denying_enforcer_blocks_create_type` | Type endpoints reject unauthorized (403) | PASS |
| 11 | `authz_denying_enforcer_blocks_get_type` | Type endpoints reject unauthorized (403) | PASS |
| 12 | `authz_denying_enforcer_blocks_update_type` | Type endpoints reject unauthorized (403) | PASS |
| 13 | `authz_denying_enforcer_blocks_list_types` | Type endpoints reject unauthorized (403) | PASS |
| 14 | `authz_denying_enforcer_blocks_delete_type` | Type endpoints reject unauthorized (403) | PASS |
| 15 | `authz_mock_enforcer_allows_full_flow` | Group CRUD via SDK with enforcer | PASS |
| 16 | `authz_mock_enforcer_type_crud_flow` | Type CRUD via SDK — `require_constraints(false)` | PASS |
| 17 | `authz_mock_enforcer_membership_flow` | Membership CRUD via SDK with enforcer | PASS |
| 18 | `authz_cross_tenant_isolation_via_sdk` | Cross-tenant isolation (tenant A sees only A's groups) | PASS |
| 19 | `authz_enforcer_error_mapping_compile_failed` | CompileFailed → DomainError::Database (500) | PASS |
| 20 | `authz_enforcer_error_mapping_evaluation_failed` | EvaluationFailed → DomainError::Database (503) | PASS |
| 21 | `authz_enforcer_error_mapping_denied` | Denied → DomainError::Forbidden (403) | PASS |
| 22 | `authz_read_hierarchy_bypasses_enforcer` | ReadHierarchy bypasses PolicyEnforcer (system access) | PASS |
| 23 | `authz_scoped_get_group_outside_scope_returns_not_found` | GET group outside tenant scope → 404 | PASS |
| 24 | `authz_scoped_create_group_outside_scope_rejected` | POST group with wrong tenant_id → rejected | PASS |
| 25 | `authz_scoped_delete_group_outside_scope_returns_not_found` | DELETE group outside tenant scope → 404 | PASS |
| 26 | `authz_scoped_list_memberships_returns_only_tenant_memberships` | GET memberships scoped to tenant's groups | PASS |
| 27 | `authz_scoped_add_membership_to_outside_scope_group_rejected` | POST membership to out-of-scope group → 404 | PASS |

### 1.2 Unit Tests — static-authz-plugin (`service.rs`, `client.rs`)

| # | Test Name | AC Coverage | Status |
|---|-----------|-------------|--------|
| 1 | `plugin_trait_evaluates_successfully` | Plugin client trait works | PASS |
| 2 | `list_operation_with_tenant_context` | Tenant context → OWNER_TENANT_ID constraint | PASS |
| 3 | `list_operation_without_tenant_falls_back_to_subject_properties` | Subject property fallback | PASS |
| 4 | `missing_tenant_context_and_subject_property_is_denied` | Missing tenant → deny | PASS |
| 5 | `nil_tenant_is_denied` | Nil tenant → deny | PASS |
| 6 | `group_hierarchy_allows_when_group_belongs_to_tenant` | Hierarchy check passes | PASS |
| 7 | `group_hierarchy_denies_when_group_belongs_to_other_tenant` | Cross-tenant hierarchy → deny | PASS |
| 8 | `group_hierarchy_fallback_on_error` | RG error → graceful degradation | PASS |
| 9 | `without_group_hierarchy_capability_skips_hierarchy_check` | No capability → skip check | PASS |
| 10 | `group_hierarchy_with_no_hierarchy_client_falls_back` | No hierarchy client → tenant-only | PASS |

## 2. Key Design Decisions Tested

### 2.1 Type operations use `require_constraints(false)`
Types are global resources without `OWNER_TENANT_ID`. The PDP returns empty constraints for them. Using `access_scope_with(&AccessRequest::new().require_constraints(false))` prevents `ConstraintsRequiredButAbsent` error.

### 2.2 ReadHierarchy bypasses PolicyEnforcer
`ResourceGroupReadHierarchy::list_group_depth` uses `AccessScope::allow_all()` to avoid infinite loop: Plugin.evaluate → RG.list_group_depth → PolicyEnforcer → AuthZ → Plugin.evaluate.

### 2.3 EnforcerError mapping
- `Denied` → `DomainError::Forbidden` → `403 Forbidden`
- `CompileFailed` → `DomainError::Database` → `500 Internal Server Error`
- `EvaluationFailed` → `DomainError::Database` → `503 Service Unavailable` (mapped at REST layer)

### 2.4 MockAuthZResolver property awareness
Mock checks `supported_properties` to decide whether to return `OWNER_TENANT_ID` constraints. This mimics real PDP behavior.

## 3. Acceptance Criteria Coverage

| AC from Feature 0005 | Test(s) |
|---|---|
| PolicyEnforcer instantiated in module init | Build helpers use `PolicyEnforcer::new(authz)` |
| All group endpoints call `access_scope()` | Tests 1-6, 15 |
| All membership endpoints call `access_scope()` | Tests 7-9, 17 |
| All type endpoints call `access_scope()` | Tests 10-14, 16 |
| GET /groups returns only tenant's groups | Test 18 (`authz_cross_tenant_isolation_via_sdk`) |
| GET /groups/{id} outside scope → 404 | Test 23 (`authz_scoped_get_group_outside_scope_returns_not_found`) |
| POST /groups with wrong tenant_id → rejected | Test 24 (`authz_scoped_create_group_outside_scope_rejected`) |
| DELETE /groups/{id} outside scope → 404 | Test 25 (`authz_scoped_delete_group_outside_scope_returns_not_found`) |
| GET /memberships scoped to tenant's groups | Test 26 (`authz_scoped_list_memberships_returns_only_tenant_memberships`) |
| POST /memberships to out-of-scope group → 404 | Test 27 (`authz_scoped_add_membership_to_outside_scope_group_rejected`) |
| Type endpoints reject unauthorized with 403 | Tests 10-14 |
| AuthZ denial → 403 | Test 21 |
| AuthZ service unavailable → 500/503 | Test 20 |
| Constraint compilation failure → 500 | Test 19 |
| AuthZ plugin can call ReadHierarchy w/o enforcer | Test 22 |
| SecurityContext forwarded to PolicyEnforcer | Implicit in all mock enforcer tests |
| RG does not interpret policy | Implicit — only applies AccessScope |

## 4. Running Tests

```bash
# Resource-group module (71 tests, including 5 cross-module + 5 E2E scope enforcement)
cargo test -p cf-resource-group --lib

# Static AuthZ plugin (10 tests)
cargo test -p cf-static-authz-plugin --lib

# Both modules (81 tests total)
cargo test -p cf-resource-group -p cf-static-authz-plugin --lib
```

### 1.3 Cross-Module Tests — static-authz-plugin + resource-group (`service_test.rs::cross_module_authz_rg`)

| # | Test Name | AC Coverage | Status |
|---|-----------|-------------|--------|
| 1 | `authz_plugin_allows_group_in_correct_tenant` | Full path: plugin→RG hierarchy→real DB, group in correct tenant | PASS |
| 2 | `authz_plugin_denies_group_in_wrong_tenant` | Full path: cross-tenant group → deny | PASS |
| 3 | `authz_plugin_nonexistent_group_behavior` | Non-existent group → plugin behavior (deny or fallback) | PASS |
| 4 | `authz_plugin_hierarchy_with_child_groups` | Multi-level hierarchy, parent+child same tenant | PASS |
| 5 | `authz_plugin_no_hierarchy_capability_skips_rg_call` | No GroupHierarchy capability → skip hierarchy check | PASS |

These tests wire `static_authz_plugin::domain::Service` directly to `RgService` via `ResourceGroupReadHierarchy` trait with a real SQLite database. No mocks — tests the full data path.

## 5. Not Yet Covered (Future Work)

- **Integration with running service**: REST endpoint E2E tests (curl against running instance) — requires full platform bootstrap
- **MTLS authentication path**: Deferred to Feature 0006
- **Plugin gateway routing**: Deferred to Feature 0006
- **Advanced constraint types** (`in_tenant_subtree`, `in_group`, `in_group_subtree`): Feature 0007
- **Load/stress testing**: Not in scope for unit test plan
