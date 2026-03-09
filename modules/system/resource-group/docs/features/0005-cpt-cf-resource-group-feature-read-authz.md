# Feature: Integration Read & AuthZ Interop

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-read-authz`
- [ ] `p2` - `cpt-cf-resource-group-feature-integration-read`

**Status**: DEFERRED — blocked on AuthZ module readiness (PolicyEnforcer, MTLS infrastructure, plugin architecture)

## 1. Feature Context

### 1.1 Overview

Expose the read-only `ResourceGroupReadHierarchy` contract for AuthZ plugin consumption, implement plugin gateway routing (built-in vs vendor-specific provider), and enforce JWT/MTLS dual authentication with endpoint-level allowlisting.

### 1.2 Purpose

Without this feature, the AuthZ plugin cannot resolve tenant hierarchies to produce access constraints. The integration read contract is the bridge between RG's data layer and AuthZ's policy evaluation engine. MTLS authentication allows the AuthZ plugin to call RG without circular AuthZ evaluation. JWT authentication ensures all public API calls go through standard AuthZ flow.

Addresses:
- `cpt-cf-resource-group-fr-integration-read-port` — read-only consumer contract for hierarchy/membership access
- `cpt-cf-resource-group-fr-dual-auth-modes` — JWT (all endpoints, AuthZ-evaluated) and MTLS (hierarchy-only, AuthZ-bypassed) authentication paths
- `cpt-cf-resource-group-principle-policy-agnostic` — RG handles graph/membership data only
- `cpt-cf-resource-group-principle-tenant-scope-ownership-graph` — tenant-scoped ownership-graph semantics
- `cpt-cf-resource-group-constraint-no-authz-decision` — RG cannot return allow/deny decisions
- `cpt-cf-resource-group-constraint-no-sql-filter-generation` — RG cannot generate SQL fragments or access-scope objects

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-authz-plugin-consumer` | Reads hierarchy data via `ResourceGroupReadHierarchy` (MTLS path or in-process ClientHub) |
| `cpt-cf-resource-group-actor-tenant-administrator` | Accesses RG API via JWT path — all endpoints with AuthZ evaluation |
| `cpt-cf-resource-group-actor-instance-administrator` | Accesses RG API via JWT path — all endpoints with AuthZ evaluation |
| `cpt-cf-resource-group-actor-apps` | Programmatic access via `ResourceGroupClient` SDK or REST API |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md)
  - [ ] `p2` - `cpt-cf-resource-group-feature-integration-read`
- **Design Components**: `cpt-cf-resource-group-component-integration-read-service`
- **Design Sequences**: `cpt-cf-resource-group-seq-authz-rg-sql-split`, `cpt-cf-resource-group-seq-e2e-authz-flow`, `cpt-cf-resource-group-seq-auth-modes`, `cpt-cf-resource-group-seq-mtls-authz-read`, `cpt-cf-resource-group-seq-jwt-rg-request`
- **Dependencies**:
  - [x] `p1` - `cpt-cf-resource-group-feature-entity-hierarchy`
- **External Dependencies** (BLOCKING):
  - AuthZ module — `PolicyEnforcer`, `AuthZResolverClient`, `EvaluationRequest`
  - AuthN module — MTLS certificate verification, `SecurityContext` population
  - Platform infrastructure — MTLS CA bundle, endpoint allowlist configuration

## 2. Actor Flows (CDSL)

### MTLS Hierarchy Read Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-mtls-hierarchy-read`

**Actor**: `cpt-cf-resource-group-actor-authz-plugin-consumer`

**Success Scenarios**:
- AuthZ plugin reads hierarchy data via MTLS — returns `Page<ResourceGroupWithDepth>`

**Error Scenarios**:
- Invalid/untrusted client certificate — `403 Forbidden`
- Endpoint not in MTLS allowlist — `403 Forbidden`
- Reference group not found — `NotFound`

**Steps**:
1. [ ] - `p1` - AuthZ Plugin sends MTLS request: GET /api/resource-group/v1/groups/{group_id}/depth (client certificate) - `inst-mtls-read-1`
2. [ ] - `p1` - RG Gateway: verify client certificate against trusted CA bundle - `inst-mtls-read-2`
3. [ ] - `p1` - **IF** certificate invalid or untrusted - `inst-mtls-read-3`
   1. [ ] - `p1` - **RETURN** `403 Forbidden` — untrusted certificate - `inst-mtls-read-3a`
4. [ ] - `p1` - RG Gateway: check endpoint against MTLS allowlist (`/groups/{id}/depth` only) - `inst-mtls-read-4`
5. [ ] - `p1` - **IF** endpoint not in allowlist - `inst-mtls-read-5`
   1. [ ] - `p1` - **RETURN** `403 Forbidden` — endpoint not allowed for MTLS - `inst-mtls-read-5a`
6. [ ] - `p1` - RG Gateway: construct system `SecurityContext` (MTLS identity, no AuthZ evaluation) - `inst-mtls-read-6`
7. [ ] - `p1` - Delegate to `ResourceGroupReadHierarchy.list_group_depth(system_ctx, group_id, query)` - `inst-mtls-read-7`
8. [ ] - `p1` - **RETURN** `Page<ResourceGroupWithDepth>` - `inst-mtls-read-8`

### JWT Authenticated Request Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-jwt-request`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- User accesses RG API via JWT — request evaluated by AuthZ, `AccessScope` applied

**Error Scenarios**:
- Invalid JWT — `401 Unauthorized`
- AuthZ denies access — `403 Forbidden`

**Steps**:
1. [ ] - `p1` - Actor sends JWT request to any RG REST endpoint - `inst-jwt-req-1`
2. [ ] - `p1` - API Gateway authenticates JWT via `AuthNResolverClient` — returns `SecurityContext` - `inst-jwt-req-2`
3. [ ] - `p1` - **IF** JWT invalid — **RETURN** `401 Unauthorized` - `inst-jwt-req-3`
4. [ ] - `p1` - RG Gateway calls `PolicyEnforcer.access_scope(ctx, RESOURCE_GROUP, action)` - `inst-jwt-req-4`
5. [ ] - `p1` - PolicyEnforcer calls `AuthZResolverClient.evaluate(EvaluationRequest)` - `inst-jwt-req-5`
6. [ ] - `p1` - AuthZ Plugin internally calls `ResourceGroupReadHierarchy.list_group_depth()` for hierarchy resolution - `inst-jwt-req-6`
7. [ ] - `p1` - AuthZ returns decision + constraints (e.g., `owner_tenant_id IN (T1, T7)`) - `inst-jwt-req-7`
8. [ ] - `p1` - PolicyEnforcer compiles constraints into `AccessScope` - `inst-jwt-req-8`
9. [ ] - `p1` - RG service applies `AccessScope` via SecureORM to query - `inst-jwt-req-9`
10. [ ] - `p1` - **IF** AuthZ denies — **RETURN** `403 Forbidden` - `inst-jwt-req-10`
11. [ ] - `p1` - **RETURN** scoped query results - `inst-jwt-req-11`

### Plugin Gateway Routing Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-plugin-gateway`

**Actor**: `cpt-cf-resource-group-actor-apps`

**Success Scenarios**:
- Built-in provider: local hierarchy data path — direct delegation to `ResourceGroupReadHierarchy`
- Vendor provider: resolve scoped plugin instance via `ResourceGroupReadPluginClient`

**Error Scenarios**:
- Plugin not found — `ServiceUnavailable`

**Steps**:
1. [ ] - `p1` - Caller requests hierarchy read via `ResourceGroupReadHierarchy` SDK trait - `inst-plugin-gw-1`
2. [ ] - `p1` - Determine provider type from configuration (built-in vs vendor) - `inst-plugin-gw-2`
3. [ ] - `p1` - **IF** built-in provider - `inst-plugin-gw-3`
   1. [ ] - `p1` - Delegate directly to local `GroupService.list_group_depth()` - `inst-plugin-gw-3a`
4. [ ] - `p1` - **ELSE** (vendor provider) - `inst-plugin-gw-4`
   1. [ ] - `p1` - Resolve scoped plugin instance via `ResourceGroupReadPluginClient` trait - `inst-plugin-gw-4a`
   2. [ ] - `p1` - **IF** plugin not found — **RETURN** `ServiceUnavailable` - `inst-plugin-gw-4b`
   3. [ ] - `p1` - Delegate to vendor plugin instance - `inst-plugin-gw-4c`
5. [ ] - `p1` - **RETURN** plugin response - `inst-plugin-gw-5`

## 3. Processes / Business Logic (CDSL)

### MTLS Endpoint Allowlist Enforcement

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-mtls-allowlist`

**Input**: Request path, MTLS mode flag

**Output**: Allow or reject with 403

**Steps**:
1. [ ] - `p1` - **IF** request is MTLS authenticated - `inst-allowlist-1`
2. [ ] - `p1` - Check request path against allowlist: only `/groups/{id}/depth` (hierarchy endpoint) - `inst-allowlist-2`
3. [ ] - `p1` - **IF** path matches allowlist — **RETURN** allow - `inst-allowlist-3`
4. [ ] - `p1` - **ELSE** — **RETURN** `403 Forbidden` — endpoint not allowed for MTLS - `inst-allowlist-4`

### Authentication Mode Detection

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-auth-mode-detection`

**Input**: Incoming request headers/transport metadata

**Output**: JWT mode or MTLS mode

**Steps**:
1. [ ] - `p1` - **IF** request has valid client TLS certificate — **RETURN** MTLS mode - `inst-auth-detect-1`
2. [ ] - `p1` - **IF** request has Authorization Bearer header — **RETURN** JWT mode - `inst-auth-detect-2`
3. [ ] - `p1` - **ELSE** — **RETURN** `401 Unauthorized` — no valid authentication - `inst-auth-detect-3`

## 4. States (CDSL)

Not applicable. Integration read operations are stateless — they return data from the hierarchy without lifecycle transitions.

## 5. Definitions of Done

### Integration Read Contract

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-integration-read-contract`

The system **MUST** expose `ResourceGroupReadHierarchy` as a ClientHub-registered trait providing read-only hierarchy traversal. The trait **MUST** have a single method `list_group_depth(ctx, group_id, query)` returning `Page<ResourceGroupWithDepth>`. This contract **MUST** be the only way AuthZ plugins access RG hierarchy data. The implementation **MUST NOT** make AuthZ decisions or generate SQL/ORM filter fragments.

**Implements**:
- `cpt-cf-resource-group-flow-mtls-hierarchy-read` (step 7)
- `cpt-cf-resource-group-flow-plugin-gateway`

**Touches**:
- SDK: `ResourceGroupReadHierarchy` trait
- ClientHub: `Arc<dyn ResourceGroupReadHierarchy>` registration
- DB: `resource_group`, `resource_group_closure` (read)

### Dual Authentication Modes

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-dual-auth`

The system **MUST** support two authentication paths: (1) JWT — all endpoints, with AuthZ evaluation via `PolicyEnforcer` producing `AccessScope`, and (2) MTLS — hierarchy endpoint only, bypassing AuthZ with system `SecurityContext`. MTLS **MUST** enforce endpoint allowlist: only `GET /groups/{id}/depth` is reachable; all other endpoints return `403`. JWT path **MUST** apply `AccessScope` to queries via SecureORM.

**Implements**:
- `cpt-cf-resource-group-flow-mtls-hierarchy-read`
- `cpt-cf-resource-group-flow-jwt-request`
- `cpt-cf-resource-group-algo-mtls-allowlist`
- `cpt-cf-resource-group-algo-auth-mode-detection`

**Touches**:
- API: All RG REST endpoints (JWT), `GET /groups/{id}/depth` (MTLS)
- Module: RG Gateway authentication middleware
- External: `PolicyEnforcer`, `AuthNResolverClient`, MTLS CA bundle

### Plugin Gateway Routing

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-plugin-gateway`

The system **MUST** support pluggable hierarchy read providers: built-in (local data path) and vendor-specific (via `ResourceGroupReadPluginClient` trait). Built-in provider delegates directly to `GroupService`. Vendor provider resolves a scoped plugin instance. Provider selection **MUST** be configuration-driven.

**Implements**:
- `cpt-cf-resource-group-flow-plugin-gateway`

**Touches**:
- SDK: `ResourceGroupReadPluginClient` trait
- Config: provider selection
- ClientHub: plugin instance resolution

## 6. Acceptance Criteria

- [ ] `ResourceGroupReadHierarchy` trait is registered in ClientHub as `Arc<dyn ResourceGroupReadHierarchy>`
- [ ] `list_group_depth` returns hierarchy data with relative depth from reference group
- [ ] AuthZ plugin can call `ResourceGroupReadHierarchy` via in-process ClientHub or MTLS REST
- [ ] MTLS requests with valid client certificate reach `/groups/{id}/depth` endpoint
- [ ] MTLS requests to any other endpoint return `403 Forbidden`
- [ ] MTLS requests with invalid/untrusted certificate return `403 Forbidden`
- [ ] MTLS requests bypass AuthZ evaluation (no `PolicyEnforcer` call)
- [ ] MTLS `SecurityContext` is system-level (trusted service principal)
- [ ] JWT requests to all endpoints are authenticated via `AuthNResolverClient`
- [ ] JWT requests go through `PolicyEnforcer.access_scope()` before query execution
- [ ] `AccessScope` from AuthZ evaluation is applied to RG queries via SecureORM
- [ ] AuthZ denial returns `403 Forbidden`
- [ ] Built-in plugin provider delegates to local `GroupService.list_group_depth()`
- [ ] Vendor plugin provider resolves via `ResourceGroupReadPluginClient` trait
- [ ] Plugin not found returns `ServiceUnavailable`
- [ ] RG does not make AuthZ decisions (policy-agnostic)
- [ ] RG does not generate SQL fragments or access-scope objects
- [ ] Hierarchy read responses include `tenant_id` for each group
- [ ] `SecurityContext` is forwarded to read operations without policy interpretation

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — integration reads are stateless.
- **Usability (UX)**: Not applicable — backend API only.
- **Compliance (COMPL)**: Not applicable — read contract does not introduce new compliance requirements.
- **Operations (OPS)**: Standard platform patterns. MTLS certificate rotation is an infrastructure concern outside this feature.

## 8. Implementation Notes (DEFERRED)

This feature is **DEFERRED** pending the following external dependencies:

1. **AuthZ module** — `PolicyEnforcer`, `AuthZResolverClient`, `EvaluationRequest/Response` types must be available for JWT authentication path integration.
2. **AuthN module** — MTLS certificate verification middleware must exist in the platform layer.
3. **Plugin architecture** — `ResourceGroupReadPluginClient` trait requires the GTS plugin discovery mechanism to be available.
4. **Configuration** — MTLS CA bundle path, endpoint allowlist, provider selection configuration must be defined.

**What is already implemented** (from Features 1-3):
- `ResourceGroupReadHierarchy` trait definition in SDK (`api.rs`)
- `ResourceGroupReadHierarchy` implementation in `service.rs` (delegates to `list_group_depth`)
- `list_group_depth` REST endpoint wired and functional
- ClientHub registration of `ResourceGroupReadHierarchy` in `module.rs`

**What remains**:
- MTLS authentication middleware and endpoint allowlist enforcement
- JWT + `PolicyEnforcer` integration for access-scoped queries
- Plugin gateway routing (built-in vs vendor provider)
- `ResourceGroupReadPluginClient` trait definition and registration
- `AccessScope` application to all query paths via SecureORM
