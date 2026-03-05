# Feature: Type Management

- [ ] `p1` - **ID**: `cpt-cf-resource-group-featstatus-type-management`
- [x] `p1` - `cpt-cf-resource-group-feature-type-management`

## 1. Feature Context

### 1.1 Parent Decomposition

- **DECOMPOSITION**: [../DECOMPOSITION.md](../DECOMPOSITION.md)
- **Feature ID**: `cpt-cf-resource-group-feature-type-management`

### 1.2 Purpose

Implement the full type lifecycle — CRUD operations, code format validation with case-insensitive normalization, uniqueness enforcement, seed path, and delete-if-unused guard.

### 1.3 Scope

- Type CRUD (create/get/list/update/delete)
- Code validation (format, length, case-insensitive normalization)
- Uniqueness enforcement (`code_ci` constraint)
- Seed types path
- Delete guard (only if no entities reference the type)
- REST endpoints: `/types`, `/types/{code}`

### 1.4 Design Components

- `cpt-cf-resource-group-component-type-service`
