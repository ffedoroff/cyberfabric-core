# Feature: Integration Read & AuthZ Interop

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-integration-read`
- [x] `p2` - `cpt-cf-resource-group-feature-integration-read`

## 1. Feature Context

### 1.1 Parent Decomposition

- **DECOMPOSITION**: [../DECOMPOSITION.md](../DECOMPOSITION.md)
- **Feature ID**: `cpt-cf-resource-group-feature-integration-read`

### 1.2 Purpose

Expose the read-only `ResourceGroupReadHierarchy` contract for AuthZ plugin consumption, implement plugin gateway routing, and enforce JWT/MTLS dual authentication with endpoint-level allowlisting.

### 1.3 Scope

- `ResourceGroupReadHierarchy` trait implementation
- `ResourceGroupReadPluginClient` for vendor-specific delegation
- Plugin gateway routing (built-in vs vendor-specific)
- MTLS authentication and endpoint allowlist
- JWT authentication with AuthZ evaluation
- Tenant projection rules for integration reads
- Caller identity propagation

### 1.4 Design Components

- `cpt-cf-resource-group-component-integration-read-service`
