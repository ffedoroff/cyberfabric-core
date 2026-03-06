# Feature: Type Management

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-type-management`
- [x] `p1` - `cpt-cf-resource-group-feature-type-management`

## 1. Feature Context

### 1.1 Overview

Implement the full resource group type lifecycle: create, get, list, update, and delete operations with code format validation, case-insensitive normalization, uniqueness enforcement via persistence constraint, deterministic seed path for pre-deployment type provisioning, and delete-if-unused guard that prevents type removal while entities reference it.

### 1.2 Purpose

Types define the hierarchy structure (e.g. `tenant` → `department` → `branch`). They are tenant-independent and globally available. Without a validated type model, entity creation (Feature 3) cannot enforce parent-child compatibility rules.

Addresses:
- `cpt-cf-resource-group-fr-manage-types` — type CRUD operations
- `cpt-cf-resource-group-fr-validate-type-code` — code format validation with case normalization
- `cpt-cf-resource-group-fr-reject-duplicate-type` — unique `code_ci` persistence constraint
- `cpt-cf-resource-group-fr-delete-type-only-if-empty` — delete guard when entities reference the type
- `cpt-cf-resource-group-fr-seed-types` — optional deployment-specific type data seeding (plugin data migration, manual DB admin, or RG API)
- `cpt-cf-resource-group-principle-dynamic-types` — runtime-configurable type rules

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-instance-administrator` | Manages type definitions via REST API, seeds types at deployment, deletes unused types |
| `cpt-cf-resource-group-actor-apps` | Programmatic type management via `ResourceGroupClient` SDK |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) — `cpt-cf-resource-group-feature-type-management`
- **OpenAPI**: [openapi.yaml](../openapi.yaml) — `/api/resource-group/v1/types`, `/api/resource-group/v1/types/{code}`
- **Migration**: [migration.sql](../migration.sql) — `resource_group_type` table and `idx_resource_group_type_code_lower`
- **Design Components**: `cpt-cf-resource-group-component-type-service`
- **Dependencies**: `cpt-cf-resource-group-feature-domain-foundation` (SDK, module shell, persistence adapter, DB schema)

## 2. Actor Flows (CDSL)

### Create Type Flow

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-create`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Type is created with validated code and parents array
- Type is returned with normalized code

**Error Scenarios**:
- Code format validation fails (invalid characters, empty, too long)
- Parents array is empty (minimum 1 element required)
- Duplicate code — case-insensitive conflict with existing type
- Referenced parent type codes do not exist (except empty string `""` which permits root placement)

**Steps**:
1. [x] - `p1` - Actor sends API: POST /api/resource-group/v1/types ({ code, parents }) - `inst-type-create-1`
2. [x] - `p1` - Invoke type code validation process (`cpt-cf-resource-group-algo-type-code-validation`) on `code` - `inst-type-create-2`
3. [x] - `p1` - **IF** code validation fails - `inst-type-create-3`
   1. [x] - `p1` - **RETURN** `Validation` error with field-level detail - `inst-type-create-3a`
4. [x] - `p1` - Validate `parents` array has at least 1 element - `inst-type-create-4`
5. [x] - `p1` - **IF** parents array is empty - `inst-type-create-5`
   1. [x] - `p1` - **RETURN** `Validation` error — parents must contain at least one element - `inst-type-create-5a`
6. [x] - `p1` - DB: INSERT INTO resource_group_type (code, parents, created) — normalized code, validated parents - `inst-type-create-6`
7. [x] - `p1` - **IF** unique constraint violation on `LOWER(code)` - `inst-type-create-7`
   1. [x] - `p1` - **RETURN** `TypeAlreadyExists` error with conflicting code - `inst-type-create-7a`
8. [x] - `p1` - **RETURN** created `ResourceGroupType` { code, parents } - `inst-type-create-8`

### Get Type By Code Flow

- [x] `p2` - **ID**: `cpt-cf-resource-group-flow-type-get`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Type is returned by exact code match

**Error Scenarios**:
- Type not found

**Steps**:
1. [x] - `p2` - Actor sends API: GET /api/resource-group/v1/types/{code} - `inst-type-get-1`
2. [x] - `p2` - DB: SELECT code, parents FROM resource_group_type WHERE code = :code - `inst-type-get-2`
3. [x] - `p2` - **IF** row found - `inst-type-get-3`
   1. [x] - `p2` - **RETURN** `ResourceGroupType` { code, parents } - `inst-type-get-3a`
4. [x] - `p2` - **ELSE** - `inst-type-get-4`
   1. [x] - `p2` - **RETURN** `NotFound` error - `inst-type-get-4a`

### List Types Flow

- [x] `p2` - **ID**: `cpt-cf-resource-group-flow-type-list`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Paginated list of types returned sorted by `code` ASC
- OData `$filter` on `code` field applied correctly

**Error Scenarios**:
- Invalid OData filter expression — `Validation` error

**Steps**:
1. [x] - `p2` - Actor sends API: GET /api/resource-group/v1/types?$filter={expr}&$top={n}&$skip={m} - `inst-type-list-1`
2. [x] - `p2` - Parse OData parameters: `$filter` on `code` (eq, ne, in), `$top` (1..300, default 50), `$skip` (default 0) - `inst-type-list-2`
3. [x] - `p2` - **IF** OData parse fails - `inst-type-list-3`
   1. [x] - `p2` - **RETURN** `Validation` error with parse detail - `inst-type-list-3a`
4. [x] - `p2` - DB: SELECT code, parents FROM resource_group_type WHERE {filter} ORDER BY code ASC LIMIT $top OFFSET $skip - `inst-type-list-4`
5. [x] - `p2` - **RETURN** `Page<ResourceGroupType>` { items, page_info: { top, skip } } - `inst-type-list-5`

### Update Type Flow

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-update`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Type parents array updated successfully

**Error Scenarios**:
- Type not found
- Parents array is empty

**Steps**:
1. [x] - `p1` - Actor sends API: PUT /api/resource-group/v1/types/{code} ({ parents }) - `inst-type-update-1`
2. [x] - `p1` - DB: SELECT code FROM resource_group_type WHERE code = :code - `inst-type-update-2`
3. [x] - `p1` - **IF** type not found - `inst-type-update-3`
   1. [x] - `p1` - **RETURN** `NotFound` error - `inst-type-update-3a`
4. [x] - `p1` - Validate `parents` array has at least 1 element - `inst-type-update-4`
5. [x] - `p1` - **IF** parents array is empty - `inst-type-update-5`
   1. [x] - `p1` - **RETURN** `Validation` error — parents must contain at least one element - `inst-type-update-5a`
6. [x] - `p1` - DB: UPDATE resource_group_type SET parents = :parents, modified = NOW() WHERE code = :code - `inst-type-update-6`
7. [x] - `p1` - **RETURN** updated `ResourceGroupType` { code, parents } - `inst-type-update-7`

### Delete Type Flow

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-delete`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- Type deleted when no entities reference it

**Error Scenarios**:
- Type not found
- Entities exist that reference this type — delete blocked

**Steps**:
1. [x] - `p1` - Actor sends API: DELETE /api/resource-group/v1/types/{code} - `inst-type-delete-1`
2. [x] - `p1` - DB: SELECT code FROM resource_group_type WHERE code = :code - `inst-type-delete-2`
3. [x] - `p1` - **IF** type not found - `inst-type-delete-3`
   1. [x] - `p1` - **RETURN** `NotFound` error - `inst-type-delete-3a`
4. [x] - `p1` - Invoke type delete usage guard (`cpt-cf-resource-group-algo-type-delete-guard`) for :code - `inst-type-delete-4`
5. [x] - `p1` - **IF** usage guard rejects (entities reference this type) - `inst-type-delete-5`
   1. [x] - `p1` - **RETURN** `ConflictActiveReferences` error with entity count - `inst-type-delete-5a`
6. [x] - `p1` - DB: DELETE FROM resource_group_type WHERE code = :code - `inst-type-delete-6`
7. [x] - `p1` - **RETURN** success (204 No Content) - `inst-type-delete-7`

### Seed Types Flow

- [x] `p1` - **ID**: `cpt-cf-resource-group-flow-type-seed`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- All seed type definitions are upserted deterministically
- Existing types with matching codes are updated (parents overwritten)
- New types are created with normalized codes

**Error Scenarios**:
- Seed data contains invalid type code — seed aborts with validation error

**Steps**:
1. [x] - `p1` - Instance Administrator triggers seed operation (pre-deployment step with system SecurityContext) - `inst-type-seed-1`
2. [x] - `p1` - **FOR EACH** type definition in seed data - `inst-type-seed-2`
   1. [x] - `p1` - Invoke type code validation (`cpt-cf-resource-group-algo-type-code-validation`) on type code - `inst-type-seed-2a`
   2. [x] - `p1` - **IF** code validation fails - `inst-type-seed-2b`
      1. [x] - `p1` - **RETURN** `Validation` error — seed aborts - `inst-type-seed-2b1`
   3. [x] - `p1` - DB: UPSERT resource_group_type (code, parents) — insert or update parents on conflict - `inst-type-seed-2c`
3. [x] - `p1` - **RETURN** seed complete — all types provisioned - `inst-type-seed-3`

## 3. Processes / Business Logic (CDSL)

### Type Code Validation

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-type-code-validation`

**Input**: Raw type code string from create or seed request

**Output**: Normalized code string or validation error

**Steps**:
1. [x] - `p1` - **IF** code is empty or blank - `inst-codeval-1`
   1. [x] - `p1` - **RETURN** `Validation` error — code must not be empty - `inst-codeval-1a`
2. [x] - `p1` - **IF** code exceeds maximum length (aligned with `resource_group_type.code` TEXT column practical limit) - `inst-codeval-2`
   1. [x] - `p1` - **RETURN** `Validation` error — code exceeds maximum length - `inst-codeval-2a`
3. [x] - `p1` - **IF** code contains invalid characters (only lowercase alphanumeric, hyphens, underscores allowed after normalization) - `inst-codeval-3`
   1. [x] - `p1` - **RETURN** `Validation` error — code contains invalid characters - `inst-codeval-3a`
4. [x] - `p1` - Apply case-insensitive normalization (lowercase the code) - `inst-codeval-4`
5. [x] - `p1` - **RETURN** normalized code string - `inst-codeval-5`

### Type Delete Usage Guard

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-type-delete-guard`

**Input**: Type code to delete

**Output**: Allow delete or reject with active reference count

**Steps**:
1. [x] - `p1` - DB: SELECT COUNT(*) FROM resource_group WHERE group_type = :code - `inst-delguard-1`
2. [x] - `p1` - **IF** count > 0 - `inst-delguard-2`
   1. [x] - `p1` - **RETURN** reject — entities reference this type (include count) - `inst-delguard-2a`
3. [x] - `p1` - **ELSE** - `inst-delguard-3`
   1. [x] - `p1` - **RETURN** allow — type has no active references - `inst-delguard-3a`

## 4. States (CDSL)

Not applicable. Resource group types have no lifecycle states — they exist or they do not. Creation and deletion are atomic operations.

## 5. Definitions of Done

### Type Create with Code Validation

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-create`

The system **MUST** create a resource group type via `POST /api/resource-group/v1/types` accepting `{ code, parents }`. The code **MUST** be validated for format, length, and allowed characters, then case-insensitive normalized before persistence. The parents array **MUST** contain at least one element. Empty string `""` in parents permits root placement (entity of this type can have no parent). The created type **MUST** be returned as `ResourceGroupType` with the normalized code.

**Implements**:
- `cpt-cf-resource-group-flow-type-create`
- `cpt-cf-resource-group-algo-type-code-validation`

**Touches**:
- API: `POST /api/resource-group/v1/types`
- DB: `resource_group_type`
- Entities: `ResourceGroupType`

### Type Read Operations

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-read`

The system **MUST** retrieve a single type by code via `GET /api/resource-group/v1/types/{code}` returning `ResourceGroupType` or `NotFound`. The system **MUST** list types via `GET /api/resource-group/v1/types` with OData `$filter` on `code` field (eq, ne, in), `$top` (1..300, default 50), `$skip` (default 0). Results **MUST** be sorted by `code` ASC for deterministic pagination.

**Implements**:
- `cpt-cf-resource-group-flow-type-get`
- `cpt-cf-resource-group-flow-type-list`

**Touches**:
- API: `GET /api/resource-group/v1/types`, `GET /api/resource-group/v1/types/{code}`
- DB: `resource_group_type`
- Entities: `ResourceGroupType`

### Type Update

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-update`

The system **MUST** update a type's parents array via `PUT /api/resource-group/v1/types/{code}` accepting `{ parents }`. The type **MUST** exist or return `NotFound`. The parents array **MUST** contain at least one element. The `modified` timestamp **MUST** be set on update. The updated type **MUST** be returned as `ResourceGroupType`.

**Implements**:
- `cpt-cf-resource-group-flow-type-update`

**Touches**:
- API: `PUT /api/resource-group/v1/types/{code}`
- DB: `resource_group_type`
- Entities: `ResourceGroupType`

### Type Delete with Usage Guard

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-delete`

The system **MUST** delete a type via `DELETE /api/resource-group/v1/types/{code}`. Before deletion, the system **MUST** check whether any `resource_group` rows reference this type via `group_type`. If references exist, the system **MUST** reject the delete with `ConflictActiveReferences` error including the entity count. If no references exist, the type **MUST** be deleted and the response **MUST** be `204 No Content`.

**Implements**:
- `cpt-cf-resource-group-flow-type-delete`
- `cpt-cf-resource-group-algo-type-delete-guard`

**Touches**:
- API: `DELETE /api/resource-group/v1/types/{code}`
- DB: `resource_group_type`, `resource_group` (read for usage check)
- Entities: `ResourceGroupType`

### Type Code Uniqueness Enforcement

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-uniqueness`

The system **MUST** enforce case-insensitive uniqueness for type codes via the `idx_resource_group_type_code_lower` unique index on `LOWER(code)`. When a create operation conflicts with an existing type code (case-insensitive match), the persistence constraint violation **MUST** be deterministically mapped to `TypeAlreadyExists` error with the conflicting code. The mapping **MUST** distinguish this constraint violation from other DB errors.

**Implements**:
- `cpt-cf-resource-group-flow-type-create`

**Touches**:
- DB: `resource_group_type` (unique index `idx_resource_group_type_code_lower`)

### Seed Types Path

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-type-seed`

Type data seeding is **optional** and deployment-specific. It can be performed via plugin data migration, manual database administration, or RG API calls. When performed via the module's seed path, seed operations **MUST** run with system `SecurityContext` (bypassing AuthZ). For each type in seed data: validate code format, normalize case, then insert or update parents on conflict. Seed **MUST** abort on the first validation error. The seed path **MUST** produce identical results when re-run against the same data (idempotent).

**Implements**:
- `cpt-cf-resource-group-flow-type-seed`
- `cpt-cf-resource-group-algo-type-code-validation`

**Touches**:
- DB: `resource_group_type`
- Entities: `ResourceGroupType`

## 6. Acceptance Criteria

- [x] `POST /api/resource-group/v1/types` creates a type and returns `201` with `ResourceGroupType`
- [x] Type code is normalized to lowercase before persistence
- [x] Empty code is rejected with `Validation` error
- [x] Code with invalid characters is rejected with `Validation` error
- [x] Duplicate code (case-insensitive) is rejected with `TypeAlreadyExists` error
- [x] Empty parents array is rejected with `Validation` error
- [x] `GET /api/resource-group/v1/types/{code}` returns the type or `404`
- [x] `GET /api/resource-group/v1/types` returns paginated types sorted by `code` ASC
- [x] OData `$filter` on `code` works for eq, ne, in operators
- [x] `$top` limits page size (1..300), `$skip` offsets from beginning
- [x] `PUT /api/resource-group/v1/types/{code}` updates parents and returns updated type
- [x] Update sets `modified` timestamp
- [x] Update on non-existent type returns `404`
- [x] `DELETE /api/resource-group/v1/types/{code}` returns `204` when no entities reference the type
- [x] Delete is rejected with `ConflictActiveReferences` when `resource_group` rows reference the type
- [x] Delete on non-existent type returns `404`
- [x] Seed types path upserts definitions deterministically and is idempotent
- [x] Seed aborts on first invalid type code
- [x] Seed runs with system SecurityContext (AuthZ bypass)
- [x] Parents array with empty string `""` permits root placement for entities of this type

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — types have no lifecycle states; they exist or they do not.
- **Usability (UX)**: Not applicable — backend API only, no frontend.
- **Compliance (COMPL)**: Not applicable — types do not contain regulated or personal data.
- **Operations (OPS)**: Standard platform patterns apply; no type-specific deployment or observability requirements.
- **Performance**: Type table is small (~50 rows at production scale per DESIGN section 4.1). No performance-critical paths; standard index coverage sufficient.
- **External Integrations**: Not applicable — type management is internal to RG module, no external system calls.
