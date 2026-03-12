Created:  2026-03-11 by Constructor Tech

# ADR-001: GTS Type Paths for Resource Group Type Codes and Resource Types

## Status

**Proposed** — under review.

## Context

Resource Group (RG) module has two string fields that classify entities by type:

| Field | Table | Current definition | Example values |
|---|---|---|---|
| `resource_group_type.code` | `resource_group_type` | `TEXT PRIMARY KEY CHECK (code = LOWER(code))` | `tenant`, `department`, `branch` |
| `resource_group_membership.resource_type` | `resource_group_membership` | `TEXT NOT NULL` | `User`, `Course`, `Document` |

Additionally, these derived fields reference `resource_group_type.code`:

| Field | Table | Mechanism |
|---|---|---|
| `resource_group.group_type` | `resource_group` | FK → `resource_group_type.code` + `CHECK (group_type = LOWER(group_type))` |
| `resource_group_type.allowed_parents` | `resource_group_type` | `TEXT[]` — array of type codes (validated at application level) |

Currently all values are **free-form strings** with no structural governance. The platform team has decided to adopt GTS (Global Type System) as the universal type identification standard. This ADR evaluates how to integrate GTS type paths into the RG module.

### GTS Type Path Format

GTS spec: https://github.com/GlobalTypeSystem/gts-spec

A **type identifier** (schema) follows the canonical format:

```
gts.<vendor>.<package>.<namespace>.<type>.v<MAJOR>[.<MINOR>]~
```

Key rules:
- Prefix `gts.` — required, appears once
- Segments: `[a-z_][a-z0-9_]*` — lowercase ASCII, digits, underscores
- Version: `v<MAJOR>` or `v<MAJOR>.<MINOR>`, no leading zeros
- Trailing `~` — marks this as a type (schema), not an instance
- Chained types supported: `gts.base.seg.ment.type.v1~derived.seg.ment.type.v2~`
- Max total length: 1024 characters

POSIX regex (PostgreSQL-compatible):
```
^gts\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?(?:~[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?)*~$
```

### GTS and `allowed_parents`

GTS does **not** have a built-in mechanism for parent-child containment rules. The `allowed_parents` / `can_be_root` semantics are RG-specific domain behavior.

However, GTS provides the **`x-gts-traits`** extension mechanism — schema-level metadata for processing, runtime, and governance behavior. `allowed_parents` and `can_be_root` are natural candidates for traits:

```json
{
  "$id": "gts://gts.x.system.resource_group.department.v1~",
  "$schema": "gts://gts.x.system.resource_group.type.v1~",
  "x-gts-traits": {
    "can_be_root": false,
    "allowed_parents": ["gts.x.system.resource_group.tenant.v1~"]
  }
}
```

This means a future Type Registry COULD host these traits, making the `resource_group_type` table potentially redundant. See Decision 2 below.

### Team Discussion Context

Question raised: if types are not per-tenant and GTS has a Type Registry, should `resource_group_type` table exist at all? The registry would hold `allowed_parents`/`can_be_root` as `x-gts-traits`.

Developer confirmed the team wants GTS adoption, but Type Registry doesn't exist yet.

---

## Decision 1: GTS Type Paths for Field Values

### Option A: Keep free-form strings (status quo)

**Pros:**
- Zero migration effort
- Simple, familiar values (`tenant`, `User`)
- No external spec dependency
- Short URLs: `/types/tenant`

**Cons:**
- No structural governance — typos, inconsistencies go undetected
- Not interoperable — `User` vs `user` vs `gts.x.idp.users.user.v1~` across modules
- No versioning semantics — what happens when a type evolves?
- No vendor/namespace isolation — collision risk between modules
- Blocks future Type Registry integration

### Option B: GTS type paths (recommended)

**Pros:**
- **Global uniqueness** — vendor + package + namespace + type eliminates collisions
- **Self-documenting** — `gts.x.idp.users.user.v1~` tells you exactly what module/domain this type belongs to
- **Versioned** — type evolution with major/minor semver built into the identifier
- **Interoperable** — same format across all platform modules; `resource_type` in membership matches the type identifier used by the owning module
- **Future-proof** — ready for Type Registry integration without value migration
- **Validated** — regex constraint prevents garbage data at the DB level
- **Consistent with existing usage** — DESIGN.md already uses `gts.x.lms.course.v1~` in AuthZ evaluation examples

**Cons:**
- **Verbose** — `gts.x.system.resource_group.tenant.v1~` (44 chars) vs `tenant` (6 chars)
- **Longer URLs** — `/types/gts.x.system.resource_group.tenant.v1~` but `~` and `.` are valid unreserved URI chars (RFC 3986)
- **Learning curve** — developers must understand the 4-level hierarchy (vendor.package.namespace.type)
- **Upfront naming decisions** — must choose vendor/package/namespace at type creation time
- **Regex validation cost** — CHECK constraint on every INSERT/UPDATE (negligible for write-infrequent type definitions; measurable but small for membership writes)
- **Larger index footprint** — longer keys in B-tree indexes; ~7x more bytes per key for type codes, ~4x for resource_type values compared to current short strings
- **Breaking change** — all existing API consumers must switch to GTS format (acceptable since module is pre-release on a feature branch)

### Decision

**Option B** — adopt GTS type paths.

Rationale: the platform has committed to GTS. Adopting it now (pre-release) avoids a painful data migration later. The verbosity cost is acceptable given the governance and interoperability benefits.

---

## Decision 2: `resource_group_type` Table — Keep, Remove, or Hybrid?

### Option 2A: Keep table, codes become GTS paths

The table stays. `code` column type changes from free-form TEXT to GTS-validated. `allowed_parents` and `can_be_root` remain in the table.

**Pros:**
- Module stays fully autonomous — no external service dependency
- Local DB validation — no network calls for parent-type checks during group create/move
- Transactional consistency — type + group operations in a single DB transaction
- Works today — no dependency on non-existent Type Registry
- Simple mental model — "types live in RG's database"

**Cons:**
- Type definitions fragmented — not discoverable via central registry
- No cross-module governance — nothing prevents different modules from defining conflicting types
- Duplication — when registry arrives, data must be synced or migrated
- CRUD API overhead — RG exposes its own type management endpoints that may become redundant

### Option 2B: Remove table, move everything to Type Registry

`resource_group_type` table is deleted. `allowed_parents`, `can_be_root` live as `x-gts-traits` in GTS type schemas stored in the Type Registry. RG reads type metadata from the registry at validation time.

**Pros:**
- Single source of truth — one place for all type definitions
- Cross-module discovery — other modules can find RG types via registry
- No duplication — registry IS the data store
- Cleaner RG module — fewer tables, no type CRUD API

**Cons:**
- **Type Registry doesn't exist yet** — this option is blocked
- Hard runtime dependency — RG cannot validate parent-type rules without registry availability
- Network latency — every group create/move requires a registry lookup (or cache)
- No transactional consistency — type validation and group write are in different services
- Seed ordering — registry must be populated before RG can accept any writes
- Ownership ambiguity — are `allowed_parents`/`can_be_root` registry concerns or RG concerns? Different modules may define different containment rules for the same types

### Option 2C: Hybrid with phased migration (recommended)

Phase 1 (now): Table stays, codes are GTS type paths. RG is self-contained.
Phase 2 (when registry exists): Types are registered in registry with `x-gts-traits`. RG table becomes a local cache/projection, synced via events.
Phase 3 (optional): Table removed entirely; RG reads from registry (with local cache for performance).

**Pros:**
- Works today without blocking on registry
- GTS compatibility is established from day one
- Smooth migration path — no big-bang rewrite
- RG stays autonomous in phases 1-2
- Each phase is independently valuable and deployable

**Cons:**
- Temporary duplication in phase 2 (local table + registry)
- Additional sync complexity in phase 2
- Must design the local table to be "registry-compatible" from the start (which GTS type paths ensure)

### Decision

**Option 2C** — hybrid phased approach, **with a critical refinement from Decision 3 below**: `allowed_parents` and `can_be_root` stay in RG permanently, not just temporarily.

Rationale: the table stays for now with GTS type paths. This gives us GTS compatibility immediately while keeping the module autonomous. When Type Registry arrives, the transition path is clear because the identifiers are already in GTS format.

---

## Decision 3: Trait Ownership — Where Do `allowed_parents` and `can_be_root` Live Long-Term?

### The Problem

Decision 2C proposes that `allowed_parents`/`can_be_root` might eventually move to Type Registry (phase 3). But this creates a **distributed constraint validation** problem:

> If `allowed_parents` lives in the Registry, and someone removes `org_unit` from `department.allowed_parents`, the Registry cannot validate this safely — it doesn't know whether RG's database contains department groups with org_unit parents.

Currently RG validates this locally: `AllowedParentsViolation` error if removing an allowed parent would break existing hierarchy. In a distributed setup, this requires cross-service coordination.

### Industry Research

We analyzed how real systems handle constraint tightening across service boundaries:

| System | Approach | Lesson |
|---|---|---|
| **Confluent Schema Registry** | Checks schema-vs-schema only, never checks data | Deliberately avoids the problem; consumers may fail at runtime |
| **Kubernetes CRD Ratcheting** | Grandfathers existing resources, gates new ones | Works because API server owns BOTH schema AND data |
| **Terraform** | StateUpgrader migration functions; old data transformed, not validated | "Don't validate old data against new rules — transform it" |
| **Stripe** | Never removes fields, only adds; version gates at the edge | Avoids constraint tightening entirely |
| **Protobuf / buf.build** | `ENUM_VALUE_NO_DELETE` — never delete, only deprecate + reserve | "Deletion is fundamentally unsafe in distributed systems" |
| **Apollo GraphQL** | Checks schema changes against recorded live traffic | Closest analog — validates against actual usage |
| **PostgreSQL** | `NOT VALID` + `VALIDATE CONSTRAINT` two-phase approach | Separates "start enforcing" from "verify existing data" |

### Available Distributed Patterns

**Pattern A: Synchronous Pre-flight** — Registry calls RG before committing.

```
Registry → RG: "Any department groups with org_unit parent?"
RG → Registry: { violations: 47 } → REJECTED
```
Pro: Strong consistency. Con: TOCTOU race, cross-service dependency.

**Pattern B: Saga/Choreography** — Registry publishes `TypeConstraintChangeRequested`, RG approves/rejects.

Pro: Decoupled. Con: Eventual consistency, complex error handling.

**Pattern C: Two-Phase Constraint** (PostgreSQL `NOT VALID` analog):

Phase 1: Mark `org_unit` as `deprecated` in `allowed_parents`. RG rejects NEW groups with this parent but grandfathers existing ones.
Phase 2: Operator validates zero violations in RG, then Registry commits the removal.

Pro: Safe, matches industry patterns. Con: Two-phase protocol complexity.

### Why This Matters: Concrete Example

**Co-located (current design) — safe:**

```
UPDATE allowed_parents → remove org_unit from department?

BEGIN TRANSACTION;
  SELECT COUNT(*) FROM resource_group rg
    JOIN resource_group parent ON rg.parent_id = parent.id
    WHERE rg.group_type = 'department' AND parent.group_type = 'org_unit';
  → 2 violations → ROLLBACK → AllowedParentsViolation

  -- OR: 0 violations → UPDATE resource_group_type SET allowed_parents = ... → COMMIT
END;
```

Constraint and data in the same DB → check is atomic, race conditions impossible.

**Separated (Registry + RG) — unsafe:**

```
1. Registry receives request: remove org_unit from department.allowed_parents
2. Registry calls RG: "any department groups with org_unit parent?"
3. RG responds: "yes, 2 groups"
4. Registry rejects the update

BUT between steps 2 and 4:
  - someone creates a new department with parent=org_unit in RG
  - Registry already got "2 violations" but now there are 3
  - this is a TOCTOU race condition (Time-of-Check vs Time-of-Use)
```

To solve this you need a **distributed lock** or **two-phase commit** — enormous complexity just to remove one string from an array.

### How Real Systems Avoid This Problem

Each system arrived at the same conclusion from a different angle:

**PostgreSQL** — constraints (`CHECK`, `NOT NULL`) live in the same DB as the data. Even the two-phase `NOT VALID` → `VALIDATE CONSTRAINT` trick works only because everything is in one process, one DB.

**Kubernetes** — CRD (type definition) and CR (instances) are stored in the same etcd, handled by the same API server. When a CRD is tightened, the API server can check existing resources in the same call. No cross-service problem.

**Confluent Schema Registry** — deliberately does NOT check data. It only validates schema-vs-schema compatibility (structural). If Kafka data doesn't match the new schema, consumers crash at runtime. Confluent decided this is the **consumer's responsibility**, not the registry's.

**Protobuf / buf.build** — outright **forbids deletion** of enum values (`ENUM_VALUE_NO_DELETE`). Decided it's safer to never allow constraint tightening at all.

**Stripe** — never removes fields from the API. Every API version lives forever.

### The Key Insight

`allowed_parents` is not a property of the type "department" as such. It is a rule about **how RG uses that type in its hierarchy**.

Analogy: the type `User` exists in IDP. But the rule "a user can only be a member of certain groups" is not a property of User. It is a rule of RG.

```
Type Registry answers:   "What is department? What fields does it have?"
RG answers:              "Where can department appear in the hierarchy?"
```

These are **different questions** → **different owners** → **different stores**.

If tomorrow another module (not RG) also builds hierarchies, it may have **its own** `allowed_parents` for the same type `department`. And that's correct — different modules, different containment rules for the same type.

### Option 3A: Traits Move to Registry Eventually (Original 2C Phase 3)

**Pros:**
- Single source of truth for type metadata
- Central discovery of all type constraints

**Cons:**
- **Distributed constraint validation** — every `allowed_parents` change requires cross-service coordination (Pattern A, B, or C above)
- **Ownership ambiguity** — `allowed_parents` constrains RG's data, not the type itself; different modules could have different containment rules for the same type
- **Availability coupling** — RG cannot validate parent-type rules without Registry
- **Transactional boundary broken** — type constraint check and group write are in different services, no ACID
- Patterns A-C add significant complexity for dubious benefit

### Option 3B: `allowed_parents` and `can_be_root` Stay in RG Permanently (recommended)

The Registry owns **type identity and structure** (GTS path, JSON Schema, version).
RG owns **topology constraints** (`allowed_parents`, `can_be_root`) because they constrain RG's instance data.

```
Type Registry:  "gts.x.system.resource_group.department.v1~ is a type with fields {name, ...}"
RG module:      "department groups can be children of tenant or org_unit groups"
```

**Pros:**
- **No distributed constraint validation** — the constraint and the data it governs are in the same service
- **Correct ownership boundary** — `allowed_parents` is a property of "how this type is used in RG hierarchy", not an intrinsic property of the type itself
- **Transactional consistency** — type constraint checks and group writes in one DB transaction
- **Module autonomy** — RG doesn't depend on Registry availability for writes
- **Simplicity** — no cross-service coordination protocols

**Cons:**
- Type topology rules not discoverable via Registry (only via RG API)
- If another module needs similar containment rules, they define their own (acceptable — different modules, different rules)

### Decision

**Option 3B** — `allowed_parents` and `can_be_root` are **permanently RG-owned**.

This refines Decision 2C: Phase 3 ("table removed entirely") is **dropped**. The `resource_group_type` table is not a temporary cache — it is the canonical store for RG topology rules. The Registry, when it arrives, will hold type identity and structure. RG will hold topology constraints. These are complementary, not duplicative.

**Key principle:** Do not separate constraint definitions from the data they constrain.

---

## Decision 5: Type Registry Integration Path

### Current State: GTS Is a Specification, Not a Service

GTS (Global Type System) is a **format specification** — it defines how type identifiers look (`gts.vendor.package.namespace.type.vN~`). It is not a running service, has no API, and stores nothing. Analogous to the UUID specification: it defines the format, not the storage.

For a full GTS ecosystem, the platform needs three layers:

| Layer | What | Status |
|---|---|---|
| **GTS Spec** | Document: regex, format rules, versioning rules | Exists (`gts-spec` repo) |
| **GTS Library** | Rust crate: `GtsTypePath` newtype, parse/validate | Needs to be created (`modkit-gts` or inline in RG SDK) |
| **Type Registry** | Service with REST API: stores GTS type schemas, enables discovery | Does not exist yet |

### Phase 1 (Now): RG Without Type Registry

```
Client → RG API
           │
           ▼
     Regex validation (format only: is this a valid GTS path?)
           │
           ▼
     INSERT INTO resource_group_type / resource_group_membership
```

RG validates **format** (regex at SDK, REST, and DB layers). RG does **not** validate whether the GTS type is registered anywhere — it accepts any syntactically valid GTS path.

- `resource_group_type.code`: RG is the source of truth — the type exists because RG created it
- `resource_group_membership.resource_type`: RG trusts the caller — stores the GTS path as-is

### Phase 2 (Future): RG With Type Registry

```
Client → RG API
           │
           ▼
     Regex validation (format)
           │
           ▼
     GET registry/types/{gts_path}     ← NEW: one HTTP call
     200 OK → type exists in registry
     404    → reject "unknown GTS type"
           │
           ▼
     INSERT INTO resource_group_type / resource_group_membership
```

The **only change** is adding a Registry client call in the RG domain service. Everything else remains identical:

| Component | Changes? | Details |
|---|---|---|
| **DB schema** | No | Same tables, same DOMAIN, same indexes |
| **DB data** | No | Values are already valid GTS paths — nothing to migrate |
| **`allowed_parents`, `can_be_root`** | No | Stay in RG's `resource_group_type` table (Decision 3) |
| **HTTP API contract** | No | Same endpoints, same request/response schemas |
| **Rust SDK contract** | No | Same traits, same `GtsTypePath` type |
| **Format validation** | No | Regex stays at all 3 layers |
| **Existence validation** | **Yes** | +1 HTTP call to Registry per type-write operation |

### What Type Registry Will NOT Do for RG

- Will NOT store `allowed_parents` or `can_be_root` — those are RG's domain constraints (Decision 3)
- Will NOT replace `resource_group_type` table — RG needs local transactional access to topology rules
- Will NOT change the format of GTS paths stored in RG — they are already correct

### What Type Registry Will Do for RG

- **Validate existence**: confirm that `gts.x.idp.users.user.v1~` is a registered type (catches typos)
- **Provide discovery**: RG can query "which types exist under `gts.x.idp.*`?" for tooling/admin UIs
- **Resolve schemas**: if RG ever needs to inspect type structure (field names, etc.), it can fetch the JSON Schema from Registry

### Integration is Optional and Gradual

Registry validation can be:
- **Disabled** (default): RG accepts any valid GTS path — current behavior
- **Warn mode**: RG logs a warning if a type is not in Registry, but accepts it
- **Strict mode**: RG rejects unknown types — requires Registry to be available

This allows gradual rollout without big-bang migration.

---

## Decision 4: Surrogate INT Keys for Internal Storage

### The Problem

GTS type paths are verbose (~30-44 bytes). They appear in:
- `resource_group.group_type` — moderate table (thousands to millions of rows)
- `resource_group_membership.resource_type` — large table (~455M rows projected)
- Composite indexes on both tables

Should we store short INT identifiers internally and map to/from GTS paths at the API boundary?

### Important Distinction

Two different fields with different characteristics:

| Field | Source of types | Cardinality | Volume |
|---|---|---|---|
| `resource_group.group_type` | `resource_group_type` table (FK) | Small (10-50 types) | Moderate |
| `resource_group_membership.resource_type` | External domain types (no FK) | Unbounded | ~455M rows |

### Option 4A: Natural Keys — GTS paths stored directly (current plan)

```sql
resource_group.group_type        = 'gts.x.system.resource_group.department.v1~'  -- ~44 bytes
resource_group_membership.resource_type = 'gts.x.idp.users.user.v1~'             -- ~30 bytes
```

**Pros:**
- Simple — no lookup/join overhead
- Debuggable — `SELECT * FROM resource_group` shows human-readable types directly
- No additional tables or indirection
- `ON UPDATE CASCADE` propagates GTS path renames automatically
- Grep-friendly — can search the DB for a GTS path directly

**Cons:**
- **Index bloat** — `resource_group_membership` composite unique index `(group_id, resource_type, resource_id)`: 16 + 30 + N bytes per entry. At 455M rows, resource_type alone costs ~13 GB in the index
- **Wider rows** — 30-44 extra bytes per row vs INT
- **Slower comparisons** — TEXT comparison vs INT comparison in JOINs, WHERE, and index lookups

### Option 4B: Surrogate SMALLINT for `resource_group_type` only

```sql
resource_group_type (
    id SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    code gts_type_path UNIQUE NOT NULL,
    ...
);

resource_group (
    group_type_id SMALLINT NOT NULL REFERENCES resource_group_type(id),
    ...
);

-- resource_group_membership.resource_type stays as gts_type_path (no FK, external types)
```

**Pros:**
- `resource_group.group_type_id` is 2 bytes instead of ~44 — saves ~42 bytes/row
- Faster FK JOINs (SMALLINT comparison)
- If a type's GTS path changes, only `resource_group_type.code` updates (no cascade needed)
- `allowed_parents` as `SMALLINT[]` instead of `gts_type_path[]` — much smaller

**Cons:**
- Every read requires JOIN to resolve `group_type_id` → GTS path
- Every write requires lookup to resolve GTS path → `group_type_id`
- Raw SQL debugging less readable
- Doesn't help the BIG table (`resource_group_membership`) — its `resource_type` remains TEXT
- Mixed model — some fields use SMALLINT internally, others use TEXT

### Option 4C: Surrogate SMALLINT for ALL type references via lookup table (recommended)

```sql
-- Shared GTS type path → SMALLINT dictionary
CREATE TABLE gts_type_ref (
    id SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    gts_path gts_type_path UNIQUE NOT NULL
);

-- RG types reference gts_type_ref
CREATE TABLE resource_group_type (
    type_ref_id SMALLINT PRIMARY KEY REFERENCES gts_type_ref(id),
    can_be_root BOOLEAN NOT NULL DEFAULT false,
    allowed_parent_ids SMALLINT[] NOT NULL DEFAULT '{}',
    ...
);

-- Groups use INT FK
CREATE TABLE resource_group (
    group_type_id SMALLINT NOT NULL REFERENCES resource_group_type(type_ref_id),
    ...
);

-- Memberships use INT FK to the shared dictionary
CREATE TABLE resource_group_membership (
    group_id UUID NOT NULL,
    resource_type_id SMALLINT NOT NULL REFERENCES gts_type_ref(id),
    resource_id TEXT NOT NULL,
    ...
    UNIQUE (group_id, resource_type_id, resource_id)
);
```

**Pros:**
- **Massive index savings on membership table** — `resource_type_id` is 2 bytes vs ~30 bytes. At 455M rows:
  - Composite unique index: saves ~13 GB
  - Reverse lookup index `(resource_type_id, resource_id)`: saves ~13 GB
  - Row storage: saves ~13 GB
  - **Total savings: ~38 GB**
- Uniform model — all type references are SMALLINT internally
- Type path renames are single-row updates in `gts_type_ref`, zero cascade
- SMALLINT comparison faster in all query paths

**Cons:**
- **Extra lookup table** — `gts_type_ref` must be populated before memberships can be created
- **Write overhead** — every membership write needs: GTS path → `SELECT id FROM gts_type_ref WHERE gts_path = $1` → use SMALLINT. Mitigated by:
  - Small table (100s of entries), always in buffer cache
  - `INSERT ... ON CONFLICT DO NOTHING RETURNING id` for auto-registration
- **Read overhead** — every response must JOIN to resolve SMALLINT → GTS path. Mitigated by:
  - Tiny table, always cached
  - Application-level LRU cache (bidirectional: path↔id)
- **Debugging** — raw queries need JOINs to see type names
- **Slightly more complex migration/seeding** — types must be registered in `gts_type_ref` first
- **Added table** — one more entity to manage

### Option 4D: Keep natural keys (no surrogate)

Same as 4A. Accept the storage/index cost. It's within acceptable bounds.

**Rationale for this being viable:**
- 12 GB extra index space is ~3% of projected total (~455M × ~250 bytes avg row = ~110 GB)
- Modern SSDs handle this easily
- No query complexity overhead
- PostgreSQL B-tree prefix compression (deduplication) reduces actual impact
- Premature optimization vs. simplicity tradeoff

### Decision

**Option 4D** — natural GTS TEXT keys for now.

Rationale: simplicity wins at this stage — no JOINs, no lookup table, human-readable queries. The storage overhead is acceptable (~3% of projected total).

**Future optimization note:** if benchmarks show index/storage pressure, migrate to Option 4C (SMALLINT surrogate keys). See Appendix C for the ready-made schema. The migration is mechanical and does not affect the API/SDK (they always use GTS paths). Estimated savings: **~38 GB (~25% of projected membership storage)** at 455M rows.

---

## Impact Analysis

### 3.1 Database Schema (migration.sql)

#### New: `gts_type_path` DOMAIN

```sql
CREATE DOMAIN gts_type_path AS TEXT
    CHECK (
        LENGTH(VALUE) <= 1024
        AND VALUE ~ '^gts\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?(?:~[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?)*~$'
    );
```

Why DOMAIN and not per-column CHECK:
- Reusable across all 4 fields (including `gts_type_path[]` arrays — PG validates each element automatically)
- Single place to update if GTS spec evolves
- Semantically expressive — the type system documents intent

#### Column Changes

| Table.Column | Before | After |
|---|---|---|
| `resource_group_type.code` | `TEXT PRIMARY KEY CHECK (code = LOWER(code))` | `gts_type_path PRIMARY KEY` |
| `resource_group_type.allowed_parents` | `TEXT[] NOT NULL DEFAULT '{}'` | `gts_type_path[] NOT NULL DEFAULT '{}'` |
| `resource_group.group_type` | `TEXT NOT NULL CHECK (group_type = LOWER(group_type))` | `gts_type_path NOT NULL` (FK inherits validation) |
| `resource_group_membership.resource_type` | `TEXT NOT NULL` | `gts_type_path NOT NULL` |

Old CHECK constraints (`code = LOWER(code)`, `group_type = LOWER(group_type)`) are removed — the DOMAIN regex enforces lowercase implicitly (char class `[a-z_][a-z0-9_]*`).

#### Index Impact

Indexes remain structurally unchanged. Longer keys increase B-tree size:
- Type code: ~6 bytes → ~44 bytes average (~7x)
- Resource type: ~4 bytes → ~30 bytes average
- For projected volumes (~455M membership rows), resource_type adds ~12 GB in indexes — acceptable (~3% of projected total ~110 GB)

> **Future optimization:** surrogate SMALLINT keys (Option 4C, Appendix C) would save ~38 GB (~25%) by replacing TEXT keys with 2-byte IDs. Migration is mechanical and API-transparent.

### 3.2 HTTP API (openapi.yaml)

#### New Reusable Schema Component

```yaml
GtsTypePath:
  type: string
  maxLength: 1024
  pattern: '^gts\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?(?:~[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?)*~$'
  description: |
    GTS type path — globally unique, versioned type identifier.
    Format: gts.<vendor>.<package>.<namespace>.<type>.v<MAJOR>[.<MINOR>]~
    Spec: https://github.com/GlobalTypeSystem/gts-spec
  example: gts.x.system.resource_group.tenant.v1~
```

#### Field Changes

All fields that were previously free-form strings now reference `$ref: '#/components/schemas/GtsTypePath'`:

- `Type.code`, `CreateTypeRequest.code`
- `Type.allowed_parents[*]`, `CreateTypeRequest.allowed_parents[*]`, `UpdateTypeRequest.allowed_parents[*]`
- `Group.group_type`, `CreateGroupRequest.group_type`, `UpdateGroupRequest.group_type`, `GroupWithDepth.group_type`
- `Membership.resource_type`

#### Path Parameters

- `TypeCode` parameter: `example: department` → `example: gts.x.system.resource_group.department.v1~`
- `MembershipResourceType` parameter: `example: User` → `example: gts.x.idp.users.user.v1~`

#### URL Examples

Before:
```
GET  /api/resource-group/v1/types/department
POST /api/resource-group/v1/memberships/{group_id}/User/R4
GET  /api/resource-group/v1/groups?$filter=group_type eq 'department'
```

After:
```
GET  /api/resource-group/v1/types/gts.x.system.resource_group.department.v1~
POST /api/resource-group/v1/memberships/{group_id}/gts.x.idp.users.user.v1~/R4
GET  /api/resource-group/v1/groups?$filter=group_type eq 'gts.x.system.resource_group.department.v1~'
```

Note: `~` is an unreserved URI character (RFC 3986 §2.3), `.` is a valid sub-delimiter in path segments. No URL encoding needed.

#### Example Values Update

| Context | Before | After |
|---|---|---|
| Type code | `tenant` | `gts.x.system.resource_group.tenant.v1~` |
| Type code | `department` | `gts.x.system.resource_group.department.v1~` |
| Type code | `branch` | `gts.x.system.resource_group.branch.v1~` |
| allowed_parents | `[tenant]` | `[gts.x.system.resource_group.tenant.v1~]` |
| allowed_parents | `[department]` | `[gts.x.system.resource_group.department.v1~]` |
| group_type | `department` | `gts.x.system.resource_group.department.v1~` |
| resource_type | `User` | `gts.x.idp.users.user.v1~` |

### 3.3 Rust SDK (DESIGN.md)

#### New: `GtsTypePath` Newtype

```rust
/// GTS type path — validated, globally unique type identifier.
/// Format: gts.<vendor>.<package>.<namespace>.<type>.v<MAJOR>[.<MINOR>]~
///
/// Validates on construction; all SDK methods accept `&GtsTypePath`
/// instead of `&str` for type-safe API boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GtsTypePath(String);

impl GtsTypePath {
    /// Parse and validate a GTS type path.
    /// Returns `Err` if the value does not match the GTS type path format.
    pub fn new(value: impl Into<String>) -> Result<Self, GtsTypePathError> { ... }

    /// Returns the inner string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}
```

Open question: should `GtsTypePath` live in the RG SDK crate or in a shared platform crate (e.g., `modkit-gts` or `gts-core`)? Since GTS is platform-wide, a shared crate is preferable. If it doesn't exist yet, RG SDK can define it locally and re-export, migrating to the shared crate later.

#### SDK Model Changes

```rust
// ── Before ──────────────────────────────────────────────────────────
pub struct ResourceGroupType {
    pub code: String,                       // free-form
    pub can_be_root: bool,
    pub allowed_parents: Vec<String>,       // free-form
}

pub struct ResourceGroup {
    pub group_type: String,                 // free-form
    ...
}

pub struct ResourceGroupMembership {
    pub resource_type: String,              // free-form
    ...
}

// ── After ───────────────────────────────────────────────────────────
pub struct ResourceGroupType {
    pub code: GtsTypePath,                  // validated GTS type path
    pub can_be_root: bool,
    pub allowed_parents: Vec<GtsTypePath>,  // each element validated
}

pub struct ResourceGroup {
    pub group_type: GtsTypePath,            // validated
    ...
}

pub struct ResourceGroupMembership {
    pub resource_type: GtsTypePath,         // validated
    ...
}
```

Full list of affected structs:

| Struct | Field | `String` → `GtsTypePath` |
|---|---|---|
| `ResourceGroupType` | `code` | yes |
| `ResourceGroupType` | `allowed_parents` | `Vec<String>` → `Vec<GtsTypePath>` |
| `CreateTypeRequest` | `code` | yes |
| `CreateTypeRequest` | `allowed_parents` | `Vec<String>` → `Vec<GtsTypePath>` |
| `UpdateTypeRequest` | `allowed_parents` | `Vec<String>` → `Vec<GtsTypePath>` |
| `ResourceGroup` | `group_type` | yes |
| `ResourceGroupWithDepth` | `group_type` | yes |
| `CreateGroupRequest` | `group_type` | yes |
| `UpdateGroupRequest` | `group_type` | yes |
| `ResourceGroupMembership` | `resource_type` | yes |
| `AddMembershipRequest` | `resource_type` | yes |
| `RemoveMembershipRequest` | `resource_type` | yes |

#### Trait Signature Changes

```rust
#[async_trait]
pub trait ResourceGroupClient: Send + Sync {
    // code parameter: &str → &GtsTypePath
    async fn get_type(&self, ctx: &SecurityContext, code: &GtsTypePath) -> Result<ResourceGroupType, ResourceGroupError>;
    async fn update_type(&self, ctx: &SecurityContext, code: &GtsTypePath, request: UpdateTypeRequest) -> Result<ResourceGroupType, ResourceGroupError>;
    async fn delete_type(&self, ctx: &SecurityContext, code: &GtsTypePath) -> Result<(), ResourceGroupError>;

    // All other signatures unchanged (they use request structs which contain GtsTypePath internally)
    // ...
}
```

#### Error Type Addition

```rust
pub enum ResourceGroupError {
    // ... existing variants ...

    /// GTS type path validation failed.
    InvalidGtsTypePath {
        field: String,
        value: String,
        reason: String,
    },
}
```

#### Usage Examples (Before → After)

```rust
// ── Before ──────────────────────────────────────────────────────────
rg.add_membership(&ctx, AddMembershipRequest {
    group_id: group_a,
    resource_type: "User".to_string(),
    resource_id: "task_1".to_string(),
}).await?;

// ── After ───────────────────────────────────────────────────────────
rg.add_membership(&ctx, AddMembershipRequest {
    group_id: group_a,
    resource_type: GtsTypePath::new("gts.x.idp.users.user.v1~")?,
    resource_id: "task_1".to_string(),
}).await?;
```

### 3.4 Validation Strategy (Defense in Depth)

Validation happens at three layers:

| Layer | Mechanism | Purpose |
|---|---|---|
| **Rust SDK** | `GtsTypePath::new()` constructor | Fail fast at API boundary; compile-time type safety |
| **REST API** | OpenAPI `pattern` + request validation middleware | Reject invalid requests before hitting domain logic |
| **Database** | `gts_type_path` DOMAIN CHECK constraint | Last line of defense; prevents data corruption from any write path |

### 3.5 Backward Compatibility

This is a **breaking change** for all three layers (DB, HTTP, SDK).

Acceptable because:
- Module is pre-release (feature branch, no production data)
- No published SDK crate yet
- No external consumers depend on current API

If this were a post-release change, it would require:
- DB data migration (`UPDATE resource_group_type SET code = 'gts.x.system.resource_group.' || code || '.v1~'`)
- API versioning (v2 endpoints)
- SDK major version bump

---

## Consequences

### Positive

1. RG module is GTS-compatible from first release — no future migration needed
2. Type identifiers are globally unique, versioned, and self-documenting
3. `resource_type` in memberships unambiguously identifies the domain type, enabling cross-module queries
4. Ready for Type Registry integration (phase 2-3) without value changes
5. DB-level validation prevents data corruption regardless of write path
6. Consistent with platform direction (team consensus on GTS adoption)

### Negative

1. Longer values increase storage and index size (~12 GB for projected membership volume). Can be reduced to near-zero with SMALLINT surrogate keys (Option 4C, Appendix C) — saves ~38 GB (~25%)
2. More verbose API calls — developers must construct GTS paths instead of simple slugs
3. URL paths become longer (but remain valid without encoding)
4. Regex validation adds marginal CPU cost on writes
5. Developers must learn GTS naming conventions (vendor, package, namespace, type, version)

### Risks

1. **GTS spec evolution** — if the spec changes, the regex in the DOMAIN must be updated. Mitigation: DOMAIN is a single update point; GTS spec is pre-1.0 but the identifier format is stable.
2. **Naming convention disputes** — teams may disagree on vendor/package/namespace choices. Mitigation: establish a platform-wide naming guide (out of scope for this ADR).
3. **Performance at scale** — longer keys in composite indexes (membership table). Mitigation: benchmark during capacity planning phase; prefix compression in PostgreSQL B-trees mitigates impact. If needed, migrate to SMALLINT surrogate keys (Option 4C, Appendix C) for ~38 GB savings.

---

## Appendix A: GTS Type Path Examples for RG

### Resource Group Types (resource_group_type.code)

```
gts.x.system.resource_group.tenant.v1~
gts.x.system.resource_group.department.v1~
gts.x.system.resource_group.branch.v1~
gts.x.system.resource_group.faculty.v1~
gts.x.system.resource_group.team.v1~
```

### Resource Types (resource_group_membership.resource_type)

```
gts.x.idp.users.user.v1~
gts.x.lms.courses.course.v1~
gts.x.lms.quizzes.quiz.v1~
gts.x.cms.documents.document.v1~
gts.x.webstore.products.product.v1~
```

### Chained (Derived) Type Example

```
gts.x.core.entities.entity.v1~x.idp.users.user.v1~
```

## Appendix B: migration.sql (current — natural GTS TEXT keys)

```sql
CREATE DOMAIN gts_type_path AS TEXT
    CHECK (
        LENGTH(VALUE) <= 1024
        AND VALUE ~ '^gts\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?(?:~[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?)*~$'
    );

CREATE TABLE resource_group_type (
    code gts_type_path PRIMARY KEY,
    can_be_root BOOLEAN NOT NULL DEFAULT false,
    allowed_parents gts_type_path[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    CONSTRAINT chk_type_has_placement
        CHECK (can_be_root OR cardinality(allowed_parents) >= 1)
);

CREATE TABLE resource_group (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID,
    group_type gts_type_path NOT NULL,
    name TEXT NOT NULL,
    tenant_id UUID NOT NULL,
    external_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    CONSTRAINT fk_resource_group_type
        FOREIGN KEY (group_type) REFERENCES resource_group_type(code)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT fk_resource_group_parent
        FOREIGN KEY (parent_id) REFERENCES resource_group(id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE TABLE resource_group_membership (
    group_id UUID NOT NULL,
    resource_type gts_type_path NOT NULL,
    resource_id TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_resource_group_membership_group_id
        FOREIGN KEY (group_id) REFERENCES resource_group(id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_membership_unique
        UNIQUE (group_id, resource_type, resource_id)
);
```

## Appendix C: migration.sql (future optimization — SMALLINT surrogate keys)

```sql
CREATE DOMAIN gts_type_path AS TEXT
    CHECK (
        LENGTH(VALUE) <= 1024
        AND VALUE ~ '^gts\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?(?:~[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*\.v(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))?)*~$'
    );

-- ── GTS type path dictionary ────────────────────────────────────────────
-- Shared lookup: every GTS type path used by RG gets a compact INT id.
-- Auto-populated on first use (INSERT ... ON CONFLICT DO NOTHING RETURNING id).
CREATE TABLE gts_type_ref (
    id SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    gts_path gts_type_path UNIQUE NOT NULL
);

COMMENT ON TABLE gts_type_ref
    IS 'Dictionary mapping GTS type paths to compact INT ids for internal storage. API/SDK always use gts_path; INT ids are internal only.';

-- ── Resource group types ────────────────────────────────────────────────
CREATE TABLE resource_group_type (
    type_ref_id SMALLINT PRIMARY KEY REFERENCES gts_type_ref(id),
    can_be_root BOOLEAN NOT NULL DEFAULT false,
    allowed_parent_ids SMALLINT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    CONSTRAINT chk_type_has_placement
        CHECK (can_be_root OR cardinality(allowed_parent_ids) >= 1)
);

COMMENT ON TABLE resource_group_type
    IS 'Resource group type definitions. type_ref_id → gts_type_ref.id; allowed_parent_ids[] → gts_type_ref.id.';

-- ── Resource groups ─────────────────────────────────────────────────────
CREATE TABLE resource_group (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID,
    group_type_id SMALLINT NOT NULL REFERENCES resource_group_type(type_ref_id),
    name TEXT NOT NULL,
    tenant_id UUID NOT NULL,
    external_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    CONSTRAINT fk_resource_group_parent
        FOREIGN KEY (parent_id) REFERENCES resource_group(id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

-- ── Memberships ─────────────────────────────────────────────────────────
CREATE TABLE resource_group_membership (
    group_id UUID NOT NULL,
    resource_type_id SMALLINT NOT NULL REFERENCES gts_type_ref(id),
    resource_id TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_resource_group_membership_group_id
        FOREIGN KEY (group_id) REFERENCES resource_group(id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_membership_unique
        UNIQUE (group_id, resource_type_id, resource_id)
);
```

### Option 4C: Storage Comparison

| Table.Column | Option 4D (TEXT) | Option 4C (SMALLINT) | Savings per row |
|---|---|---|---|
| `resource_group.group_type` | ~44 bytes | 2 bytes | ~42 bytes |
| `resource_group_membership.resource_type` | ~30 bytes | 2 bytes | ~28 bytes |
| `resource_group_type.allowed_parents` | ~44 × N bytes | 2 × N bytes | ~42 × N bytes |

At 455M membership rows: **~38 GB total savings** (rows + indexes).

### Option 4C: Application-Level Caching

The `gts_type_ref` table is small (100s of entries) and changes rarely. The application layer maintains a bidirectional LRU cache:

```rust
/// In-process bidirectional cache for gts_path ↔ SMALLINT resolution.
/// Populated on startup, updated on type registration.
struct GtsTypeRefCache {
    path_to_id: HashMap<GtsTypePath, i16>,
    id_to_path: HashMap<i16, GtsTypePath>,
}
```

Cache invalidation is trivial: entries are append-only (new types added, never removed or renamed — GTS paths are immutable identifiers; new versions get new paths).
