# Feature: GTS Type Management

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-type-management`

- [x] `p1` - `cpt-cf-resource-group-feature-type-management`

<!-- toc -->

- [1. Feature Context](#1-feature-context)
  - [1.1 Overview](#11-overview)
  - [1.2 Purpose](#12-purpose)
  - [1.3 Actors](#13-actors)
  - [1.4 References](#14-references)
- [2. Actor Flows (CDSL)](#2-actor-flows-cdsl)
  - [Create Type](#create-type)
  - [Update Type](#update-type)
  - [Delete Type](#delete-type)
- [3. Processes / Business Logic (CDSL)](#3-processes--business-logic-cdsl)
  - [Type Input Validation](#type-input-validation)
  - [Hierarchy Safety Check for Type Update](#hierarchy-safety-check-for-type-update)
  - [Type Seeding](#type-seeding)
- [4. States (CDSL)](#4-states-cdsl)
- [5. Definitions of Done](#5-definitions-of-done)
  - [Type Service CRUD](#type-service-crud)
  - [Type REST Handlers](#type-rest-handlers)
  - [Type Data Seeding](#type-data-seeding)
- [6. Acceptance Criteria](#6-acceptance-criteria)

<!-- /toc -->

## 1. Feature Context

### 1.1 Overview

Full lifecycle management for GTS resource group types: create, list, get, update, and delete type definitions with code format validation, case-insensitive uniqueness enforcement, hierarchy-safe update checks, delete-if-unused policy, and idempotent type seeding for deployment bootstrapping.

### 1.2 Purpose

Types define the structural rules for the resource group hierarchy — which parent-child relationships are allowed, which resource types can be members, and whether a type permits root placement. This feature enables runtime-configurable type governance through API and seed data.

**Requirements**: `cpt-cf-resource-group-fr-manage-types`, `cpt-cf-resource-group-fr-validate-type-code`, `cpt-cf-resource-group-fr-reject-duplicate-type`, `cpt-cf-resource-group-fr-seed-types`, `cpt-cf-resource-group-fr-validate-type-update-hierarchy`, `cpt-cf-resource-group-fr-delete-type-only-if-empty`

**Principles**: `cpt-cf-resource-group-principle-dynamic-types`

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-instance-administrator` | Manages type definitions via REST API, operates type seeding |
| `cpt-cf-resource-group-actor-apps` | Programmatic type management via `ResourceGroupClient` SDK |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md) — sections 5.1, 8.1
- **Design**: [DESIGN.md](../DESIGN.md) — sections 3.1, 3.2 (Type Service), 3.3, 3.7 (gts_type tables)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) entry 2.2
- **Dependencies**: Feature 0001 — SDK traits, persistence adapter, error mapping

## 2. Actor Flows (CDSL)

### Create Type

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-mgmt-create-type`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Type is created with schema_id, allowed_parents, allowed_memberships, and metadata_schema
- Type is immediately available for group creation

**Error Scenarios**:
- Invalid GTS type path format → Validation error
- Duplicate schema_id → TypeAlreadyExists
- Referenced allowed_parents type does not exist → Validation error
- Referenced allowed_memberships type does not exist → Validation error
- Placement invariant violated (not can_be_root AND no allowed_parents) → Validation error

**Steps**:
1. [x] - `p1` - Actor sends POST /api/types-registry/v1/types with type definition payload - `inst-create-type-1`
2. [x] - `p1` - Validate GTS type path format via `GtsTypePath` value object - `inst-create-type-2`
3. [x] - `p1` - Validate placement invariant: `can_be_root OR len(allowed_parents) >= 1` - `inst-create-type-3`
4. [x] - `p1` - **IF** allowed_parents is non-empty - `inst-create-type-4`
   1. [x] - `p1` - DB: SELECT id FROM gts_type WHERE schema_id IN (allowed_parents) — verify all referenced parent types exist - `inst-create-type-4a`
   2. [x] - `p1` - **IF** any parent type not found → **RETURN** Validation error with missing type paths - `inst-create-type-4b`
5. [x] - `p1` - **IF** allowed_memberships is non-empty - `inst-create-type-5`
   1. [x] - `p1` - DB: SELECT id FROM gts_type WHERE schema_id IN (allowed_memberships) — verify all referenced membership types exist - `inst-create-type-5a`
   2. [x] - `p1` - **IF** any membership type not found → **RETURN** Validation error with missing type paths - `inst-create-type-5b`
6. [x] - `p1` - Resolve GTS type path to SMALLINT surrogate ID at persistence boundary - `inst-create-type-6`
7. [x] - `p1` - DB: INSERT INTO gts_type (schema_id, metadata_schema) — with uniqueness constraint on schema_id - `inst-create-type-7`
8. [x] - `p1` - **IF** unique constraint violation → **RETURN** TypeAlreadyExists with conflicting schema_id - `inst-create-type-8`
9. [x] - `p1` - DB: INSERT INTO gts_type_allowed_parent (type_id, parent_type_id) for each allowed parent - `inst-create-type-9`
10. [x] - `p1` - DB: INSERT INTO gts_type_allowed_membership (type_id, membership_type_id) for each allowed membership - `inst-create-type-10`
11. [x] - `p1` - **RETURN** created ResourceGroupType with schema_id, allowed_parents, allowed_memberships, can_be_root, metadata_schema - `inst-create-type-11`

### Update Type

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-mgmt-update-type`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Type definition updated (allowed_parents, allowed_memberships, metadata_schema)
- Existing groups remain valid under new rules

**Error Scenarios**:
- Type not found → NotFound
- Removing allowed_parent that is in use by existing groups → AllowedParentsViolation
- Setting can_be_root=false when root groups of this type exist → AllowedParentsViolation
- Referenced type does not exist → Validation error
- Placement invariant violated → Validation error

**Steps**:
1. [x] - `p1` - Actor sends PUT /api/types-registry/v1/types/{code} with updated definition - `inst-update-type-1`
2. [x] - `p1` - DB: SELECT FROM gts_type WHERE schema_id = {code} — load existing type - `inst-update-type-2`
3. [x] - `p1` - **IF** type not found → **RETURN** NotFound - `inst-update-type-3`
4. [x] - `p1` - Validate placement invariant on new values - `inst-update-type-4`
5. [x] - `p1` - Validate all referenced allowed_parents and allowed_memberships types exist - `inst-update-type-5`
6. [x] - `p1` - Invoke hierarchy safety check algorithm for allowed_parents and can_be_root changes - `inst-update-type-6`
7. [x] - `p1` - **IF** hierarchy safety check fails → **RETURN** AllowedParentsViolation with violating group details - `inst-update-type-7`
8. [x] - `p1` - DB: DELETE FROM gts_type_allowed_parent WHERE type_id = {id} — clear old parents - `inst-update-type-8`
9. [x] - `p1` - DB: INSERT INTO gts_type_allowed_parent — insert new parents - `inst-update-type-9`
10. [x] - `p1` - DB: DELETE FROM gts_type_allowed_membership WHERE type_id = {id} — clear old memberships - `inst-update-type-10`
11. [x] - `p1` - DB: INSERT INTO gts_type_allowed_membership — insert new memberships - `inst-update-type-11`
12. [x] - `p1` - DB: UPDATE gts_type SET metadata_schema = {new}, updated_at = now() - `inst-update-type-12`
13. [x] - `p1` - **RETURN** updated ResourceGroupType - `inst-update-type-13`

### Delete Type

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-mgmt-delete-type`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Unused type is deleted along with its junction table entries

**Error Scenarios**:
- Type not found → NotFound
- At least one group of this type exists → ConflictActiveReferences

**Steps**:
1. [x] - `p1` - Actor sends DELETE /api/types-registry/v1/types/{code} - `inst-delete-type-1`
2. [x] - `p1` - DB: SELECT id FROM gts_type WHERE schema_id = {code} - `inst-delete-type-2`
3. [x] - `p1` - **IF** type not found → **RETURN** NotFound - `inst-delete-type-3`
4. [x] - `p1` - DB: SELECT COUNT(*) FROM resource_group WHERE gts_type_id = {type_id} - `inst-delete-type-4`
5. [x] - `p1` - **IF** count > 0 → **RETURN** ConflictActiveReferences with entity count - `inst-delete-type-5`
6. [x] - `p1` - DB: DELETE FROM gts_type WHERE id = {type_id} — CASCADE deletes junction table rows - `inst-delete-type-6`
7. [x] - `p1` - **RETURN** success (204 No Content) - `inst-delete-type-7`

## 3. Processes / Business Logic (CDSL)

### Type Input Validation

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-type-mgmt-validate-type-input`

**Input**: Type create/update payload (`schema_id`, `allowed_parents`, `allowed_memberships`, `can_be_root`, `metadata_schema`)

**Output**: Validated type definition or validation error with field-level details

**Steps**:
1. [x] - `p1` - Validate `schema_id` via GtsTypePath value object (format, length, non-empty) - `inst-val-input-1`
2. [x] - `p1` - **IF** `schema_id` does not have RG type prefix `gts.x.system.rg.type.v1~` - `inst-val-input-2`
   1. [x] - `p1` - **RETURN** Validation error: "Type schema_id must have RG type prefix" - `inst-val-input-2a`
3. [x] - `p1` - Validate placement invariant: `can_be_root == true OR len(allowed_parents) >= 1` - `inst-val-input-3`
4. [x] - `p1` - **IF** invariant violated - `inst-val-input-4`
   1. [x] - `p1` - **RETURN** Validation error: "Type must allow root placement or have at least one allowed parent" - `inst-val-input-4a`
5. [x] - `p1` - **FOR EACH** parent_path in allowed_parents - `inst-val-input-5`
   1. [x] - `p1` - Validate parent_path has RG type prefix `gts.x.system.rg.type.v1~` - `inst-val-input-5a`
   2. [x] - `p1` - Verify parent_path exists in gts_type table - `inst-val-input-5b`
6. [x] - `p1` - **FOR EACH** membership_path in allowed_memberships - `inst-val-input-6`
   1. [x] - `p1` - Validate membership_path is a valid GtsTypePath (no RG prefix requirement) - `inst-val-input-6a`
   2. [x] - `p1` - Verify membership_path exists in gts_type table - `inst-val-input-6b`
7. [x] - `p1` - **IF** metadata_schema provided, validate it is a valid JSON Schema via `jsonschema::validator_for()`. Returns validation error if the schema cannot be compiled. - `inst-val-input-7`
8. [x] - `p1` - **RETURN** validated type definition - `inst-val-input-8`

### Hierarchy Safety Check for Type Update

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-type-mgmt-check-hierarchy-safety`

**Input**: Existing type definition, proposed new `allowed_parents` and `can_be_root` values

**Output**: Pass or AllowedParentsViolation with conflicting group details

**Steps**:
1. [x] - `p1` - Compute removed parent types: `old_allowed_parents - new_allowed_parents` - `inst-hier-check-1`
2. [x] - `p1` - **FOR EACH** removed_parent_type in removed set - `inst-hier-check-2`
   1. [x] - `p1` - DB: SELECT rg.id, rg.name FROM resource_group rg JOIN resource_group parent ON rg.parent_id = parent.id WHERE rg.gts_type_id = {this_type_id} AND parent.gts_type_id = {removed_parent_type_id} - `inst-hier-check-2a`
   2. [x] - `p1` - **IF** any groups found → collect as violations - `inst-hier-check-2b`
3. [x] - `p1` - **IF** can_be_root changed from true to false - `inst-hier-check-3`
   1. [x] - `p1` - DB: SELECT id, name FROM resource_group WHERE gts_type_id = {this_type_id} AND parent_id IS NULL - `inst-hier-check-3a`
   2. [x] - `p1` - **IF** any root groups found → collect as violations - `inst-hier-check-3b`
4. [x] - `p1` - **IF** violations collected → **RETURN** AllowedParentsViolation with violating group IDs, names, and constraint details - `inst-hier-check-4`
5. [x] - `p1` - **RETURN** pass - `inst-hier-check-5`

### Type Seeding

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-type-mgmt-seed-types`

**Input**: List of type seed definitions from deployment configuration

**Output**: Seed result (types created, types updated, unchanged count)

**Steps**:
1. [x] - `p1` - Load seed definitions from configuration source - `inst-seed-1`
2. [x] - `p1` - **FOR EACH** seed_def in seed definitions (types are independent — SHOULD be executed in parallel via `JoinSet` for throughput) - `inst-seed-2`
   1. [x] - `p1` - DB: SELECT FROM gts_type WHERE schema_id = {seed_def.schema_id} - `inst-seed-2a`
   2. [x] - `p1` - **IF** type exists AND definition matches → skip (unchanged) - `inst-seed-2b`
   3. [x] - `p1` - **IF** type exists AND definition differs → update type via update flow - `inst-seed-2c`
   4. [x] - `p1` - **IF** type does not exist → create type via create flow - `inst-seed-2d`
3. [x] - `p1` - **RETURN** seed result: {created: N, updated: N, unchanged: N} - `inst-seed-3`

## 4. States (CDSL)

Not applicable. Types are configuration entities without lifecycle states. A type either exists or does not exist — there are no intermediate states or transitions. Type availability is governed by create/delete operations, not state machines.

## 5. Definitions of Done

### Type Service CRUD

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-mgmt-service-crud`

The system **MUST** implement a Type Service that provides create, list, get, update, and delete operations for GTS resource group types with full domain validation.

**Required behavior**:
- Create: validate input, check uniqueness, persist type with junction table entries, return created type
- List: paginated query with OData `$filter` on `code` field, cursor-based pagination
- Get: retrieve single type by schema_id (GTS type path), return NotFound if absent
- Update: validate input, check hierarchy safety against existing groups, update definition atomically
- Delete: check for existing groups of this type, reject if in use, delete with cascade on junction tables

**Implements**:
- `cpt-cf-resource-group-flow-type-mgmt-create-type`
- `cpt-cf-resource-group-flow-type-mgmt-update-type`
- `cpt-cf-resource-group-flow-type-mgmt-delete-type`
- `cpt-cf-resource-group-algo-type-mgmt-validate-type-input`
- `cpt-cf-resource-group-algo-type-mgmt-check-hierarchy-safety`

**Constraints**: `cpt-cf-resource-group-constraint-surrogate-ids-internal`

**Touches**:
- DB: `gts_type`, `gts_type_allowed_parent`, `gts_type_allowed_membership`
- Entities: `ResourceGroupType`, `GtsTypePath`

### Type REST Handlers

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-mgmt-rest-handlers`

The system **MUST** implement REST endpoint handlers for type management under `/api/types-registry/v1/types` using OperationBuilder.

**Required endpoints**:
- `GET /types` — list types with OData `$filter` (field: `code`, operators: `eq`, `ne`, `in`) and cursor-based pagination (`cursor`, `limit`)
- `POST /types` — create type, return 201 Created with type body
- `GET /types/{code}` — get type by GTS type path, return 404 if not found
- `PUT /types/{code}` — update type, return 200 OK with updated body
- `DELETE /types/{code}` — delete type, return 204 No Content

All endpoints **MUST** resolve GTS type paths to SMALLINT surrogate IDs at the persistence boundary. No surrogate IDs in request or response bodies.

**Implements**:
- `cpt-cf-resource-group-flow-type-mgmt-create-type`
- `cpt-cf-resource-group-flow-type-mgmt-update-type`
- `cpt-cf-resource-group-flow-type-mgmt-delete-type`

**Touches**:
- API: `GET/POST /api/types-registry/v1/types`, `GET/PUT/DELETE /api/types-registry/v1/types/{code}`

### Type Data Seeding

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-mgmt-seeding`

The system **MUST** provide an idempotent type seeding mechanism for deployment bootstrapping.

**Required behavior**:
- Accept a list of type seed definitions from deployment configuration
- For each seed: create if missing, update if definition differs, skip if unchanged
- Seeding runs as a pre-deployment step with system SecurityContext (bypasses AuthZ)
- Repeated runs produce the same result (idempotent)
- Seeding validates all type constraints (format, placement invariant, referenced types)

**Implements**:
- `cpt-cf-resource-group-algo-type-mgmt-seed-types`

**Touches**:
- DB: `gts_type`, `gts_type_allowed_parent`, `gts_type_allowed_membership`

## 6. Acceptance Criteria

- [x] Type with valid schema_id and allowed_parents is created and persisted with junction table entries
- [x] Creating type with duplicate schema_id returns `TypeAlreadyExists` (409)
- [x] Creating type with invalid GTS type path format returns validation error (400) with field details
- [x] Creating type without `can_be_root` and without `allowed_parents` returns validation error (placement invariant)
- [x] Updating type to remove allowed_parent that is in use by existing groups returns `AllowedParentsViolation` (409)
- [x] Updating type to set `can_be_root=false` when root groups exist returns `AllowedParentsViolation` (409)
- [x] Updating type to add new allowed_parent succeeds when no existing groups violate new rules
- [x] Deleting unused type succeeds (204) and removes junction table entries via CASCADE
- [x] Deleting type with existing groups returns `ConflictActiveReferences` (409) with response body including entity count so the caller can display what prevents deletion
- [x] Type seeding creates missing types, updates changed types, skips unchanged types (idempotent)
- [x] List types endpoint supports OData `$filter` on `code` field with `eq`, `ne`, `in` operators
- [x] All REST responses use GTS type paths — no SMALLINT surrogate IDs exposed
- [x] Creating type with invalid metadata_schema (not valid JSON Schema) returns validation error (400)
