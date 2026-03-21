# AuthZ + Resource Group Integration Test Plan

How to verify the RG ↔ AuthZ interaction locally in hyperspot-server. Covers three phases: tenant scoping (implemented), group-based predicates (requires new predicate types), and MTLS bypass (requires cert infrastructure).

For background on how AuthZ uses RG data, see [RESOURCE_GROUP_MODEL.md](./RESOURCE_GROUP_MODEL.md). For concrete SQL-level scenarios, see [AUTHZ_USAGE_SCENARIOS.md](./AUTHZ_USAGE_SCENARIOS.md) scenarios S14–S21.

---

## Current State

| Component | Status | Notes |
|-----------|--------|-------|
| RG Module | Ready | ClientHub: `dyn ResourceGroupClient` + `dyn ResourceGroupReadHierarchy` |
| AuthZ Resolver | Ready | Plugin discovery, `PolicyEnforcer`, `AccessScope` → SecureORM |
| Static AuthZ Plugin | Ready | Returns `In(owner_tenant_id, [tid])` — tenant predicates only |
| **PolicyEnforcer in RG handlers** | **Done** | `GroupService` calls `enforcer.access_scope()` for list/get/hierarchy |
| **AccessScope → SecureORM in RG repo** | **Done** | `GroupRepository.list_groups`, `find_by_id`, `list_hierarchy` accept `&AccessScope` |
| **Rust integration tests** | **Done** | 19 tests: enforcer flow + tenant scoping + full-chain verification |
| **E2E HTTP tests** | **Done** | pytest: CRUD, hierarchy, membership, tenant isolation |
| Group predicates (`in_group`, `in_group_subtree`) | Not implemented | Static plugin does not call `ResourceGroupReadHierarchy` |

---

## File Layout

```
testing/e2e/modules/resource_group/        ← E2E tests (pytest, HTTP against running server)
  conftest.py                              ← Fixtures: base_url, auth headers, type/group factories
  test_authz_tenant_scoping.py             ← Phase 1: CRUD + tenant isolation + hierarchy + membership

modules/system/resource-group/
  resource-group/tests/                    ← Rust integration tests (in-process, no HTTP)
    authz_integration_test.rs              ← PolicyEnforcer + mock AuthZ: 9 tests
    tenant_scoping_test.rs                 ← AccessScope scoping: 10 tests
```

Follows existing project conventions: `testing/e2e/modules/{module}/` for HTTP-level tests (see `oagw/`, `mini_chat/`, `types_registry/`), `modules/.../tests/` for Rust in-process tests.

---

## Prerequisites

- Rust stable (MSRV 1.92.0)
- Docker (for PostgreSQL)
- `protoc` installed (`brew install protobuf` on macOS)

### Start PostgreSQL

```bash
docker run -d --name rg-postgres \
  -e POSTGRES_USER=hyperspot \
  -e POSTGRES_PASSWORD=hyperspot \
  -e POSTGRES_DB=resource_group \
  -p 5433:5432 postgres:16-alpine
```

### Server Configuration

In `config/quickstart.yaml`, the resource-group module requires PostgreSQL:

```yaml
modules:
  resource-group:
    database:
      dsn: "postgres://hyperspot:hyperspot@127.0.0.1:5433/resource_group"
      pool:
        max_conns: 5
        acquire_timeout: "30s"
    config: {}
```

### Build and Run

```bash
# Without AuthZ (dev mode, auth_disabled: true)
cargo run --bin hyperspot-server -- --config config/quickstart.yaml run

# With AuthZ (auth_disabled: false + static plugins)
cargo run --bin hyperspot-server \
  --features static-authn,static-authz \
  -- --config config/quickstart.yaml run
```

### Run Tests

```bash
# Rust integration tests (no server/DB required)
cargo test -p cf-resource-group --test authz_integration_test --test tenant_scoping_test

# E2E tests (requires running server + PostgreSQL)
E2E_BASE_URL=http://localhost:8087 pytest testing/e2e/modules/resource_group/ -v
```

---

## Phase 1: Tenant Scoping via PolicyEnforcer ✅ IMPLEMENTED

**Goal**: Verify that RG endpoints apply `AccessScope` from AuthZ pipeline, filtering results by `tenant_id` from `SecurityContext`.

### Implementation summary

The full AuthZ → RG chain is now wired:

1. **Module init** (`module.rs`): resolves `dyn AuthZResolverClient` from ClientHub, creates `PolicyEnforcer`
2. **GroupService** (`group_service.rs`): receives `PolicyEnforcer`; `list_groups`, `get_group`, `list_group_hierarchy` call `enforcer.access_scope(&ctx, &RG_GROUP_RESOURCE, action, resource_id)`
3. **GroupRepository** (`group_repo.rs`): `list_groups`, `find_by_id`, `list_hierarchy` accept `&AccessScope` and pass it to `SecureORM` via `.secure().scope_with(scope)`
4. **Handlers** (`handlers/groups.rs`): pass `&ctx` to service methods (no longer `_ctx`)
5. **Error handling** (`error.rs`): `DomainError::AccessDenied` → HTTP 403

### AuthZ flow (implemented)

```
Request → API Gateway (AuthN) → SecurityContext{tenant=T1}
  → RG Handler(list_groups) → GroupService.list_groups(&ctx, &query)
    → PolicyEnforcer.access_scope(&ctx, RG_GROUP_RESOURCE, "list", None)
      → Static AuthZ Plugin → decision=true, constraints=[In(owner_tenant_id, [T1])]
    → AccessScope{owner_tenant_id IN (T1)}
    → GroupRepository.list_groups(&conn, &scope, &query)
      → SecureORM → WHERE tenant_id IN ('T1')
  → Response: groups from T1 only
```

### Rust integration tests (19 tests)

**`authz_integration_test.rs`** (9 tests):
- `enforcer_tenant_scoping_produces_correct_access_scope` — mock PDP → correct scope
- `enforcer_different_tenants_get_different_scopes` — tenant isolation at scope level
- `enforcer_deny_all_returns_denied_error` — deny flow
- `enforcer_allow_all_no_constraints_returns_allow_all` — unconstrained path
- `enforcer_allow_all_with_required_constraints_fails` — fail-closed
- `enforcer_passes_resource_id_to_pdp` — request params verification
- `enforcer_works_for_all_crud_actions` — all 5 CRUD actions
- `full_chain_list_groups_calls_enforcer_with_correct_params` — **full chain**: capturing mock verifies PDP receives `RG_GROUP_RESOURCE`, `"list"`, correct tenant_id; scope filters by tenant
- `full_chain_deny_all_blocks_list_groups` — **full chain deny**: deny-all PDP blocks operation

**`tenant_scoping_test.rs`** (10 tests):
- AccessScope construction, isolation, `tenant_only()`, `deny_all()`, `for_resource()`

### E2E HTTP tests (9 tests)

**`test_authz_tenant_scoping.py`**:
- `test_create_and_get_type` — type CRUD
- `test_create_and_get_group` — group with tenant_id from SecurityContext
- `test_list_groups_returns_created_groups` — list returns own groups
- `test_group_has_tenant_id_from_security_context` — consistent tenant_id across groups
- `test_child_group_inherits_parent_tenant` — parent-child tenant enforcement
- `test_group_hierarchy_returns_parent_and_children` — hierarchy traversal
- `test_delete_group` — delete + 404 verification
- `test_membership_add_and_list` — membership CRUD
- `test_tenant_isolation_same_token_sees_own_groups` — same-tenant visibility

---

## Phase 2: Group-Based Predicates

**Goal**: Verify the full S14/S15 scenario — AuthZ plugin queries RG hierarchy, returns `in_group`/`in_group_subtree` predicates, PEP compiles them into SQL JOINs.

### What needs to be implemented

#### 2.1 New predicate types in AuthZ SDK

`authz-resolver-sdk/src/constraints.rs` — add:

```rust
pub enum Predicate {
    Eq(EqPredicate),
    In(InPredicate),
    InGroup(InGroupPredicate),           // NEW
    InGroupSubtree(InGroupSubtreePredicate), // NEW
}

pub struct InGroupPredicate {
    pub resource_property: String,  // e.g., "id"
    pub group_ids: Vec<Uuid>,
}

pub struct InGroupSubtreePredicate {
    pub resource_property: String,
    pub root_group_id: Uuid,
}
```

#### 2.2 Constraint compiler

`authz-resolver-sdk/src/pep/compiler.rs` — compile new predicates into `AccessScope`:

- `InGroup` → `ScopeFilter::InSubquery { property, table: "resource_group_membership", join_column: "resource_id", filter_column: "group_id", values }`
- `InGroupSubtree` → nested subquery: `resource_group_membership` JOIN `resource_group_closure`

#### 2.3 RG-aware AuthZ plugin

Either extend `static-authz-plugin` or create a new plugin that:

1. Checks `capabilities` in `EvaluationRequestContext` for `group_membership` or `group_hierarchy`
2. Resolves `dyn ResourceGroupReadHierarchy` from ClientHub
3. Calls `list_group_depth()` to get hierarchy data
4. Produces `InGroup`/`InGroupSubtree` predicates

#### 2.4 Projection tables in domain DB

A test domain service needs `resource_group_membership` and/or `resource_group_closure` as local projection tables for SQL JOINs. For dev/test, this can be the same PostgreSQL database.

### Test scenario (corresponds to AUTHZ_USAGE_SCENARIOS S14)

```
Setup:
  1. Create type "project" (can_be_root=true, allowed_memberships=["task"])
  2. Create type "task" (can_be_root=true)
  3. Create group "ProjectA" (type=project)
  4. Create group "ProjectB" (type=project)
  5. Add membership (ProjectA, task, task-001)
  6. Add membership (ProjectA, task, task-002)
  7. Add membership (ProjectB, task, task-003)

Test:
  8. GET /tasks (user has access to ProjectA only)
     → AuthZ plugin: capabilities=["group_membership"]
     → Plugin calls RG: list groups for user's role
     → Returns: in_group(id, [ProjectA])
     → SQL: WHERE id IN (SELECT resource_id FROM resource_group_membership
                          WHERE group_id = 'ProjectA-uuid')
     → Result: task-001, task-002 (not task-003)
```

### Verification

- Tasks in ProjectA visible, tasks in ProjectB invisible
- SQL query contains JOIN against `resource_group_membership`
- AuthZ plugin logged calls to `ResourceGroupReadHierarchy`

---

## Phase 3: MTLS Authentication Mode

**Goal**: Verify that AuthZ plugin can read RG hierarchy via MTLS-authenticated request (microservice deployment mode), bypassing AuthZ evaluation.

### What needs to be implemented

#### 3.1 Certificate infrastructure

Generate self-signed certs for dev:

```bash
# CA
openssl req -x509 -newkey rsa:2048 -keyout ca-key.pem -out ca.pem -days 365 -nodes \
  -subj "/CN=rg-mtls-ca"

# AuthZ plugin client cert
openssl req -newkey rsa:2048 -keyout plugin-key.pem -out plugin.csr -nodes \
  -subj "/CN=authz-resolver-plugin"
openssl x509 -req -in plugin.csr -CA ca.pem -CAkey ca-key.pem -out plugin.pem -days 365
```

#### 3.2 RG MTLS configuration

```yaml
modules:
  resource-group:
    config:
      mtls:
        ca_cert: "certs/ca.pem"
        allowed_clients: ["authz-resolver-plugin"]
        allowed_endpoints:
          - method: GET
            path: "/api/resource-group/v1/groups/{group_id}/hierarchy"
```

#### 3.3 API Gateway TLS termination

Configure API Gateway to forward client certificate CN header to RG module for MTLS mode detection.

### Test scenario

```bash
# MTLS request to allowed endpoint (hierarchy) — AuthZ bypassed
curl --cert plugin.pem --key plugin-key.pem --cacert ca.pem \
  https://127.0.0.1:8087/cf/resource-group/v1/groups/{group_id}/hierarchy
# Expected: 200 OK with hierarchy data

# MTLS request to disallowed endpoint (POST groups) — rejected
curl --cert plugin.pem --key plugin-key.pem --cacert ca.pem \
  -X POST https://127.0.0.1:8087/cf/resource-group/v1/groups
# Expected: 403 Forbidden

# JWT request to hierarchy endpoint — full AuthZ applied
curl -H "Authorization: Bearer test" \
  http://127.0.0.1:8087/cf/resource-group/v1/groups/{group_id}/hierarchy
# Expected: 200 OK with AuthZ-scoped results
```

### Verification

- MTLS + allowed endpoint → 200, no AuthZ evaluation in logs
- MTLS + disallowed endpoint → 403
- JWT + same endpoint → 200, AuthZ evaluation logged
- Invalid cert CN → 403

---

## Effort Estimate

| Phase | Scope | Effort | Status |
|-------|-------|--------|--------|
| Phase 1 | Tenant scoping via PolicyEnforcer | 2–3 hours | **Done** |
| Phase 2 | Group predicates (in_group/in_group_subtree) | 1–2 days | Not started |
| Phase 3 | MTLS verification | 2–3 hours | Not started |

**Recommended order**: Phase 1 ✅ → Phase 3 → Phase 2 (Phase 1 is prerequisite for AuthZ integration; Phase 3 is independent; Phase 2 is the largest piece).

---

## References

- [RESOURCE_GROUP_MODEL.md](./RESOURCE_GROUP_MODEL.md) — How AuthZ uses RG data
- [AUTHZ_USAGE_SCENARIOS.md](./AUTHZ_USAGE_SCENARIOS.md) — SQL-level scenarios (S14–S21 for groups)
- [RG DESIGN](../../../modules/system/resource-group/docs/DESIGN.md) — RG module design, auth modes, init sequence
- [AuthZ DESIGN](./DESIGN.md) — Core authorization design
- [RG PRD](../../../modules/system/resource-group/docs/PRD.md) — Product requirements
