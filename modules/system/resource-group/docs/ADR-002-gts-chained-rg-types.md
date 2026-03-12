Created:  2026-03-12 by Constructor Tech

# ADR-002: Chained GTS Identifiers for RG Type Definitions

## Status

**Proposed** — under review.

## Context

ADR-001 established that `resource_group_type.code` and `resource_group_membership.resource_type` will use GTS type paths. This ADR defines **how RG type definitions are modeled** in GTS.

GTS supports chained identifiers where left = base type, right = derived type:

```
gts.A~B~  =  "B is-a A"  =  "B inherits from A"
```

Examples from GTS spec:

```
gts.x.core.events.type.v1~x.commerce.orders.order_placed.v1.0~
    ─── base (event) ─────  ─── derived (order_placed) ──────

gts.x.infra.compute.vm.v1~nutanix.ahv._.vm.v1~
    ─── base (vm) ─────────  ─ derived (nutanix vm) ──
```

---

## Decision

A base GTS type `gts.x.system.rg.type.v1~` defines the RG type contract (schema with `can_be_root` and `allowed_parents`). Each concrete RG type inherits from it via chaining:

```
gts.x.system.rg.type.v1~                              ← base: defines {can_be_root, allowed_parents}
gts.x.system.rg.type.v1~x.system.rg.tenant.v1~       ← derived: tenant
gts.x.system.rg.type.v1~x.system.rg.department.v1~   ← derived: department
gts.x.system.rg.type.v1~x.system.rg.branch.v1~       ← derived: branch
```

### Base Type Schema

```json
{
  "$id": "gts://gts.x.system.rg.type.v1~",
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Resource Group Type Definition",
  "description": "Base schema for all RG type definitions. Concrete types inherit via chaining.",
  "type": "object",
  "required": ["can_be_root", "allowed_parents"],
  "properties": {
    "can_be_root": {
      "type": "boolean",
      "description": "Whether this type permits root placement (no parent_id)."
    },
    "allowed_parents": {
      "type": "array",
      "items": {
        "type": "string",
        "x-gts-ref": "gts.x.system.rg.type.v1~"
      },
      "description": "GTS type paths of allowed parent types."
    }
  },
  "additionalProperties": false,
  "x-gts-constraints": {
    "placement-invariant": "can_be_root OR len(allowed_parents) >= 1"
  }
}
```

### Derived Type Schema (example: tenant)

```json
{
  "$id": "gts://gts.x.system.rg.type.v1~x.system.rg.tenant.v1~",
  "$schema": "http://json-schema.org/draft-07/schema#",
  "allOf": [
    { "$ref": "gts://gts.x.system.rg.type.v1~" },
    {
      "type": "object",
      "properties": {
        "can_be_root": { "const": true },
        "allowed_parents": {
          "const": ["gts.x.system.rg.type.v1~x.system.rg.tenant.v1~"]
        }
      }
    }
  ]
}
```

---

## Benefits

**1. Self-describing identifiers**

Anyone seeing `gts.x.system.rg.type.v1~x.system.rg.department.v1~` immediately knows:
- This is an RG type definition (base = `rg.type.v1~`)
- The specific type is `department`
- It conforms to the `rg.type.v1~` schema (has `can_be_root` and `allowed_parents`)

**2. Precise `x-gts-ref` in other schemas**

Other modules can formally reference "any RG type":

```json
{
  "group_type": {
    "type": "string",
    "x-gts-ref": "gts.x.system.rg.type.v1~"
  }
}
```

GTS validation ensures the value starts with the base type prefix.

**3. Trivial "list all RG types" query**

```sql
WHERE code LIKE 'gts.x.system.rg.type.v1~%'
```

**4. Type Registry structural validation**

When Type Registry arrives, it can validate that every type with base `rg.type.v1~` actually provides `can_be_root` and `allowed_parents` in the correct format.

**5. Base type versioning**

If the RG type contract evolves (e.g., a third field is added):

```
gts.x.system.rg.type.v2~x.system.rg.tenant.v1~    ← new contract version
gts.x.system.rg.type.v1~x.system.rg.tenant.v1~    ← old contract version
```

The base version explicitly declares which contract the type follows.

**6. `allowed_parents` values are self-validating**

Each entry in `allowed_parents` is a chained GTS path starting with `rg.type.v1~`. `x-gts-ref` validates that these references point to actual RG types, not arbitrary strings.

**7. Consistent with GTS patterns**

Events, VMs, modules — all use chaining for "X is-a Y" relationships. RG types follow the same convention.

---

## Impact on ADR-001

All ADR-001 decisions remain unchanged. Only example values are affected:

| Field | ADR-001 | ADR-002 |
|---|---|---|
| `resource_group_type.code` | `gts.x.system.rg.tenant.v1~` | `gts.x.system.rg.type.v1~x.system.rg.tenant.v1~` |
| `resource_group_type.code` | `gts.x.system.rg.department.v1~` | `gts.x.system.rg.type.v1~x.system.rg.department.v1~` |
| `resource_group_type.code` | `gts.x.system.rg.branch.v1~` | `gts.x.system.rg.type.v1~x.system.rg.branch.v1~` |
| `allowed_parents` | `[gts.x.system.rg.tenant.v1~]` | `[gts.x.system.rg.type.v1~x.system.rg.tenant.v1~]` |
| `resource_group.group_type` | `gts.x.system.rg.department.v1~` | `gts.x.system.rg.type.v1~x.system.rg.department.v1~` |
| `membership.resource_type` | `gts.x.idp.users.user.v1~` | `gts.x.idp.users.user.v1~` **(unchanged)** |

Note: `resource_type` in memberships is **not affected** — those are external domain types (User, Course), not RG type definitions.

### URL Examples

```
GET  /api/resource-group/v1/types/gts.x.system.rg.type.v1~x.system.rg.department.v1~
POST /api/resource-group/v1/groups
     { "group_type": "gts.x.system.rg.type.v1~x.system.rg.department.v1~", ... }
GET  /api/resource-group/v1/groups?$filter=group_type eq 'gts.x.system.rg.type.v1~x.system.rg.department.v1~'
```

### Seed Data

```sql
INSERT INTO resource_group_type (code, can_be_root, allowed_parents) VALUES
  ('gts.x.system.rg.type.v1~x.system.rg.tenant.v1~', true,
   '{"gts.x.system.rg.type.v1~x.system.rg.tenant.v1~"}'),
  ('gts.x.system.rg.type.v1~x.system.rg.department.v1~', false,
   '{"gts.x.system.rg.type.v1~x.system.rg.tenant.v1~"}'),
  ('gts.x.system.rg.type.v1~x.system.rg.branch.v1~', false,
   '{"gts.x.system.rg.type.v1~x.system.rg.department.v1~"}');
```

### DB Schema

No changes — the `gts_type_path` DOMAIN from ADR-001 already supports chained identifiers.
