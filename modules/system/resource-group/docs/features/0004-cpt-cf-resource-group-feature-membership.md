# Feature: Membership Management

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-membership`
- [x] `p2` - `cpt-cf-resource-group-feature-membership`

## 1. Feature Context

### 1.1 Parent Decomposition

- **DECOMPOSITION**: [../DECOMPOSITION.md](../DECOMPOSITION.md)
- **Feature ID**: `cpt-cf-resource-group-feature-membership`

### 1.2 Purpose

Implement membership CRUD with tenant-scoped ownership-graph semantics, seed path, and indexed lookups by group and resource.

### 1.3 Scope

- Add/remove/list membership links
- Tenant scope validation (ownership-graph profile)
- Active-reference guard
- Seed memberships path
- Reverse lookups (by resource_type + resource_id)
- REST endpoints: `/memberships`, `/memberships/{group_id}/{resource_type}/{resource_id}`

### 1.4 Design Components

- `cpt-cf-resource-group-component-membership-service`
