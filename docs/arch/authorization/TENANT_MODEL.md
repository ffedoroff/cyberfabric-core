# Tenant Model

This document describes Cyber Fabric's multi-tenancy model, tenant topology, and isolation mechanisms.

## Table of Contents

- [Tenant Model](#tenant-model)
  - [Table of Contents](#table-of-contents)
  - [Overview](#overview)
  - [Tenant Topology: Forest](#tenant-topology-forest)
  - [Tenant Properties](#tenant-properties)
  - [Hierarchical Access](#hierarchical-access)
  - [Context Tenant vs Subject Tenant](#context-tenant-vs-subject-tenant)
  - [Tenant Subtree Queries](#tenant-subtree-queries)
  - [Closure Table (via Resource Group)](#closure-table-via-resource-group)
  - [References](#references)

---

## Overview

Cyber Fabric uses a **hierarchical multi-tenancy** model where tenants form a forest (multiple independent trees). Each tenant can have child tenants, creating organizational structures like:

```
Vendor
├── Organization A
│   ├── Team A1
│   └── Team A2
└── Organization B
    ├── Team B1
    └── Team B2
```

Key principles:
- **Isolation by default** — tenants cannot access each other's data
- **Hierarchical access** — parent tenants can access child tenant data throughout the entire subtree
- **No barriers** — all tenants in a subtree are accessible to their ancestor tenants

---

## Tenant Topology: Forest

The tenant structure is a **forest** — a collection of independent trees with no single global root.

```
       [T1]              [T5]           ← Root tenants (no parent)
      /    \               |
   [T2]    [T3]          [T6]
     |
   [T4]
```

**Properties:**
- Each tree has exactly one root tenant (`parent_id = NULL`)
- A tenant belongs to exactly one tree
- Trees are completely isolated from each other
- Depth is unlimited (but deep hierarchies may impact performance)

**Why forest, not single tree?**
- Supports multiple independent vendors/organizations
- No artificial "super-root" that would complicate access control
- Each tree can have different policies and configurations
- Enables datacenter migration — vendor can gradually move tenant trees between regions/datacenters without cross-tree dependencies

---

## Tenant Properties

| Property | Type | Description |
|----------|------|-------------|
| `id` | UUID | Unique tenant identifier |
| `parent_id` | UUID? | Parent tenant (NULL for root tenants) |
| `status` | enum | `active` |

Tenants are modeled as resource groups with `group_type = 'tenant'`. See [RESOURCE_GROUP_MODEL.md](./RESOURCE_GROUP_MODEL.md).

**Status semantics:**
- `active` — normal operation

There is no soft delete. Tenants are either present (`active`) or hard-deleted. Suspension semantics, if needed, are handled at the tenant-resolver level, not in the closure/hierarchy model.

---

## Hierarchical Access

All tenants in a subtree are accessible to their ancestor tenants. Parent tenants always have full visibility into their entire subtree.

**Example:**

```
T1 (parent)
├── T2
│   └── T3
└── T4
```

**Access from T1's perspective:**
- Can access T1's own resources
- Can access T2's resources (child)
- Can access T3's resources (grandchild, via T2)
- Can access T4's resources (child)

**Access from T2's perspective:**
- Can access T2's own resources
- Can access T3's resources (child)
- Cannot access T1's or T4's resources (T2 is not their ancestor)

Fine-grained access control within the subtree is handled by authorization policies at the module/endpoint level, not by the tenant hierarchy model itself.

---

## Context Tenant vs Subject Tenant

Two different tenant concepts appear in authorization:

| Concept | Description | Example |
|---------|-------------|---------|
| **Subject Tenant** | Tenant the user belongs to (from token/identity) | User's "home" organization |
| **Context Tenant** | Tenant scope for the current operation | May differ for cross-tenant operations |

**Typical case:** Subject tenant = Context tenant (user operates in their own tenant)

**Cross-tenant case:** Admin from parent tenant T1 operates in child tenant T2's context:
- Subject tenant: T1 (where admin belongs)
- Context tenant: T2 (where operation is scoped)

**In authorization requests:**
```jsonc
{
  "subject": {
    "properties": { "tenant_id": "T1" }  // Subject tenant
  },
  "context": {
    "tenant_context": {
      "mode": "root_only",  // Single tenant T2
      "root_id": "T2"
    }
    // OR for subtree:
    // "tenant_context": {
    //   "mode": "subtree",   // T2 + descendants
    //   "root_id": "T2"
    // }
  }
}
```

---

## Tenant Subtree Queries

Many operations need to query "all resources in tenant T and its children". This is a **subtree query**.

**Options for subtree queries:**

| Approach | Pros | Cons |
|----------|------|------|
| Recursive CTE | No extra tables | Slow for deep hierarchies, not portable |
| Explicit ID list from PDP | Simple SQL | Doesn't scale (thousands of IDs) |
| Closure table | O(1) JOIN, scales well | Requires sync, storage overhead |

Cyber Fabric recommends **closure tables** for production deployments with hierarchical tenants.

**Tenant scope parameters (in `context.tenant_context`):**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `mode` | `"subtree"` | `"root_only"` (single tenant) or `"subtree"` (tenant + descendants) |
| `root_id` | — | Root tenant. Optional — PDP can determine from `token_scopes` or `subject.properties.tenant_id` |

---

## Closure Table (via Resource Group)

Tenants are resource groups with `group_type = 'tenant'`, and the hierarchy is stored in the `resource_group_closure` table. Tenant subtree queries use a JOIN to filter by group type.

See [RESOURCE_GROUP_MODEL.md](./RESOURCE_GROUP_MODEL.md) for the full resource group closure table design.

**`resource_group_closure` schema (relevant columns):**

| Column | Type | Description |
|--------|------|-------------|
| `ancestor_id` | UUID | Ancestor resource group |
| `descendant_id` | UUID | Descendant resource group |
| `depth` | INT | Distance between ancestor and descendant (0 for self-references) |

**`resource_group` schema (relevant columns):**

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Resource group identifier |
| `group_type` | VARCHAR | Type discriminator (`'tenant'`, etc.) |

**Example data for the hierarchy:**

```
T1
├── T2
│   └── T3
└── T4
```

`resource_group` rows (tenant entries only):

| id | group_type |
|----|------------|
| T1 | tenant |
| T2 | tenant |
| T3 | tenant |
| T4 | tenant |

`resource_group_closure` rows (tenant entries only):

| ancestor_id | descendant_id | depth |
|-------------|---------------|-------|
| T1 | T1 | 0 |
| T1 | T2 | 1 |
| T1 | T3 | 2 |
| T1 | T4 | 1 |
| T2 | T2 | 0 |
| T2 | T3 | 1 |
| T3 | T3 | 0 |
| T4 | T4 | 0 |

**Key observations:**
- Every tenant has a self-referencing row (`depth = 0`)
- All ancestor-descendant pairs are present, enabling O(1) subtree lookups via JOIN
- Parent tenants have full visibility into their subtree

**Query: "All tenants in T1's subtree"**

```sql
SELECT c.descendant_id
FROM resource_group_closure c
JOIN resource_group rg ON rg.id = c.descendant_id
WHERE c.ancestor_id = 'T1'
  AND rg.group_type = 'tenant'
```

Result: T1, T2, T3, T4

**Query: "All tenants in T2's subtree"**

```sql
SELECT c.descendant_id
FROM resource_group_closure c
JOIN resource_group rg ON rg.id = c.descendant_id
WHERE c.ancestor_id = 'T2'
  AND rg.group_type = 'tenant'
```

Result: T2, T3

**Query: "Direct children of T1"**

```sql
SELECT c.descendant_id
FROM resource_group_closure c
JOIN resource_group rg ON rg.id = c.descendant_id
WHERE c.ancestor_id = 'T1'
  AND c.depth = 1
  AND rg.group_type = 'tenant'
```

Result: T2, T4

**Synchronization:** How the closure table is synchronized with tenant lifecycle events, consistency guarantees, and conflict resolution are out of scope for this document. See Tenant Resolver design documentation (TBD).

---

## References

- [DESIGN.md](./DESIGN.md) — Core authorization design
- [RESOURCE_GROUP_MODEL.md](./RESOURCE_GROUP_MODEL.md) — Resource group topology, membership, hierarchy
- [AUTHZ_USAGE_SCENARIOS.md](./AUTHZ_USAGE_SCENARIOS.md) — Authorization scenarios with tenant examples
