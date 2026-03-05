# Feature: Entity & Hierarchy Management

- [ ] `p1` - **ID**: `cpt-cf-resource-group-featstatus-entity-hierarchy`
- [x] `p1` - `cpt-cf-resource-group-feature-entity-hierarchy`

## 1. Feature Context

### 1.1 Parent Decomposition

- **DECOMPOSITION**: [../DECOMPOSITION.md](../DECOMPOSITION.md)
- **Feature ID**: `cpt-cf-resource-group-feature-entity-hierarchy`

### 1.2 Purpose

Implement entity CRUD with strict forest topology enforcement, closure-table hierarchy maintenance, subtree operations (move/delete), depth-based hierarchy queries, and query profile enforcement.

### 1.3 Scope

- Entity create/get/update/move/delete
- Forest invariant enforcement (single parent, cycle prevention)
- Parent type compatibility validation
- Closure table maintenance
- Ancestor/descendant queries ordered by depth
- Subtree move and force delete
- Query profile enforcement (max_depth/max_width)
- Seed groups path
- REST endpoints: `/groups`, `/groups/{id}`, `/groups/{id}/depth`

### 1.4 Design Components

- `cpt-cf-resource-group-component-entity-service`
- `cpt-cf-resource-group-component-hierarchy-service`
