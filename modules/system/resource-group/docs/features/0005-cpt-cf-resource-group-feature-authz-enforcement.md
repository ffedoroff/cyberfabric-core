# Feature: AuthZ Enforcement

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-authz-enforcement`
- [x] `p1` - `cpt-cf-resource-group-feature-authz-enforcement`

## 1. Feature Context

### 1.1 Overview

Integrate PolicyEnforcer into all RG REST handlers to enforce authorization on every request. Group and membership endpoints receive tenant-scoped `AccessScope` constraints compiled from AuthZ evaluation; type endpoints require authentication but operate on global (tenant-independent) data. All repository queries are executed through SecureORM with the resolved `AccessScope`.

### 1.2 Purpose

Features 1-4 implemented RG domain logic and REST endpoints with `.authenticated()` mode but without authorization evaluation. `SecurityContext` is extracted in handlers but unused (`_ctx`). This feature closes the authorization gap by wiring PolicyEnforcer into the request pipeline so that every query respects the caller's tenant scope and every mutation is checked against the AuthZ policy.

Addresses:
- `cpt-cf-resource-group-fr-dual-auth-modes` — JWT authentication path with AuthZ evaluation via PolicyEnforcer
- `cpt-cf-resource-group-principle-policy-agnostic` — RG does not make AuthZ decisions; it delegates to PolicyEnforcer and applies the resulting `AccessScope`
- `cpt-cf-resource-group-constraint-no-authz-decision` — RG never interprets policy; it only applies compiled constraints
- `cpt-cf-resource-group-constraint-no-sql-filter-generation` — RG does not generate SQL fragments; SecureORM handles constraint-to-SQL compilation

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-tenant-administrator` | Accesses group and membership endpoints — PolicyEnforcer scopes queries to caller's tenant |
| `cpt-cf-resource-group-actor-instance-administrator` | Accesses type endpoints (global) and group/membership endpoints (full scope) |
| `cpt-cf-resource-group-actor-apps` | Programmatic access via REST API — same PolicyEnforcer flow as human actors |
| `cpt-cf-resource-group-actor-authz-plugin-consumer` | Calls `ResourceGroupReadHierarchy` via in-process ClientHub with system `SecurityContext` (bypasses PolicyEnforcer) |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md)
  - [x] `p1` - `cpt-cf-resource-group-feature-authz-enforcement`
- **AuthZ Architecture**: `docs/arch/authorization/DESIGN.md` — PDP/PEP model, constraint compilation
- **PolicyEnforcer SDK**: `authz-resolver-sdk/src/pep/enforcer.rs` — `PolicyEnforcer`, `ResourceType`, `AccessRequest`
- **AccessScope**: `libs/modkit-security/src/access_scope.rs` — `AccessScope`, `pep_properties`
- **SecureORM**: `libs/modkit-db/src/secure/` — `SecureConn`, `SecureTx`
- **Design Components**: `cpt-cf-resource-group-component-module`, all service components
- **Design Constraints**: `cpt-cf-resource-group-constraint-no-authz-decision`, `cpt-cf-resource-group-constraint-no-sql-filter-generation`
- **Dependencies**:
  - [x] `p1` - `cpt-cf-resource-group-feature-domain-foundation`
  - [x] `p1` - `cpt-cf-resource-group-feature-type-management`
  - [x] `p1` - `cpt-cf-resource-group-feature-entity-hierarchy`
  - [x] `p2` - `cpt-cf-resource-group-feature-membership`
- **External Dependencies**:
  - AuthZ Resolver module — `AuthZResolverClient` registered in ClientHub
  - Static AuthZ Plugin (or vendor plugin) — returns `OWNER_TENANT_ID` constraints

## 2. Actor Flows (CDSL)

### JWT Scoped Request Flow (Groups & Memberships)

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-jwt-scoped-request`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Tenant admin accesses groups/memberships — PolicyEnforcer returns `AccessScope` with `OWNER_TENANT_ID IN (...)` constraint, queries return only tenant-visible data

**Error Scenarios**:
- AuthZ denies access — `403 Forbidden`
- AuthZ service unavailable — `503 Service Unavailable`
- Constraint compilation fails — `500 Internal Server Error`

**Steps**:
1. [x] - `p1` - Actor sends JWT request to group or membership endpoint - `inst-jwt-scoped-1`
2. [x] - `p1` - API Gateway authenticates JWT — `SecurityContext` populated with `subject_id`, `subject_tenant_id`, `token_scopes` - `inst-jwt-scoped-2`
3. [x] - `p1` - Handler extracts `SecurityContext` and determines action (`list`, `get`, `create`, `update`, `delete`) - `inst-jwt-scoped-3`
4. [x] - `p1` - Handler calls `PolicyEnforcer.access_scope(ctx, RESOURCE_GROUP, action, resource_id)` - `inst-jwt-scoped-4`
5. [x] - `p1` - PolicyEnforcer builds `EvaluationRequest` with subject, action, resource type, context (tenant_id, supported_properties: `[OWNER_TENANT_ID, RESOURCE_ID]`) - `inst-jwt-scoped-5`
6. [x] - `p1` - PolicyEnforcer calls `AuthZResolverClient.evaluate(request)` — PDP returns decision + constraints - `inst-jwt-scoped-6`
7. [x] - `p1` - **IF** decision is false - `inst-jwt-scoped-7`
   1. [x] - `p1` - **RETURN** `403 Forbidden` with deny reason (if provided) - `inst-jwt-scoped-7a`
8. [x] - `p1` - PolicyEnforcer compiles constraints into `AccessScope` (e.g., `OWNER_TENANT_ID IN (T1, T7)`) - `inst-jwt-scoped-8`
9. [x] - `p1` - **IF** compilation fails (unknown properties, all constraints failed) - `inst-jwt-scoped-9`
   1. [x] - `p1` - **RETURN** `500 Internal Server Error` — constraint compilation failure - `inst-jwt-scoped-9a`
10. [x] - `p1` - Handler passes `AccessScope` to domain service method - `inst-jwt-scoped-10`
11. [x] - `p1` - Domain service passes `AccessScope` to repository — SecureORM applies `WHERE tenant_id IN (...)` to queries - `inst-jwt-scoped-11`
12. [x] - `p1` - **RETURN** scoped query results to actor - `inst-jwt-scoped-12`

### Global Admin Request Flow (Types)

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-global-admin-request`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Instance admin accesses type endpoints — types are global, no tenant scoping needed

**Error Scenarios**:
- AuthZ denies access — `403 Forbidden`
- AuthZ service unavailable — `503 Service Unavailable`

**Steps**:
1. [x] - `p1` - Actor sends JWT request to type endpoint (`/types`, `/types/{code}`) - `inst-global-admin-1`
2. [x] - `p1` - API Gateway authenticates JWT — `SecurityContext` populated - `inst-global-admin-2`
3. [x] - `p1` - Handler calls `PolicyEnforcer.access_scope(ctx, RESOURCE_GROUP_TYPE, action, None)` - `inst-global-admin-3`
4. [x] - `p1` - PolicyEnforcer evaluates — PDP returns decision (types are global, constraint may be empty) - `inst-global-admin-4`
5. [x] - `p1` - **IF** decision is false - `inst-global-admin-5`
   1. [x] - `p1` - **RETURN** `403 Forbidden` - `inst-global-admin-5a`
6. [x] - `p1` - **IF** `AccessScope` is unconstrained (no tenant filter needed for global types) - `inst-global-admin-6`
   1. [x] - `p1` - Handler calls domain service without tenant scope restriction - `inst-global-admin-6a`
7. [x] - `p1` - **RETURN** type data to actor - `inst-global-admin-7`

### ReadHierarchy System Access Flow

- [x] `p2` - **ID**: `cpt-cf-resource-group-flow-system-hierarchy-read`

**Actor**: `cpt-cf-resource-group-actor-authz-plugin-consumer`

**Success Scenarios**:
- AuthZ plugin reads hierarchy data via in-process ClientHub — bypasses PolicyEnforcer entirely

**Error Scenarios**:
- RG module not initialized — `ClientNotFound` from ClientHub

**Steps**:
1. [x] - `p2` - AuthZ plugin resolves `Arc<dyn ResourceGroupReadHierarchy>` from ClientHub - `inst-sys-read-1`
2. [x] - `p2` - AuthZ plugin calls `list_group_depth(system_ctx, group_id, query)` with system `SecurityContext` - `inst-sys-read-2`
3. [x] - `p2` - RG service executes hierarchy query without PolicyEnforcer evaluation (in-process trust boundary) - `inst-sys-read-3`
4. [x] - `p2` - **RETURN** `Page<ResourceGroupWithDepth>` to AuthZ plugin - `inst-sys-read-4`

## 3. Processes / Business Logic (CDSL)

### PolicyEnforcer AccessScope Resolution

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-access-scope-resolution`

**Input**: `SecurityContext`, `ResourceType` descriptor, action string, optional resource_id

**Output**: `AccessScope` or `EnforcerError`

**Steps**:
1. [x] - `p1` - Build `EvaluationRequest` from `SecurityContext`: extract `subject_id`, `subject_tenant_id`, `token_scopes`, `bearer_token` - `inst-scope-res-1`
2. [x] - `p1` - Set `resource.resource_type` to `ResourceType.name` (e.g., `gts.x.core.system.resource_group.v1~`) - `inst-scope-res-2`
3. [x] - `p1` - Set `context.supported_properties` to `ResourceType.supported_properties` - `inst-scope-res-3`
4. [x] - `p1` - Set `context.require_constraints` based on action: `true` for list/get/update/delete, `false` for create - `inst-scope-res-4`
5. [x] - `p1` - Call `AuthZResolverClient.evaluate(request)` - `inst-scope-res-5`
6. [x] - `p1` - **IF** evaluation RPC fails — **RETURN** `EnforcerError::EvaluationFailed` - `inst-scope-res-6`
7. [x] - `p1` - **IF** `decision == false` — **RETURN** `EnforcerError::Denied` with optional `deny_reason` - `inst-scope-res-7`
8. [x] - `p1` - Compile constraints to `AccessScope` via `compile_to_access_scope()` - `inst-scope-res-8`
9. [x] - `p1` - **IF** compilation fails — **RETURN** `EnforcerError::CompileFailed` - `inst-scope-res-9`
10. [x] - `p1` - **RETURN** `AccessScope` (may be `allow_all()` for unconstrained, or contain `ScopeConstraint` list) - `inst-scope-res-10`

### EnforcerError to HTTP Response Mapping

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-enforcer-error-mapping`

**Input**: `EnforcerError` from PolicyEnforcer

**Output**: HTTP error response (Problem RFC 9457)

**Steps**:
1. [x] - `p1` - **IF** `EnforcerError::Denied { deny_reason }` - `inst-err-map-1`
   1. [x] - `p1` - **RETURN** `403 Forbidden` — Problem with type `authorization:denied`, include deny_reason if present - `inst-err-map-1a`
2. [x] - `p1` - **IF** `EnforcerError::EvaluationFailed(authz_error)` - `inst-err-map-2`
   1. [x] - `p1` - **RETURN** `503 Service Unavailable` — Problem with type `authorization:service-unavailable`, no internal details leaked - `inst-err-map-2a`
3. [x] - `p1` - **IF** `EnforcerError::CompileFailed(compile_error)` - `inst-err-map-3`
   1. [x] - `p1` - **RETURN** `500 Internal Server Error` — Problem with type `authorization:compile-error`, logged with full detail server-side - `inst-err-map-3a`

## 4. States (CDSL)

Not applicable. Authorization enforcement is stateless — each request is independently evaluated against the current policy. No entity lifecycle states are introduced.

## 5. Definitions of Done

### PolicyEnforcer Integration in Module Init

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-enforcer-init`

The system **MUST** resolve `Arc<dyn AuthZResolverClient>` from ClientHub during module initialization. The system **MUST** instantiate `PolicyEnforcer::new(authz)` and make it available to all REST handlers (via Axum Extension or service injection). The enforcer instance **MUST** be shared across all handlers (single allocation). The module **MUST NOT** fail startup if AuthZ module is not yet registered — it **MUST** defer resolution to first request (lazy init) or handle the circular dependency via phased initialization per `cpt-cf-resource-group-seq-init-order`.

**Implements**:
- `cpt-cf-resource-group-flow-jwt-scoped-request` (step 4)
- `cpt-cf-resource-group-flow-global-admin-request` (step 3)
- `cpt-cf-resource-group-algo-access-scope-resolution`

**Touches**:
- Module: `module.rs` init
- ClientHub: `Arc<dyn AuthZResolverClient>` resolution
- Entities: `PolicyEnforcer`

### Group Endpoint Authorization

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-group-authz`

All group REST handlers (`list`, `get`, `create`, `update`, `delete`, `list_depth`) **MUST** call `PolicyEnforcer.access_scope(ctx, RESOURCE_GROUP, action, resource_id)` before executing domain logic. The resulting `AccessScope` **MUST** be passed to domain services and propagated to repository queries. For `list` and `list_depth`, SecureORM applies `WHERE tenant_id IN (...)` from `AccessScope` constraints. For `get`, `update`, `delete`, the system **MUST** verify that the target group is within the caller's `AccessScope` (either via SecureORM scoped query returning empty, or explicit `AccessScope.contains_uuid()` check). For `create`, the system **MUST** verify that the target `tenant_id` is within the caller's scope.

**Implements**:
- `cpt-cf-resource-group-flow-jwt-scoped-request`

**Touches**:
- API: all `/groups/*` endpoints
- Domain: `GroupService` methods — accept `AccessScope` parameter
- DB: `resource_group` queries — SecureORM applies tenant scope

### Membership Endpoint Authorization

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-membership-authz`

All membership REST handlers (`list`, `add`, `remove`) **MUST** call `PolicyEnforcer.access_scope()` before executing domain logic. For `list`, SecureORM scopes the query via the membership's associated group `tenant_id` (JOIN to `resource_group`). For `add` and `remove`, the system **MUST** verify that the target group's `tenant_id` is within the caller's `AccessScope`. The `AccessScope` **MUST** be propagated through the domain service to the repository layer.

**Implements**:
- `cpt-cf-resource-group-flow-jwt-scoped-request`

**Touches**:
- API: all `/memberships/*` endpoints
- Domain: `MembershipService` methods — accept `AccessScope` parameter
- DB: `resource_group_membership` queries — scope via `resource_group.tenant_id` JOIN

### Type Endpoint Authorization

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-authz`

Type REST handlers (`list`, `get`, `create`, `update`, `delete`) **MUST** call `PolicyEnforcer.access_scope(ctx, RESOURCE_GROUP_TYPE, action, None)` to verify the caller is authorized. Types are global (tenant-independent) resources, so the `AccessScope` result may be unconstrained (`allow_all()`). The PDP policy decides who can manage types (typically instance administrators only). If the PDP denies the request, the handler **MUST** return `403 Forbidden`. Type queries do **NOT** apply tenant-scoped `WHERE` clauses since `resource_group_type` has no `tenant_id` column.

**Implements**:
- `cpt-cf-resource-group-flow-global-admin-request`

**Touches**:
- API: all `/types/*` endpoints
- Domain: `TypeService` methods — verify authorization, no scope filtering needed

### ResourceType Descriptor Definition

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-resource-type-descriptor`

The system **MUST** define two `ResourceType` descriptors: (1) `RESOURCE_GROUP` for group and membership endpoints with `supported_properties: [OWNER_TENANT_ID, RESOURCE_ID]`, and (2) `RESOURCE_GROUP_TYPE` for type endpoints with `supported_properties: [RESOURCE_ID]` (no tenant column). The `name` field **MUST** follow the GTS naming convention (e.g., `gts.x.core.system.resource_group.v1~`). These descriptors are compile-time constants used by `PolicyEnforcer.access_scope()`.

**Implements**:
- `cpt-cf-resource-group-algo-access-scope-resolution` (steps 2-3)

**Touches**:
- Module: constant definitions
- SDK: `ResourceType` from `authz-resolver-sdk`

## 6. Acceptance Criteria

- [x] `PolicyEnforcer` is instantiated in module init using `AuthZResolverClient` from ClientHub
- [x] All group endpoints call `access_scope()` before domain logic
- [x] All membership endpoints call `access_scope()` before domain logic
- [x] All type endpoints call `access_scope_with(require_constraints=false)` before domain logic
- [x] `GET /groups` returns only groups within caller's tenant scope
- [x] `GET /groups/{id}` for a group outside caller's tenant scope returns `404` (scoped query returns empty)
- [x] `POST /groups` with `tenant_id` outside caller's scope is rejected
- [x] `DELETE /groups/{id}` for a group outside caller's scope returns `404`
- [x] `GET /memberships` returns only memberships for groups within caller's tenant scope
- [x] `POST /memberships/{group_id}/...` for a group outside caller's scope returns `404`
- [x] Type endpoints are accessible to authorized instance administrators
- [x] Type endpoints reject unauthorized callers with `403 Forbidden`
- [x] AuthZ denial returns `403 Forbidden` with Problem RFC 9457 format
- [x] AuthZ service unavailable returns `503 Service Unavailable`
- [x] Constraint compilation failure returns `500 Internal Server Error`
- [x] AuthZ plugin can resolve `ResourceGroupReadHierarchy` from ClientHub and call `list_group_depth` without PolicyEnforcer evaluation
- [x] `SecurityContext` is forwarded to PolicyEnforcer without modification by RG
- [x] RG does not interpret AuthZ policy — only applies compiled `AccessScope`

## 7. Design Notes

### `require_constraints` contract for global vs tenant-scoped resources

Type operations use `access_scope_with(AccessRequest::new().require_constraints(false))` because:
- `RESOURCE_GROUP_TYPE` descriptor has `supported_properties: [RESOURCE_ID]` (no `OWNER_TENANT_ID`)
- PDP returns empty constraints for resources without a tenant property
- `require_constraints(true)` + empty constraints = `ConstraintsRequiredButAbsent` error from the compiler
- Therefore type operations **MUST** use `require_constraints(false)` to avoid false 403s

Group and membership operations use default `access_scope()` which implies `require_constraints(true)`:
- `RESOURCE_GROUP` descriptor has `supported_properties: [OWNER_TENANT_ID, RESOURCE_ID]`
- PDP returns `OWNER_TENANT_ID` constraints for tenant-scoped resources
- Constraints are compiled to `AccessScope` with `ScopeConstraint` list

This contract is implicit in the interaction between `ResourceType.supported_properties`, PDP constraint generation, and PEP `require_constraints` flag. If a new global resource type is added, it must also use `require_constraints(false)`.

## 8. Non-Applicable Domains

- **States (CDSL)**: Not applicable — authorization is stateless per-request evaluation.
- **Usability (UX)**: Not applicable — backend API only.
- **Compliance (COMPL)**: Not applicable — compliance controls are platform-level.
- **MTLS Authentication**: Deferred to Feature 6 (`cpt-cf-resource-group-feature-integration-read`) — requires platform MTLS infrastructure.
- **Plugin Gateway Routing**: Deferred to Feature 6 — `ResourceGroupReadPluginClient` and vendor provider selection.
