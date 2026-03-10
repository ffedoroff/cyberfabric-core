# Feature: MTLS Auth & Plugin Gateway

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-read-authz`
- [ ] `p2` - `cpt-cf-resource-group-feature-mtls-plugin-gateway`

**Status**: DEFERRED — blocked on platform MTLS infrastructure, AuthN module, and plugin architecture readiness

## 1. Feature Context

### 1.1 Overview

Add MTLS authentication path for out-of-process AuthZ plugin consumption, implement plugin gateway routing (built-in vs vendor-specific provider), and enforce endpoint-level allowlisting. This feature handles the **transport layer** for the AuthZ↔RG integration — the in-process data contract (`ResourceGroupReadHierarchy`) and AuthZ enforcement are already implemented in Features 5 and 7.

### 1.2 Purpose

In production deployments where the AuthZ plugin runs as a separate process (out-of-process mode), it needs to call RG via REST with MTLS authentication (not JWT, to avoid circular AuthZ evaluation). Additionally, vendors may provide their own RG data source — the plugin gateway enables transparent routing between built-in (local DB) and vendor-specific providers.

Addresses:
- `cpt-cf-resource-group-fr-dual-auth-modes` — MTLS authentication path (JWT path already done in Feature 5)
- `cpt-cf-resource-group-fr-integration-read-port` — out-of-process REST access via MTLS

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-authz-plugin-consumer` | Reads hierarchy data via MTLS REST endpoint (out-of-process deployment) |
| `cpt-cf-resource-group-actor-apps` | Vendor-specific RG plugin providing hierarchy data |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md)
- **AuthZ Architecture**: `docs/arch/authorization/DESIGN.md` — Deployment modes, trust model
- **Dependencies**:
  - [x] `p1` - `cpt-cf-resource-group-feature-authz-enforcement` (Feature 5)
  - [x] `p1` - `cpt-cf-resource-group-feature-authz-constraint-types` (Feature 7)
- **External Dependencies** (BLOCKING):
  - AuthN module — MTLS certificate verification, `SecurityContext` population
  - Platform infrastructure — MTLS CA bundle, endpoint allowlist configuration
  - GTS plugin discovery mechanism for `ResourceGroupReadPluginClient`

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

### Plugin Gateway Routing Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-plugin-gateway`

**Actor**: `cpt-cf-resource-group-actor-apps`

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

**Steps**:
1. [ ] - `p1` - **IF** request is MTLS authenticated - `inst-allowlist-1`
2. [ ] - `p1` - Check request path against allowlist: only `/groups/{id}/depth` (hierarchy endpoint) - `inst-allowlist-2`
3. [ ] - `p1` - **IF** path matches allowlist — **RETURN** allow - `inst-allowlist-3`
4. [ ] - `p1` - **ELSE** — **RETURN** `403 Forbidden` - `inst-allowlist-4`

### Authentication Mode Detection

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-auth-mode-detection`

**Steps**:
1. [ ] - `p1` - **IF** request has valid client TLS certificate — **RETURN** MTLS mode - `inst-auth-detect-1`
2. [ ] - `p1` - **IF** request has Authorization Bearer header — **RETURN** JWT mode - `inst-auth-detect-2`
3. [ ] - `p1` - **ELSE** — **RETURN** `401 Unauthorized` - `inst-auth-detect-3`

## 4. States (CDSL)

Not applicable. MTLS and plugin gateway operations are stateless.

## 5. Definitions of Done

### Dual Authentication Modes

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-dual-auth`

The system **MUST** support MTLS authentication path: hierarchy endpoint only, bypassing AuthZ with system `SecurityContext`. MTLS **MUST** enforce endpoint allowlist: only `GET /groups/{id}/depth` is reachable; all other endpoints return `403`.

### Plugin Gateway Routing

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-plugin-gateway`

The system **MUST** support pluggable hierarchy read providers: built-in (local data path) and vendor-specific (via `ResourceGroupReadPluginClient` trait). Provider selection **MUST** be configuration-driven.

## 6. Acceptance Criteria

- [ ] MTLS requests with valid client certificate reach `/groups/{id}/depth` endpoint
- [ ] MTLS requests to any other endpoint return `403 Forbidden`
- [ ] MTLS requests with invalid/untrusted certificate return `403 Forbidden`
- [ ] MTLS requests bypass AuthZ evaluation (no `PolicyEnforcer` call)
- [ ] MTLS `SecurityContext` is system-level (trusted service principal)
- [ ] Built-in plugin provider delegates to local `GroupService.list_group_depth()`
- [ ] Vendor plugin provider resolves via `ResourceGroupReadPluginClient` trait
- [ ] Plugin not found returns `ServiceUnavailable`

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — stateless operations.
- **Usability (UX)**: Not applicable — backend API only.
- **JWT Auth Path**: Implemented in Feature 5 (`cpt-cf-resource-group-feature-authz-enforcement`).
- **In-process ClientHub path**: Implemented in Feature 5 and 7.
- **Advanced constraint types**: Implemented in Feature 7.

## 8. Implementation Notes (DEFERRED)

Blocked on:
1. **AuthN module** — MTLS certificate verification middleware
2. **Platform infrastructure** — MTLS CA bundle, endpoint allowlist configuration
3. **GTS plugin discovery** — `ResourceGroupReadPluginClient` trait and vendor plugin registration

**Already implemented** (Features 1-5, 7):
- `ResourceGroupReadHierarchy` trait, ClientHub registration, in-process bypass
- JWT + PolicyEnforcer + AccessScope + SecureORM pipeline
- Static-authz-plugin hierarchy resolution via ClientHub
- Group ownership validation in AuthZ plugin
