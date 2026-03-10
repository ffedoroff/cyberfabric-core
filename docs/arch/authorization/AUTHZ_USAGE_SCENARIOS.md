# Authorization Usage Scenarios

This document demonstrates the authorization model through concrete examples.
Each scenario shows the full flow: HTTP request → PDP evaluation → SQL execution.

For the core authorization design, see [DESIGN.md](./DESIGN.md).

All examples use a Task Management domain:
- **Resource:** `tasks` table with `id`, `owner_tenant_id`, `owner_id`, `title`, `status`
- **Owner:** `owner_id` references the subject (user) who owns/is assigned the task
- **Resource Groups:** Projects (tasks belong to projects)
- **Tenant Model:** Hierarchical multi-tenancy — see [TENANT_MODEL.md](./TENANT_MODEL.md) for details on topology and closure tables

---

## Table of Contents

- [Authorization Usage Scenarios](#authorization-usage-scenarios)
  - [Table of Contents](#table-of-contents)
  - [Projection Tables](#projection-tables)
    - [What Are Projection Tables?](#what-are-projection-tables)
    - [Choosing Projection Tables](#choosing-projection-tables)
    - [Capabilities and PDP Response](#capabilities-and-pdp-response)
    - [When No Projection Tables Are Needed](#when-no-projection-tables-are-needed)
    - [When to Use Tenant Hierarchy via `resource_group_closure`](#when-to-use-tenant-hierarchy-via-resource_group_closure)
    - [When to Use `resource_group_membership`](#when-to-use-resource_group_membership)
    - [When to Use `resource_group_closure`](#when-to-use-resource_group_closure)
    - [Combinations Summary](#combinations-summary)
  - [Scenarios](#scenarios)
    - [With Tenant Hierarchy (resource\_group\_closure)](#with-tenant-hierarchy-resource_group_closure)
      - [S01: LIST, tenant subtree, PEP has tenant hierarchy](#s01-list-tenant-subtree-pep-has-tenant-hierarchy)
      - [S02: GET, tenant subtree, PEP has tenant hierarchy](#s02-get-tenant-subtree-pep-has-tenant-hierarchy)
      - [S03: UPDATE, tenant subtree, PEP has tenant hierarchy](#s03-update-tenant-subtree-pep-has-tenant-hierarchy)
      - [S04: DELETE, tenant subtree, PEP has tenant hierarchy](#s04-delete-tenant-subtree-pep-has-tenant-hierarchy)
      - [S05: CREATE, PEP-provided tenant context](#s05-create-pep-provided-tenant-context)
      - [S06: CREATE, subject tenant context (no explicit tenant in API)](#s06-create-subject-tenant-context-no-explicit-tenant-in-api)
    - [Without Local Closure Table](#without-local-closure-table)
      - [S07: LIST, tenant subtree, PEP without local closure table](#s07-list-tenant-subtree-pep-without-local-closure-table)
      - [S08: GET, tenant subtree, PEP without local closure table](#s08-get-tenant-subtree-pep-without-local-closure-table)
      - [S09: UPDATE, tenant subtree, PEP without local closure table (prefetch)](#s09-update-tenant-subtree-pep-without-local-closure-table-prefetch)
      - [S10: DELETE, tenant subtree, PEP without local closure table (prefetch)](#s10-delete-tenant-subtree-pep-without-local-closure-table-prefetch)
      - [S11: CREATE, PEP without local closure table](#s11-create-pep-without-local-closure-table)
      - [S12: GET, context tenant only (no subtree)](#s12-get-context-tenant-only-no-subtree)
    - [Resource Groups](#resource-groups)
      - [S13: LIST, group membership, PEP has resource\_group\_membership](#s13-list-group-membership-pep-has-resource_group_membership)
      - [S14: LIST, group subtree, PEP has resource\_group\_closure](#s14-list-group-subtree-pep-has-resource_group_closure)
      - [S15: UPDATE, group membership, PEP has resource\_group\_membership](#s15-update-group-membership-pep-has-resource_group_membership)
      - [S16: UPDATE, group subtree, PEP has resource\_group\_closure](#s16-update-group-subtree-pep-has-resource_group_closure)
      - [S17: GET, group membership, PEP without resource\_group\_membership](#s17-get-group-membership-pep-without-resource_group_membership)
      - [S18: LIST, group subtree, PEP has membership but no closure](#s18-list-group-subtree-pep-has-membership-but-no-closure)
    - [Advanced Patterns](#advanced-patterns)
      - [S19: LIST, tenant subtree and group membership (AND)](#s19-list-tenant-subtree-and-group-membership-and)
      - [S20: LIST, tenant subtree and group subtree](#s20-list-tenant-subtree-and-group-subtree)
      - [S21: LIST, multiple access paths (OR)](#s21-list-multiple-access-paths-or)
      - [S22: Access denied](#s22-access-denied)
    - [Subject Owner-Based Access](#subject-owner-based-access)
      - [S23: LIST, owner-only access](#s23-list-owner-only-access)
      - [S24: GET, owner-only access](#s24-get-owner-only-access)
      - [S25: UPDATE, owner-only mutation](#s25-update-owner-only-mutation)
      - [S26: DELETE, owner-only mutation](#s26-delete-owner-only-mutation)
      - [S27: CREATE, owner-only](#s27-create-owner-only)
  - [TOCTOU Analysis](#toctou-analysis)
    - [When TOCTOU Matters](#when-toctou-matters)
    - [How Each Scenario Handles TOCTOU](#how-each-scenario-handles-toctou)
    - [Key Insight: Prefetch + Constraint for Mutations](#key-insight-prefetch--constraint-for-mutations)
  - [References](#references)

---

## Projection Tables

### What Are Projection Tables?

**Projection tables** are local copies of hierarchical or relational data that enable efficient SQL-level authorization. Instead of calling external services during query execution, PEP uses these pre-synced tables to enforce constraints directly in the database.

**The problem they solve:** When PDP returns constraints like "user can access resources in tenant subtree T1", the PEP needs to translate this into SQL. Without local data, PEP would need to:
1. Call an external service to resolve the tenant hierarchy, or
2. Receive thousands of explicit tenant IDs from PDP (doesn't scale)

Projection tables allow PEP to JOIN against local data, making authorization O(1) regardless of hierarchy size.

**Types of projection tables:**

| Table | Purpose | Enables |
|-------|---------|---------|
| `resource_group_closure` (with `resource_group.group_type='tenant'`) | Tenant hierarchy via resource group closure — ancestor/descendant pairs filtered by group type | `in_tenant_subtree` predicate — efficient subtree queries without recursive CTEs |
| `resource_group_membership` | Resource-to-group associations | `in_group` predicate — filter by group membership |
| `resource_group_closure` (with `resource_group.group_type` for non-tenant groups) | Denormalized group hierarchy | `in_group_subtree` predicate — filter by group subtree |

**Closure tables** specifically solve the hierarchy traversal problem. A closure table contains all ancestor-descendant pairs, allowing subtree queries with a simple `WHERE ancestor_id = X` instead of recursive tree walking.

### Choosing Projection Tables

The choice depends on the application's tenant structure, resource organization, and **endpoint requirements**. Even with a hierarchical tenant model, specific endpoints may operate within a single context tenant (see S12).

### Capabilities and PDP Response

| PEP Capability | Projection Table | Prefetch | PDP Response |
|----------------|------------------|----------|--------------|
| `tenant_hierarchy` | `resource_group_closure` (group_type='tenant') ✅ | **No** | `in_tenant_subtree` predicate |
| (none) | ❌ | **Yes** | `eq`/`in` or decision only |
| `group_hierarchy` | `resource_group_closure` ✅ | **No** | `in_group_subtree` predicate |
| `group_membership` | `resource_group_membership` ✅ | **No** | `in_group` predicate |
| (none for groups) | ❌ | **Yes** | explicit resource IDs |

### When No Projection Tables Are Needed

| Condition | Why Tables Aren't Required |
|-----------|---------------------------|
| Endpoint operates in context tenant only | No subtree traversal → `eq` on `owner_tenant_id` is sufficient (see S12) |
| Few tenants per vendor | PDP can return explicit tenant IDs in `in` predicate |
| Flat tenant structure | No hierarchy → `in_tenant_subtree` not needed |
| No resource groups | `in_group*` predicates not used |
| Low frequency LIST requests | Prefetch overhead is acceptable |

**Important:** The first condition applies regardless of overall tenant model. Even in a hierarchical multi-tenant system, specific endpoints may be designed to work within a single context tenant without subtree access. This is an endpoint-level decision, not a system-wide constraint.

**Example:** Internal enterprise tool with 10 tenants, flat structure. Or: a "My Tasks" endpoint that shows only tasks in user's direct tenant, even though the system supports tenant hierarchy for other operations.

### When to Use Tenant Hierarchy via `resource_group_closure`

Tenant hierarchy queries use `resource_group_closure` joined with `resource_group` where `group_type = 'tenant'`.

| Condition | Why Closure Is Needed |
|-----------|----------------------|
| Tenant hierarchy (parent-child) + many tenants | PDP cannot return all IDs in `in` predicate |
| Frequent LIST requests by subtree | Subtree JOINs more efficient than explicit ID lists |

**Example:** Multi-tenant SaaS with organization hierarchy (org → teams → projects) and thousands of tenants.

### When to Use `resource_group_membership`

| Condition | Why Membership Table Is Needed |
|-----------|-------------------------------|
| Resources belong to groups | Projects, workspaces, folders |
| Frequent group-based filters | "Show all tasks in Project X" |
| Access control via groups | Role assignments at group level |

**Example:** Project management tool where tasks belong to projects.

### When to Use `resource_group_closure`

| Condition | Why Group Closure Is Needed |
|-----------|----------------------------|
| Group hierarchy | Nested folders, sub-projects |
| Subtree queries by groups | "Show all in folder and subfolders" |
| Many groups | PDP cannot expand entire hierarchy to explicit IDs |

**Example:** Document management with nested folders.

### Combinations Summary

| Use Case | resource_group_closure (group_type='tenant') | group_membership | group_closure |
|----------|-----------------------------------------------|------------------|---------------|
| Simple SaaS (flat tenants, no groups) | ❌ | ❌ | ❌ |
| Enterprise SaaS (tenant hierarchy) | ✅ | ❌ | ❌ |
| Project-based SaaS (flat tenants + projects) | ❌ | ✅ | ❌ |
| Complex SaaS (hierarchy + nested projects) | ✅ | ✅ | ✅ |

---

## Scenarios

> **Note:** SQL examples use subqueries for clarity. Production implementations
> may use JOINs or EXISTS for performance optimization.

### With Tenant Hierarchy (resource_group_closure)

PEP has local `resource_group_closure` + `resource_group` tables → can enforce `in_tenant_subtree` predicates by joining on `group_type = 'tenant'`.

---

#### S01: LIST, tenant subtree, PEP has tenant hierarchy

`GET /tasks?tenant_subtree=true`

User requests all tasks visible in their tenant subtree.

**Request:**
```http
GET /tasks?tenant_subtree=true
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "in_tenant_subtree",
            "property": "owner_tenant_id",
            "root_tenant_id": "T1-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id IN (
  SELECT rc.descendant_id FROM resource_group_closure rc
  JOIN resource_group rg ON rg.id = rc.descendant_id
  WHERE rc.ancestor_id = 'T1-uuid'
    AND rg.group_type = 'tenant'
)
```

---

#### S02: GET, tenant subtree, PEP has tenant hierarchy

`GET /tasks/{id}?tenant_subtree=true`

User requests a specific task; PEP enforces tenant subtree access at query level.

**Request:**
```http
GET /tasks/task456-uuid?tenant_subtree=true
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "read" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "in_tenant_subtree",
            "property": "owner_tenant_id",
            "root_tenant_id": "T1-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id IN (
    SELECT rc.descendant_id FROM resource_group_closure rc
    JOIN resource_group rg ON rg.id = rc.descendant_id
    WHERE rc.ancestor_id = 'T1-uuid'
      AND rg.group_type = 'tenant'
  )
```

**Result interpretation:**
- 1 row → return task
- 0 rows → **404 Not Found** (hides resource existence from unauthorized users)

---

#### S03: UPDATE, tenant subtree, PEP has tenant hierarchy

`PUT /tasks/{id}?tenant_subtree=true`

User updates a task; constraint ensures atomic authorization check.

**Request:**
```http
PUT /tasks/task456-uuid?tenant_subtree=true
Authorization: Bearer <token>
Content-Type: application/json

{"status": "completed"}
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "update" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "in_tenant_subtree",
            "property": "owner_tenant_id",
            "root_tenant_id": "T1-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
UPDATE tasks
SET status = 'completed'
WHERE id = 'task456-uuid'
  AND owner_tenant_id IN (
    SELECT rc.descendant_id FROM resource_group_closure rc
    JOIN resource_group rg ON rg.id = rc.descendant_id
    WHERE rc.ancestor_id = 'T1-uuid'
      AND rg.group_type = 'tenant'
  )
```

**Result interpretation:**
- 1 row affected → success
- 0 rows affected → **404 Not Found** (task doesn't exist or no access)

---

#### S04: DELETE, tenant subtree, PEP has tenant hierarchy

`DELETE /tasks/{id}?tenant_subtree=true`

DELETE follows the same pattern as UPDATE (S03). PDP returns `in_tenant_subtree` constraint, PEP applies it in the DELETE's WHERE clause.

**SQL:**
```sql
DELETE FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id IN (
    SELECT rc.descendant_id FROM resource_group_closure rc
    JOIN resource_group rg ON rg.id = rc.descendant_id
    WHERE rc.ancestor_id = 'T1-uuid'
      AND rg.group_type = 'tenant'
  )
```

**Result interpretation:**
- 1 row affected → success
- 0 rows affected → **404 Not Found** (task doesn't exist or no access)

---

#### S05: CREATE, PEP-provided tenant context

`POST /tasks`

User creates a new task. PDP returns constraints for CREATE just like other operations — the PEP will enforce them before the INSERT.

**Request:**
```http
POST /tasks
Authorization: Bearer <token>
Content-Type: application/json

{"title": "New Task", "owner_tenant_id": "T2-uuid"}
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "create" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "properties": {
      "owner_tenant_id": "T2-uuid"
    }
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T2-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T2-uuid"
          }
        ]
      }
    ]
  }
}
```

**PEP compiles constraints**, then enforces them before the INSERT:

**SQL:**
```sql
INSERT INTO tasks (id, owner_tenant_id, title, status)
VALUES ('tasknew-uuid', 'T2-uuid', 'New Task', 'pending')
```

**Note:** PDP returns constraints for CREATE using the same flow as other operations. PEP validates that the INSERT's `owner_tenant_id` (or other resource properties in case of RBAC) matches the constraint. This prevents the caller from creating resources in tenants the PDP didn't authorize.

---

#### S06: CREATE, subject tenant context (no explicit tenant in API)

`POST /tasks`

PEP's API does not include a target tenant in the request body. PEP uses `subject_tenant_id` from `SecurityContext` as the `owner_tenant_id` for the new resource, then sends it to PDP for validation — same flow as S05.

**Request:**
```http
POST /tasks
Authorization: Bearer <token>
Content-Type: application/json

{"title": "New Task"}
```

**PEP resolves tenant from SecurityContext:**

The PEP reads `subject_tenant_id` (T1-uuid) from the `SecurityContext` produced by AuthN Resolver. This is the subject's home tenant — the natural owner for the new resource when no explicit tenant is provided in the API.

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "create" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "properties": {
      "owner_tenant_id": "T1-uuid"
    }
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          }
        ]
      }
    ]
  }
}
```

**PEP compiles constraints**, then enforces them before the INSERT:

**SQL:**
```sql
INSERT INTO tasks (id, owner_tenant_id, title, status)
VALUES ('tasknew-uuid', 'T1-uuid', 'New Task', 'pending')
```

**Difference from S05:** In S05, PEP knows the target tenant from the request body (explicit `owner_tenant_id` field). Here, the API has no tenant field — PEP uses `SecurityContext.subject_tenant_id` instead. Both scenarios follow the same PDP validation flow.

**Design rationale:** Constraints are enforcement predicates (WHERE clauses), not a data source. The PEP should never extract `owner_tenant_id` for INSERT from PDP constraints. Instead, the tenant for a new resource is always determined by the PEP — either from the request body (S05) or from `SecurityContext.subject_tenant_id` (S06) — and then validated by the PDP through the standard constraint flow.

---

### Without Local Closure Table

PEP has no local closure table for tenant hierarchy → PDP returns explicit IDs or PEP prefetches attributes.

---

#### S07: LIST, tenant subtree, PEP without local closure table

`GET /tasks?tenant_subtree=true`

PEP doesn't have a local closure table. PDP resolves the subtree and returns explicit tenant IDs.

**Request:**
```http
GET /tasks?tenant_subtree=true
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

PDP resolves the subtree internally and returns explicit IDs:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "in",
            "property": "owner_tenant_id",
            "values": ["T1-uuid", "T2-uuid", "T3-uuid"]
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id IN ('T1-uuid', 'T2-uuid', 'T3-uuid')
```

**Trade-off:** PDP must know the tenant hierarchy and resolve it. Works well for small tenant counts; may not scale for thousands of tenants.

---

#### S08: GET, tenant subtree, PEP without local closure table

`GET /tasks/{id}?tenant_subtree=true`

PEP doesn't have a local closure table. PEP fetches the resource first (prefetch), then asks PDP for an access decision based on resource attributes with `require_constraints: false`. Since PEP already has the entity, it doesn't need row-level SQL constraints — the PDP decision alone is sufficient.

If the PDP returns `decision: true` **without** constraints, PEP returns the prefetched entity directly (no second query). If the PDP returns constraints despite `require_constraints: false`, PEP compiles them and performs a scoped re-read as a fallback.

**Request:**
```http
GET /tasks/task456-uuid?tenant_subtree=true
Authorization: Bearer <token>
```

**Step 1 — PEP prefetches resource:**
```sql
SELECT * FROM tasks WHERE id = 'task456-uuid'
```
Result: full task record with `owner_tenant_id = 'T2-uuid'`

**Step 2 — PEP → PDP Request (with resource properties, `require_constraints: false`):**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "read" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid",
    "properties": {
      "owner_tenant_id": "T2-uuid"
    }
  },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": false,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

PDP validates that T2 is in T1's subtree. Since `require_constraints: false`, PDP may return a decision-only response (no constraints):

```json
{
  "decision": true,
  "context": {
    "constraints": []
  }
}
```

Alternatively, PDP may still return constraints (e.g., `eq(owner_tenant_id, T2-uuid)`) — the PEP handles both cases.

**Step 3 — Enforce and return result:**

PEP compiles the response into `AccessScope`:
- **No constraints** (`scope.is_unconstrained()`) → return the prefetched entity directly. No second SQL query needed.
- **Constraints returned** → compile to `AccessScope` and perform a scoped re-read (`SELECT ... WHERE id = 'task456-uuid' AND owner_tenant_id = 'T2-uuid'`).
- Resource not found in Step 1 → **404 Not Found**.
- `decision: false` → **404 Not Found** (hides resource existence from unauthorized callers).

**Why no TOCTOU concern:** For GET, the "use" is returning data to the client. Even if `owner_tenant_id` changed between prefetch and response, no security violation occurs — the client either gets data they had access to at query time, or gets 404. For mutations (UPDATE/DELETE), see S09.

---

#### S09: UPDATE, tenant subtree, PEP without local closure table (prefetch)

`PUT /tasks/{id}?tenant_subtree=true`

Unlike S08 (GET), mutations require TOCTOU protection. PEP prefetches `owner_tenant_id`, gets `eq` constraint from PDP, and applies it in UPDATE's WHERE clause. This ensures atomic check-and-modify.

**Request:**
```http
PUT /tasks/task456-uuid?tenant_subtree=true
Authorization: Bearer <token>
Content-Type: application/json

{"status": "completed"}
```

**Step 1 — PEP prefetches:**
```sql
SELECT owner_tenant_id FROM tasks WHERE id = 'task456-uuid'
```
Result: `owner_tenant_id = 'T2-uuid'`

**Step 2 — PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "update" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid",
    "properties": {
      "owner_tenant_id": "T2-uuid"
    }
  },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T2-uuid"
          }
        ]
      }
    ]
  }
}
```

**Step 3 — SQL with constraint:**
```sql
UPDATE tasks
SET status = 'completed'
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T2-uuid'
```

**TOCTOU protection:** If another request changed `owner_tenant_id` between prefetch and UPDATE, the WHERE clause won't match → 0 rows affected → **404**. This prevents unauthorized modification even in a race condition.

---

#### S10: DELETE, tenant subtree, PEP without local closure table (prefetch)

`DELETE /tasks/{id}?tenant_subtree=true`

DELETE follows the same pattern as UPDATE (S09). PEP prefetches `owner_tenant_id`, gets `eq` constraint from PDP, and applies it in the DELETE's WHERE clause.

**SQL:**
```sql
DELETE FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T2-uuid'
```

TOCTOU protection is identical to S09: if `owner_tenant_id` changed between prefetch and DELETE, the WHERE clause won't match → 0 rows → **404**.

---

#### S11: CREATE, PEP without local closure table

CREATE does not query existing rows, so the presence of a local closure table is irrelevant. Both PEP-provided and PDP-resolved tenant patterns work identically regardless of PEP capabilities. See S05 and S06.

**`require_constraints: false` optimization:** When PEP sends resource properties (e.g., `owner_tenant_id` of the entity being created) to the PDP, it can set `require_constraints: false`. If the PDP returns `decision: true` without constraints, the resulting `AccessScope` is `allow_all()`, and `validate_insert_scope` skips validation (its `is_unconstrained()` fast path). If the PDP returns constraints, they are compiled and validated against the insert as usual. This avoids unnecessary constraint compilation when the PDP decision alone is sufficient.

---

#### S12: GET, context tenant only (no subtree)

`GET /tasks/{id}`

Simplest case — access limited to context tenant only, no subtree traversal. User can only access resources directly owned by their tenant.

**Request:**
```http
GET /tasks/task456-uuid
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "read" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
```

**Note:** No prefetch needed, no closure table required. PDP returns direct `eq` constraint based on context tenant. This pattern applies when the endpoint operates within a single-tenant context, regardless of whether the overall tenant model is hierarchical.

---

### Resource Groups

> **Note:** Resource groups are tenant-scoped. **PDP guarantees** that any `group_ids` or `root_group_id` returned in constraints belong to the request context tenant. PEP trusts this guarantee — it has no group metadata to validate against (only `resource_group_membership` table).
>
> All group-based constraints also include a tenant predicate on the resource (typically `eq` on `owner_tenant_id`) as defense in depth, ensuring tenant isolation at the resource level.

---

#### S13: LIST, group membership, PEP has resource_group_membership

`GET /tasks`

User has access to specific projects (flat group membership, no hierarchy).

**Request:**
```http
GET /tasks
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["group_membership"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Tenant constraint is always included — groups don't bypass tenant isolation:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in_group",
            "property": "id",
            "group_ids": ["ProjectA-uuid", "ProjectB-uuid"]
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id = 'T1-uuid'
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id IN ('ProjectA-uuid', 'ProjectB-uuid')
  )
```

---

#### S14: LIST, group subtree, PEP has resource_group_closure

`GET /tasks`

User has access to a project folder and all its subfolders.

**Request:**
```http
GET /tasks
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["group_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Tenant constraint is always included:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in_group_subtree",
            "property": "id",
            "root_group_id": "FolderA-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id = 'T1-uuid'
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id IN (
      SELECT descendant_id FROM resource_group_closure
      WHERE ancestor_id = 'FolderA-uuid'
  )
)
```

---

#### S15: UPDATE, group membership, PEP has resource_group_membership

`PUT /tasks/{id}`

User updates a task; PEP has resource_group_membership table. Similar to tenant-based UPDATE scenarios, but filtering by group membership.

**Request:**
```http
PUT /tasks/task456-uuid
Authorization: Bearer <token>
Content-Type: application/json

{"status": "completed"}
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "update" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["group_membership"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Tenant constraint is always included:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in_group",
            "property": "id",
            "group_ids": ["ProjectA-uuid", "ProjectB-uuid"]
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
UPDATE tasks
SET status = 'completed'
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id IN ('ProjectA-uuid', 'ProjectB-uuid')
  )
```

**Result interpretation:**
- 1 row affected → success
- 0 rows affected → task doesn't exist or not in user's accessible groups → **404**

---

#### S16: UPDATE, group subtree, PEP has resource_group_closure

`PUT /tasks/{id}`

User updates a task; PEP has both resource_group_membership and resource_group_closure tables.

**Request:**
```http
PUT /tasks/task456-uuid
Authorization: Bearer <token>
Content-Type: application/json

{"status": "completed"}
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "update" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["group_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Tenant constraint is always included:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in_group_subtree",
            "property": "id",
            "root_group_id": "FolderA-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
UPDATE tasks
SET status = 'completed'
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id IN (
      SELECT descendant_id FROM resource_group_closure
      WHERE ancestor_id = 'FolderA-uuid'
    )
  )
```

---

#### S17: GET, group membership, PEP without resource_group_membership

`GET /tasks/{id}`

PEP doesn't have resource_group_membership table. PDP resolves group membership internally and returns a tenant constraint for defense in depth.

**Request:**
```http
GET /tasks/task456-uuid
Authorization: Bearer <token>
```

**Step 1 — PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "read" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP internally:**
1. Resolves resource's group membership (via PIP or own storage)
2. Checks if subject has access to any of those groups
3. Validates tenant access

**PDP → PEP Response:**

PDP returns tenant constraint as defense in depth (group check already done):

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          }
        ]
      }
    ]
  }
}
```

**Step 2 — SQL with constraint:**
```sql
SELECT * FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
```

**Result interpretation:**
- 1 row → return task
- 0 rows → **404 Not Found**

**Note:** This pattern requires PDP to have access to group membership data. For LIST operations without resource_group_membership on PEP side, PDP would need to return explicit resource IDs (impractical for large datasets). This scenario works best for point operations (GET, UPDATE, DELETE by ID).

---

#### S18: LIST, group subtree, PEP has membership but no closure

`GET /tasks`

PEP has resource_group_membership but not resource_group_closure. PDP expands group hierarchy to explicit group IDs.

**Request:**
```http
GET /tasks
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["group_membership"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**Note:** PEP declares `group_membership` capability (has the membership table) but NOT `group_hierarchy` (no closure table).

**PDP → PEP Response:**

PDP knows user has access to FolderA and its subfolders. Since PEP can't handle `in_group_subtree`, PDP expands the hierarchy to explicit group IDs. Tenant constraint is always included:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in_group",
            "property": "id",
            "group_ids": ["FolderA-uuid", "FolderASub1-uuid", "FolderASub2-uuid", "FolderASub1Deep-uuid"]
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id = 'T1-uuid'
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id IN ('FolderA-uuid', 'FolderASub1-uuid', 'FolderASub2-uuid', 'FolderASub1Deep-uuid')
  )
```

**Trade-off:** PDP must know the group hierarchy and expand it. Works well for shallow hierarchies or small group counts; may not scale for deep/wide hierarchies with thousands of groups.

---

### Advanced Patterns

---

#### S19: LIST, tenant subtree and group membership (AND)

`GET /tasks?tenant_subtree=true`

User has access to tasks in their tenant subtree AND in specific projects. Both conditions must be satisfied.

**Request:**
```http
GET /tasks?tenant_subtree=true
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy", "group_membership"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Single constraint with multiple predicates (AND semantics):

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "in_tenant_subtree",
            "property": "owner_tenant_id",
            "root_tenant_id": "T1-uuid"
          },
          {
            "type": "in_group",
            "property": "id",
            "group_ids": ["ProjectA-uuid"]
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id IN (
    SELECT rc.descendant_id FROM resource_group_closure rc
    JOIN resource_group rg ON rg.id = rc.descendant_id
    WHERE rc.ancestor_id = 'T1-uuid'
      AND rg.group_type = 'tenant'
  )
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id = 'ProjectA-uuid'
  )
```

---

#### S20: LIST, tenant subtree and group subtree

`GET /tasks?tenant_subtree=true`

User has access to tasks that are owned by tenants in their subtree AND belong to a folder or any of its subfolders. This scenario demonstrates the most complex constraint combination using all three projection tables.

**Use case:** Manager can see tasks from their department (tenant subtree) that are in the "Q1 Projects" folder or any nested subfolder.

**Request:**
```http
GET /tasks?tenant_subtree=true
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "subtree",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy", "group_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Single constraint with two predicates (AND semantics):

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "in_tenant_subtree",
            "property": "owner_tenant_id",
            "root_tenant_id": "T1-uuid"
          },
          {
            "type": "in_group_subtree",
            "property": "id",
            "root_group_id": "FolderA-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id IN (
    SELECT rc.descendant_id FROM resource_group_closure rc
    JOIN resource_group rg ON rg.id = rc.descendant_id
    WHERE rc.ancestor_id = 'T1-uuid'
      AND rg.group_type = 'tenant'
  )
  AND id IN (
    SELECT resource_id FROM resource_group_membership
    WHERE group_id IN (
      SELECT descendant_id FROM resource_group_closure
      WHERE ancestor_id = 'FolderA-uuid'
    )
  )
```

**Projection tables used:**
- `resource_group_closure` (with `group_type='tenant'`) — resolves tenant subtree (T1 and all descendants)
- `resource_group_closure` — resolves folder hierarchy (FolderA and all subfolders)
- `resource_group_membership` — maps resources to groups

**Note:** This is the most demanding query pattern. For large datasets, ensure proper indexing on all three projection tables and consider the scalability considerations in [DESIGN.md Open Questions](./DESIGN.md#open-questions).

---

#### S21: LIST, multiple access paths (OR)

`GET /tasks`

User has multiple ways to access tasks: (1) via project membership, (2) via explicitly shared tasks.

**Request:**
```http
GET /tasks
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["group_membership"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**

Multiple constraints (OR semantics). Tenant constraint is included in each path:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in_group",
            "property": "id",
            "group_ids": ["ProjectA-uuid"]
          }
        ]
      },
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "in",
            "property": "id",
            "values": ["taskshared1-uuid", "taskshared2-uuid"]
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE (
    owner_tenant_id = 'T1-uuid'
    AND id IN (
      SELECT resource_id FROM resource_group_membership
      WHERE group_id = 'ProjectA-uuid'
    )
  )
  OR (
    owner_tenant_id = 'T1-uuid'
    AND id IN ('taskshared1-uuid', 'taskshared2-uuid')
  )
```

---

#### S22: Access denied

`GET /tasks`

User doesn't have permission to access the requested resources.

**Request:**
```http
GET /tasks
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "tenant_context": {
      "mode": "root_only",
      "root_id": "T1-uuid"
    },
    "require_constraints": true,
    "capabilities": ["tenant_hierarchy"],
    "supported_properties": ["owner_tenant_id", "id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": false,
  "context": {
    "deny_reason": {
      "error_code": "gts.x.core.errors.err.v1~x.authz.errors.insufficient_permissions.v1",
      "details": "Subject 'user123-uuid' lacks 'list' permission on 'gts.x.core.tasks.task.v1~' in tenant 'T1-uuid'"
    }
  }
}
```

**PEP Action:**
- No SQL query is executed
- Use `error_code` for programmatic handling (e.g., metrics, error categorization)
- Log `deny_reason` for audit/debugging (includes `error_code` and `details`)
- Return **403 Forbidden** to client without exposing `details`

**Fail-closed principle:** The PEP never executes a database query when `decision: false`. This prevents any data leakage and ensures authorization is enforced before resource access.

**Note on deny_reason:** The `deny_reason` is required when `decision: false`. PEP uses `error_code` for programmatic handling and logs `details` for troubleshooting, but returns a generic 403 response to prevent leaking authorization policy details to clients.

---

### Subject Owner-Based Access

PEP supports `owner_id` as a standard resource property for per-subject ownership filtering. These scenarios demonstrate how `owner_id` constraints restrict access to resources owned by a specific user.

**No projection tables** are needed — `owner_id` uses simple `eq` predicates compiled directly to SQL.

**No prefetch** is needed — PDP always knows the subject's identity from `subject.id` in the evaluation request, so it can return `eq(owner_id, subject_id)` without PEP prefetching resource attributes. This is fundamentally different from "without local closure table" scenarios (S08-S10), where PEP must prefetch `owner_tenant_id` to tell PDP which specific tenant to validate.

**`tenant_context` is omitted** from these requests. PDP infers the tenant context from `subject.properties.tenant_id` (see [DESIGN.md — tenant_context note](./DESIGN.md#request--response-example)). This is only safe when the subject's home tenant is the intended context; for cross-tenant access or service-to-service flows, supply `tenant_context` explicitly. PDP still returns `eq(owner_tenant_id, ...)` as defense-in-depth to ensure tenant isolation at the SQL level.

---

#### S23: LIST, owner-only access

`GET /tasks`

User requests only their own tasks. PDP restricts access to resources where `owner_id` matches the subject.

**Use case:** Personal task list — "show only my tasks."

**Request:**
```http
GET /tasks
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "list" },
  "resource": { "type": "gts.x.core.tasks.task.v1~" },
  "context": {
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id", "owner_id"]
  }
}
```

**PDP → PEP Response:**

Single constraint with two predicates (AND semantics) — tenant isolation (defense-in-depth) plus owner restriction:

```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "eq",
            "property": "owner_id",
            "value": "user123-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE owner_tenant_id = 'T1-uuid'
  AND owner_id = 'user123-uuid'
```

---

#### S24: GET, owner-only access

`GET /tasks/{id}`

User requests a specific task; PDP constrains access to resources owned by the subject.

**Use case:** View task details — accessible only if the user owns it.

**Request:**
```http
GET /tasks/task456-uuid
Authorization: Bearer <token>
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "read" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id", "owner_id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "eq",
            "property": "owner_id",
            "value": "user123-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
SELECT * FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
  AND owner_id = 'user123-uuid'
```

**Result interpretation:**
- 1 row → return task
- 0 rows → **404 Not Found** (task doesn't exist, wrong tenant, or user doesn't own it)

---

#### S25: UPDATE, owner-only mutation

`PUT /tasks/{id}`

User updates a task; PDP constrains the mutation to resources owned by the subject. The `owner_id` constraint in the WHERE clause provides TOCTOU protection — if ownership changed between check and execution, the update atomically fails.

**Use case:** User can only edit their own tasks.

**Request:**
```http
PUT /tasks/task456-uuid
Authorization: Bearer <token>
Content-Type: application/json

{"status": "done"}
```

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "update" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "id": "task456-uuid"
  },
  "context": {
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id", "owner_id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "eq",
            "property": "owner_id",
            "value": "user123-uuid"
          }
        ]
      }
    ]
  }
}
```

**SQL:**
```sql
UPDATE tasks
SET status = 'done'
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
  AND owner_id = 'user123-uuid'
```

**Result interpretation:**
- 1 row affected → success
- 0 rows affected → **404 Not Found** (task doesn't exist, wrong tenant, or user doesn't own it)

---

#### S26: DELETE, owner-only mutation

`DELETE /tasks/{id}`

DELETE follows the same pattern as UPDATE (S25). PDP returns `eq(owner_id)` + `eq(owner_tenant_id)` constraints, PEP applies them in the DELETE's WHERE clause.

**SQL:**
```sql
DELETE FROM tasks
WHERE id = 'task456-uuid'
  AND owner_tenant_id = 'T1-uuid'
  AND owner_id = 'user123-uuid'
```

**Result interpretation:**
- 1 row affected → success
- 0 rows affected → **404 Not Found** (task doesn't exist, wrong tenant, or user doesn't own it)

TOCTOU protection is identical to S25: if `owner_id` or `owner_tenant_id` changed between check and DELETE, the WHERE clause won't match → 0 rows → **404**.

---

#### S27: CREATE, owner-only

`POST /tasks`

User creates a new task. PEP sets `owner_id` from `SecurityContext.subject_id` — the subject owns the resource they create. PDP validates both `owner_tenant_id` and `owner_id` via constraints, preventing the caller from creating resources assigned to a different user.

**Use case:** User creates a task assigned to themselves.

**Request:**
```http
POST /tasks
Authorization: Bearer <token>
Content-Type: application/json

{"title": "New Task"}
```

**PEP resolves owner from SecurityContext:**

PEP reads `subject_id` (user123-uuid) and `subject_tenant_id` (T1-uuid) from `SecurityContext`. These become `owner_id` and `owner_tenant_id` for the new resource — same pattern as S06 for tenant context.

**PEP → PDP Request:**
```json
{
  "subject": {
    "type": "gts.x.core.security.subject_user.v1~",
    "id": "user123-uuid",
    "properties": { "tenant_id": "T1-uuid" }
  },
  "action": { "name": "create" },
  "resource": {
    "type": "gts.x.core.tasks.task.v1~",
    "properties": {
      "owner_tenant_id": "T1-uuid",
      "owner_id": "user123-uuid"
    }
  },
  "context": {
    "require_constraints": true,
    "capabilities": [],
    "supported_properties": ["owner_tenant_id", "id", "owner_id"]
  }
}
```

**PDP → PEP Response:**
```json
{
  "decision": true,
  "context": {
    "constraints": [
      {
        "predicates": [
          {
            "type": "eq",
            "property": "owner_tenant_id",
            "value": "T1-uuid"
          },
          {
            "type": "eq",
            "property": "owner_id",
            "value": "user123-uuid"
          }
        ]
      }
    ]
  }
}
```

**PEP compiles constraints**, then validates the INSERT against them:

**SQL:**
```sql
INSERT INTO tasks (id, owner_tenant_id, owner_id, title, status)
VALUES ('tasknew-uuid', 'T1-uuid', 'user123-uuid', 'New Task', 'pending')
```

**Note:** PDP returns constraints for CREATE using the same flow as other operations. PEP validates that the INSERT's `owner_tenant_id` and `owner_id` match the constraints. This prevents the caller from creating resources in unauthorized tenants or assigned to other users.

---

## TOCTOU Analysis

[Time-of-check to time-of-use (TOCTOU)](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use) is a class of race condition where a security check is performed at one point, but the protected action occurs later when conditions may have changed.

### When TOCTOU Matters

TOCTOU is a security concern only for **mutations** (UPDATE, DELETE). For **reads** (GET, LIST), there's no security violation if the resource changes between check and response — the client receives data they had access to at query time.

| Operation | TOCTOU Concern | Why |
|-----------|----------------|-----|
| GET | ❌ No | Read returns point-in-time snapshot; no state change |
| LIST | ❌ No | Same as GET — read-only |
| UPDATE | ✅ Yes | Must ensure authorization at mutation time |
| DELETE | ✅ Yes | Must ensure authorization at mutation time |
| CREATE | ❌ No | No existing resource to race against |

### How Each Scenario Handles TOCTOU

**Tenant-based scenarios:**

| Scenario | Operation | Closure | Constraint | TOCTOU Protection |
|----------|-----------|---------|------------|-------------------|
| S01-S04 | LIST/GET/UPDATE/DELETE | ✅ | `in_tenant_subtree` | ✅ Atomic SQL check |
| S08 | GET | ❌ | `eq` (prefetched) | N/A (read-only) |
| S09, S10 | UPDATE/DELETE | ❌ | `eq` (prefetched) | ✅ Atomic SQL check |
| S05, S06, S11 | CREATE | N/A | `eq` (from PDP) | N/A (no existing resource) |

**Resource group scenarios:**

| Scenario | Operation | Projection Tables | Constraint | TOCTOU Protection |
|----------|-----------|-------------------|------------|-------------------|
| S13, S14 | LIST | ✅ | `in_group` / `in_group_subtree` | ✅ Atomic SQL check |
| S15, S16 | UPDATE | ✅ | `in_group` / `in_group_subtree` | ✅ Atomic SQL check |
| S17 | GET | ❌ | `eq` (tenant) | N/A (read-only) |
| S18 | LIST | membership only | `in_group` (expanded) | ✅ Atomic SQL check |

**Subject owner-based scenarios:**

| Scenario | Operation | Constraint | TOCTOU Protection |
|----------|-----------|------------|-------------------|
| S23 | LIST | `eq` (owner) | N/A (read-only) |
| S24 | GET | `eq` (owner) | N/A (read-only) |
| S25 | UPDATE | `eq` (owner) | ✅ Atomic SQL check |
| S26 | DELETE | `eq` (owner) | ✅ Atomic SQL check |
| S27 | CREATE | `eq` (owner) | N/A (no existing resource) |

### Key Insight: Prefetch + Constraint for Mutations

Without closure tables, mutations (UPDATE/DELETE) use a two-step pattern:

1. **Prefetch:** PEP reads `owner_tenant_id = 'T2-uuid'` from database
2. **PDP check:** PDP validates T2 is accessible, returns `eq: owner_tenant_id = 'T2-uuid'`
3. **SQL execution:** `UPDATE tasks SET ... WHERE id = 'X' AND owner_tenant_id = 'T2-uuid'`
4. **If tenant changed:** WHERE clause won't match → 0 rows affected → 404

The constraint acts as a [compare-and-swap](https://en.wikipedia.org/wiki/Compare-and-swap) mechanism — if the value changed between check and use, the operation atomically fails.

**For reads (S08):** PEP prefetches the resource, asks PDP with `require_constraints: false`, and returns the prefetched data if `decision: true` with no constraints. If constraints are returned, PEP falls back to a scoped re-read.

---

## References

- [DESIGN.md](./DESIGN.md) — Core authorization design
- [TENANT_MODEL.md](./TENANT_MODEL.md) — Tenant topology and closure tables
- [RESOURCE_GROUP_MODEL.md](./RESOURCE_GROUP_MODEL.md) — Resource group topology, membership, hierarchy
- [TOCTOU - Wikipedia](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
- [Race Conditions - PortSwigger](https://portswigger.net/web-security/race-conditions)
- [AWS Multi-tenant Authorization](https://docs.aws.amazon.com/prescriptive-guidance/latest/saas-multitenant-api-access-authorization/introduction.html)
