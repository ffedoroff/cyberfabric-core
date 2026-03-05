# Feature: Integration Read & AuthZ Interop

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-integration-read`
- [ ] `p2` - `cpt-cf-resource-group-feature-integration-read`

## 1. Feature Context

### 1.1 Overview

Expose the read-only `ResourceGroupReadHierarchy` contract for AuthZ plugin consumption via SDK and REST, implement plugin gateway routing (built-in data path vs vendor-specific provider delegation), and enforce JWT/MTLS dual authentication with endpoint-level MTLS allowlisting. This feature bridges RG hierarchy data to the AuthZ evaluation flow while maintaining strict policy-agnostic boundaries.

### 1.2 Purpose

AuthZ plugins need a stable, narrow read interface to resolve tenant hierarchy context during policy evaluation. Without this feature, the AuthZ plugin cannot obtain group hierarchy data to generate tenant-scoped constraints. The dual authentication model ensures AuthZ plugin can read hierarchy data via MTLS (bypassing its own evaluation path) while all other consumers authenticate via JWT with standard AuthZ evaluation.

Addresses:
- `cpt-cf-resource-group-fr-integration-read-port` — read-only consumer contract
- `cpt-cf-resource-group-fr-dual-auth-modes` — JWT (all endpoints) and MTLS (hierarchy-only)
- `cpt-cf-resource-group-principle-policy-agnostic` — RG returns data only
- `cpt-cf-resource-group-principle-tenant-scope-ownership-graph` — tenant-scoped reads
- `cpt-cf-resource-group-constraint-no-authz-decision` — no allow/deny decisions
- `cpt-cf-resource-group-constraint-no-sql-filter-generation` — no SQL/ORM filters

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-authz-plugin-consumer` | Reads hierarchy via `ResourceGroupReadHierarchy` SDK trait or MTLS REST endpoint to resolve tenant context during policy evaluation |
| `cpt-cf-resource-group-actor-apps` | Reads hierarchy via `ResourceGroupClient` SDK (full CRUD including reads) with JWT authentication |
| `cpt-cf-resource-group-actor-tenant-administrator` | Reads hierarchy via REST API with JWT authentication and AuthZ evaluation |
| `cpt-cf-resource-group-actor-instance-administrator` | Configures provider selection (built-in vs vendor-specific) |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) — `cpt-cf-resource-group-feature-integration-read`
- **OpenAPI**: [openapi.yaml](../openapi.yaml) — `GET /groups/{group_id}/depth` (MTLS-reachable)
- **Design Components**: `cpt-cf-resource-group-component-integration-read-service`
- **Design Sequences**: `cpt-cf-resource-group-seq-authz-rg-sql-split`, `cpt-cf-resource-group-seq-e2e-authz-flow`, `cpt-cf-resource-group-seq-auth-modes`, `cpt-cf-resource-group-seq-mtls-authz-read`, `cpt-cf-resource-group-seq-jwt-rg-request`
- **Dependencies**: `cpt-cf-resource-group-feature-entity-hierarchy` (hierarchy data must be available for reads)

## 2. Actor Flows (CDSL)

### MTLS Hierarchy Read Flow (AuthZ Plugin)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-mtls-hierarchy-read`

**Actor**: `cpt-cf-resource-group-actor-authz-plugin-consumer`

**Success Scenarios**:
- AuthZ plugin reads tenant hierarchy via MTLS-authenticated REST call to `/groups/{group_id}/depth`
- Groups returned with relative depth including `tenant_id` per row

**Error Scenarios**:
- MTLS certificate validation fails — connection rejected at transport level
- MTLS request targets non-allowed endpoint — `403 Forbidden`
- Reference group not found — `NotFound`

**Steps**:
1. [ ] - `p1` - AuthZ plugin sends API: GET /api/resource-group/v1/groups/{group_id}/depth?$filter={expr} with MTLS client certificate - `inst-mtls-1`
2. [ ] - `p1` - Transport layer validates MTLS client certificate - `inst-mtls-2`
3. [ ] - `p1` - **IF** certificate validation fails - `inst-mtls-3`
   1. [ ] - `p1` - **RETURN** connection rejected (TLS handshake failure) - `inst-mtls-3a`
4. [ ] - `p1` - Invoke MTLS endpoint allowlist enforcement (`cpt-cf-resource-group-algo-mtls-allowlist`) — verify requested endpoint is allowed - `inst-mtls-4`
5. [ ] - `p1` - **IF** endpoint not in allowlist - `inst-mtls-5`
   1. [ ] - `p1` - **RETURN** `403 Forbidden` — endpoint not reachable via MTLS - `inst-mtls-5a`
6. [ ] - `p1` - Construct system `SecurityContext` from MTLS certificate identity (trusted system principal) - `inst-mtls-6`
7. [ ] - `p1` - **SKIP** AuthZ evaluation — MTLS requests bypass PolicyEnforcer entirely - `inst-mtls-7`
8. [ ] - `p1` - Invoke plugin gateway routing (`cpt-cf-resource-group-algo-provider-routing`) to resolve data source - `inst-mtls-8`
9. [ ] - `p1` - Execute `list_group_depth(ctx, group_id, query)` against resolved provider - `inst-mtls-9`
10. [ ] - `p1` - **RETURN** `Page<ResourceGroupWithDepth>` with `tenant_id` per group row - `inst-mtls-10`

### JWT Hierarchy Read Flow (Standard Consumer)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-jwt-hierarchy-read`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Consumer reads hierarchy via JWT-authenticated REST call with full AuthZ evaluation
- Hierarchy data returned scoped by caller's effective tenant scope

**Error Scenarios**:
- JWT validation fails — `401 Unauthorized`
- AuthZ evaluation denies access — `403 Forbidden`
- Reference group not found — `NotFound`

**Steps**:
1. [ ] - `p1` - Actor sends API: GET /api/resource-group/v1/groups/{group_id}/depth?$filter={expr} with JWT bearer token - `inst-jwt-1`
2. [ ] - `p1` - AuthN layer validates JWT, produces `SecurityContext` with subject_id, subject_tenant_id, token_scopes - `inst-jwt-2`
3. [ ] - `p1` - **IF** JWT validation fails - `inst-jwt-3`
   1. [ ] - `p1` - **RETURN** `401 Unauthorized` - `inst-jwt-3a`
4. [ ] - `p1` - Handler calls PolicyEnforcer for AuthZ evaluation with SecurityContext - `inst-jwt-4`
5. [ ] - `p1` - **IF** AuthZ evaluation denies - `inst-jwt-5`
   1. [ ] - `p1` - **RETURN** `403 Forbidden` - `inst-jwt-5a`
6. [ ] - `p1` - Invoke plugin gateway routing (`cpt-cf-resource-group-algo-provider-routing`) to resolve data source - `inst-jwt-6`
7. [ ] - `p1` - Execute `list_group_depth(ctx, group_id, query)` against resolved provider - `inst-jwt-7`
8. [ ] - `p1` - **RETURN** `Page<ResourceGroupWithDepth>` with `tenant_id` per group row - `inst-jwt-8`

### SDK Integration Read Flow (AuthZ Plugin In-Process)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-sdk-integration-read`

**Actor**: `cpt-cf-resource-group-actor-authz-plugin-consumer`

**Success Scenarios**:
- AuthZ plugin resolves `ResourceGroupReadHierarchy` from ClientHub and reads hierarchy in-process
- Data returned without AuthZ evaluation (in-process call is trusted)

**Error Scenarios**:
- Client not found in ClientHub — `ClientNotFound`
- Reference group not found — `NotFound`

**Steps**:
1. [ ] - `p1` - AuthZ plugin calls `hub.get::<dyn ResourceGroupReadHierarchy>()` - `inst-sdk-read-1`
2. [ ] - `p1` - **IF** client not found — **RETURN** `ClientNotFound` error - `inst-sdk-read-2`
3. [ ] - `p1` - AuthZ plugin calls `list_group_depth(ctx, group_id, query)` with system SecurityContext - `inst-sdk-read-3`
4. [ ] - `p1` - RG gateway routes to configured provider (`cpt-cf-resource-group-algo-provider-routing`) - `inst-sdk-read-4`
5. [ ] - `p1` - Provider executes depth query against closure table - `inst-sdk-read-5`
6. [ ] - `p1` - **RETURN** `Page<ResourceGroupWithDepth>` — data rows only, no policy/decision fields - `inst-sdk-read-6`

## 3. Processes / Business Logic (CDSL)

### MTLS Endpoint Allowlist Enforcement

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-mtls-allowlist`

**Input**: MTLS-authenticated request path and method

**Output**: Allowed (proceed) or denied (403 Forbidden)

**Steps**:
1. [ ] - `p1` - Check request path against MTLS allowlist: only `GET /api/resource-group/v1/groups/{group_id}/depth` is permitted - `inst-allowlist-1`
2. [ ] - `p1` - **IF** path matches allowlist entry - `inst-allowlist-2`
   1. [ ] - `p1` - **RETURN** allowed — proceed with request - `inst-allowlist-2a`
3. [ ] - `p1` - **ELSE** - `inst-allowlist-3`
   1. [ ] - `p1` - **RETURN** denied — 403 Forbidden, endpoint not reachable via MTLS - `inst-allowlist-3a`

### Plugin Provider Routing

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-provider-routing`

**Input**: Read request (hierarchy or membership), module configuration

**Output**: Data result from selected provider

**Steps**:
1. [ ] - `p1` - Read provider configuration from module config - `inst-routing-1`
2. [ ] - `p1` - **IF** provider = built-in (default) - `inst-routing-2`
   1. [ ] - `p1` - Route to local RG persistence path — execute query against local DB via repositories - `inst-routing-2a`
   2. [ ] - `p1` - **RETURN** query result from local data path - `inst-routing-2b`
3. [ ] - `p1` - **IF** provider = vendor-specific - `inst-routing-3`
   1. [ ] - `p1` - Resolve vendor plugin instance by configured vendor name (GTS scoped plugin, same pattern as tenant-resolver/authz-resolver) - `inst-routing-3a`
   2. [ ] - `p1` - Forward SecurityContext to plugin unchanged (no policy interpretation in gateway) - `inst-routing-3b`
   3. [ ] - `p1` - Delegate to `ResourceGroupReadPluginClient` method on resolved instance - `inst-routing-3c`
   4. [ ] - `p1` - **RETURN** query result from vendor provider - `inst-routing-3d`

### Tenant Projection for Integration Reads

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-tenant-projection`

**Input**: Integration read result set (hierarchy or membership rows)

**Output**: Result with correct tenant projection per contract

**Steps**:
1. [ ] - `p1` - **IF** result is hierarchy read (`list_group_depth`) - `inst-tenproj-1`
   1. [ ] - `p1` - Each `ResourceGroupWithDepth` row includes `tenant_id` from `resource_group` table — callers use this to validate tenant scope - `inst-tenproj-1a`
   2. [ ] - `p1` - Rows can legitimately contain different `tenant_id` values when caller effective scope spans tenant hierarchy levels - `inst-tenproj-1b`
2. [ ] - `p1` - **IF** result is membership read (`list_memberships`) - `inst-tenproj-2`
   1. [ ] - `p1` - `ResourceGroupMembership` rows do NOT include `tenant_id` — callers derive tenant scope from group data already obtained via hierarchy reads - `inst-tenproj-2a`
3. [ ] - `p1` - **RETURN** result set with correct tenant projection applied - `inst-tenproj-3`

## 4. States (CDSL)

Not applicable. Integration Read Service is stateless — it routes read queries to the configured provider and returns data. There are no entity lifecycle states or state machines in this feature.

## 5. Definitions of Done

### ResourceGroupReadHierarchy Trait Implementation

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-read-hierarchy-impl`

The system **MUST** implement the `ResourceGroupReadHierarchy` trait providing `list_group_depth(ctx, group_id, query)` that returns `Page<ResourceGroupWithDepth>`. The implementation **MUST** route through the plugin gateway to the configured provider (built-in or vendor-specific). The `SecurityContext` **MUST** be forwarded to the provider without policy interpretation. Returned data **MUST** be graph data only — no policy/decision fields, no SQL fragments, no access-scope objects.

**Implements**:
- `cpt-cf-resource-group-flow-sdk-integration-read`
- `cpt-cf-resource-group-algo-provider-routing`
- `cpt-cf-resource-group-algo-tenant-projection`

**Touches**:
- Entities: `ResourceGroupWithDepth`, `ResourceGroupReadHierarchy`

### Plugin Gateway Routing

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-plugin-gateway`

The system **MUST** implement plugin gateway routing that resolves the configured RG provider: built-in provider serves reads from local persistence; vendor-specific provider resolves a scoped `ResourceGroupReadPluginClient` plugin instance by configured vendor name (GTS instance ID, same pattern as tenant-resolver/authz-resolver gateways). The `SecurityContext` **MUST** be forwarded unchanged through the gateway to the selected provider. Plugin registration **MUST** be scoped (GTS instance ID).

**Implements**:
- `cpt-cf-resource-group-algo-provider-routing`

**Touches**:
- Entities: `ResourceGroupReadPluginClient`, `ResourceGroupReadHierarchy`

### MTLS Authentication with Endpoint Allowlist

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-mtls-auth`

The system **MUST** support MTLS client certificate authentication for service-to-service requests. MTLS requests **MUST** bypass AuthZ evaluation entirely — no PolicyEnforcer call, no access_evaluation_request. Only `GET /api/resource-group/v1/groups/{group_id}/depth` **MUST** be reachable via MTLS; all other endpoints **MUST** return `403 Forbidden` in MTLS mode. This is enforced by RG gateway-level allowlist, not by AuthZ evaluation. The MTLS certificate identity **MUST** produce a system `SecurityContext` (trusted system principal).

**Implements**:
- `cpt-cf-resource-group-flow-mtls-hierarchy-read`
- `cpt-cf-resource-group-algo-mtls-allowlist`

**Touches**:
- API: `GET /api/resource-group/v1/groups/{group_id}/depth` (MTLS path)

### JWT Authentication with AuthZ Evaluation

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-jwt-auth`

The system **MUST** support JWT bearer token authentication for all RG REST API endpoints. JWT-authenticated requests **MUST** pass through AuthZ evaluation via PolicyEnforcer before accessing RG data. AuthZ evaluation uses the standard `EvaluationRequest` flow: subject properties from SecurityContext, action from endpoint operation, resource type from GTS registration. The handler **MUST** apply the resulting `AccessScope` to the read query.

**Implements**:
- `cpt-cf-resource-group-flow-jwt-hierarchy-read`

**Touches**:
- API: all endpoints under `/api/resource-group/v1/` (JWT path)

### Tenant Projection Rules

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-tenant-projection`

The system **MUST** implement tenant projection rules for integration reads: hierarchy reads (`list_group_depth`) **MUST** include `tenant_id` per group row in `ResourceGroupWithDepth`; membership reads (`list_memberships` on `ResourceGroupClient`) **MUST NOT** include `tenant_id` — callers derive tenant scope from group data obtained via hierarchy reads. Hierarchy result rows can legitimately contain different `tenant_id` values when the caller's effective scope spans tenant hierarchy levels.

**Implements**:
- `cpt-cf-resource-group-algo-tenant-projection`

**Touches**:
- Entities: `ResourceGroupWithDepth`, `ResourceGroupMembership`

### Caller Identity Propagation

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-caller-identity`

The system **MUST** propagate caller `SecurityContext` through all read paths without converting it into policy decisions. For the built-in provider path, `SecurityContext` is available to the repository layer for auditing. For the vendor plugin path, `SecurityContext` **MUST** be passed through to the plugin instance unchanged. Plugin implementations decide how/if `SecurityContext` affects read access semantics. This keeps RG data-only while preserving caller identity required by AuthZ plugin/PDP flows.

**Implements**:
- `cpt-cf-resource-group-flow-mtls-hierarchy-read`
- `cpt-cf-resource-group-flow-jwt-hierarchy-read`
- `cpt-cf-resource-group-flow-sdk-integration-read`

**Touches**:
- Entities: `SecurityContext`

## 6. Acceptance Criteria

- [ ] `ResourceGroupReadHierarchy.list_group_depth()` returns `Page<ResourceGroupWithDepth>` with hierarchy data and `tenant_id` per row
- [ ] SDK trait returns data rows only — no policy fields, no SQL fragments, no access-scope objects
- [ ] Plugin gateway routes to built-in provider by default (local DB reads)
- [ ] Plugin gateway routes to vendor-specific provider when configured (GTS scoped plugin resolution)
- [ ] SecurityContext is forwarded unchanged through gateway to provider (no policy interpretation)
- [ ] MTLS-authenticated request to `GET /groups/{group_id}/depth` succeeds and bypasses AuthZ evaluation
- [ ] MTLS-authenticated request to any other endpoint returns `403 Forbidden`
- [ ] MTLS certificate identity produces system SecurityContext (trusted principal)
- [ ] MTLS requests do not call PolicyEnforcer
- [ ] JWT-authenticated request to any RG endpoint passes through AuthZ evaluation via PolicyEnforcer
- [ ] JWT AuthZ denial returns `403 Forbidden`
- [ ] JWT AuthZ approval proceeds with scoped read
- [ ] Hierarchy reads include `tenant_id` per group row
- [ ] Hierarchy result rows can contain different `tenant_id` values (multi-tenant scope)
- [ ] Membership reads do not include `tenant_id`
- [ ] AuthZ plugin can resolve `ResourceGroupReadHierarchy` from ClientHub and call `list_group_depth` in-process
- [ ] In-process SDK calls do not trigger AuthZ evaluation
- [ ] RG does not return allow/deny decisions (`cpt-cf-resource-group-constraint-no-authz-decision`)
- [ ] RG does not generate SQL fragments or access-scope objects (`cpt-cf-resource-group-constraint-no-sql-filter-generation`)
- [ ] Vendor plugin path forwards SecurityContext to plugin without modification

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — Integration Read Service is stateless; it routes queries and returns data.
- **Usability (UX)**: Not applicable — backend API and SDK only.
- **Compliance (COMPL)**: Not applicable — read-only data exposure; compliance is platform-level.
- **Performance**: Hierarchy reads leverage closure table indexes (`idx_rgc_ancestor_depth`, `idx_rgc_descendant_id`) established in Feature 1. No additional performance-specific logic in this feature beyond using existing index coverage.
- **Data Lifecycle**: Not applicable — this feature is read-only; data mutations are handled by Features 2–4.
- **Security (additional)**: MTLS allowlist enforcement is the primary security control in this feature. The allowlist restricts MTLS to a single read-only endpoint with minimal attack surface. AuthZ bypass for MTLS is intentional and required to resolve the circular dependency (AuthZ plugin cannot evaluate itself). JWT path uses standard platform AuthZ evaluation — no feature-specific security logic beyond endpoint routing.
