# Feature: Tenant Closure CDC Pipeline

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-tenant-closure-cdc`
- [ ] `p2` - `cpt-cf-resource-group-feature-tenant-closure-cdc`

**Status**: PLANNED — migration ready, CDC consumer pending

## 1. Feature Context

### 1.1 Overview

Implement the CDC (Change Data Capture) pipeline that populates the `tenant_closure` local projection table in the resource-group module's database. The `tenant_closure` table is required by `InTenantSubtree` SQL subqueries (Feature 0007) but is currently empty — the migration creates the schema, this feature populates and maintains the data.

### 1.2 Purpose

Feature 0007 introduced `InTenantSubtree` predicate support with SQL:
```sql
SELECT descendant_id FROM tenant_closure
WHERE ancestor_id = ? AND barrier = 0 AND descendant_status IN ('active')
```

The `tenant_closure` table exists (migration `m20260310_000002_tenant_closure_projection`) but contains no data. The authoritative source of tenant hierarchy is the **tenant-resolver** module, which manages tenant CRUD and hierarchy operations. The RG module needs a read-only projection of that data, kept in sync via a CDC pipeline.

### 1.3 Architecture

```
tenant-resolver module                    resource-group module
┌─────────────────────┐                  ┌─────────────────────┐
│ Tenant CRUD         │                  │ tenant_closure       │
│ Hierarchy ops       │                  │ (local projection)   │
│                     │    CDC pipeline  │                      │
│ modkit_outbox       │ ──────────────→  │ CDC Consumer         │
│ (transactional      │   outbox queue   │ (TransactionalHandler│
│  outbox enqueue)    │                  │  or DecoupledHandler) │
└─────────────────────┘                  └─────────────────────┘
```

The pipeline uses `modkit-db` transactional outbox infrastructure:
- **Producer** (tenant-resolver): enqueues `TenantHierarchyChanged` events in `modkit_outbox_incoming` atomically with hierarchy mutations
- **Consumer** (resource-group): processes events via outbox handler, updates `tenant_closure` projection

### 1.4 Actors

| Actor | Role in Feature |
|-------|-----------------|
| Tenant Resolver Module | Source of truth for tenant hierarchy; produces CDC events |
| Resource Group Module | Consumes CDC events; maintains `tenant_closure` local projection |
| Outbox Sequencer | Background task: assigns sequence numbers to incoming messages |
| Outbox Processor | Background task: dispatches messages to consumer handler |

### 1.5 References

- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md)
- **Feature 0007**: `cpt-cf-resource-group-feature-authz-constraint-types` — defines `InTenantSubtree` SQL that reads `tenant_closure`
- **INTEGRATION_AUTHZ.md**: [INTEGRATION_AUTHZ.md](../INTEGRATION_AUTHZ.md) — C4 (tenant_closure migration), Runtime Verification Checklist
- **modkit-db outbox**: `libs/modkit-db/src/outbox/` — full transactional outbox pipeline
- **Outbox examples**: `libs/modkit-db/examples/outbox_transactional.rs`, `outbox_batch_multi_queue.rs`
- **Dependencies**:
  - [x] `p1` - `cpt-cf-resource-group-feature-authz-constraint-types` (Feature 7 — InTenantSubtree SQL)
  - [ ] Tenant Resolver module — must enqueue hierarchy change events

## 2. Actor Flows (CDSL)

### Tenant Hierarchy Change → Projection Update

- [ ] `p2` - **ID**: `cpt-cf-resource-group-flow-tenant-closure-cdc`

**Actor**: Tenant Resolver Module (producer), Resource Group Module (consumer)

**Trigger**: Any tenant hierarchy mutation (create tenant, move tenant, delete tenant, change tenant status, change barrier)

**Success Scenarios**:
- Tenant hierarchy change is atomically enqueued as outbox message
- Consumer processes message and updates `tenant_closure` projection
- Subsequent `InTenantSubtree` queries reflect the updated hierarchy

**Error Scenarios**:
- Consumer fails to process message → retried (at-least-once delivery)
- Consumer fails repeatedly → message moved to dead-letter queue
- Projection temporarily stale during processing delay

**Steps**:
1. [ ] - `p2` - Tenant Resolver performs hierarchy mutation in a database transaction - `inst-cdc-1`
2. [ ] - `p2` - Within the same transaction, enqueue `TenantHierarchyChanged` event to `modkit_outbox_incoming` via `Outbox::enqueue()` - `inst-cdc-2`
3. [ ] - `p2` - Transaction commits — hierarchy change and outbox message are atomic - `inst-cdc-3`
4. [ ] - `p2` - Outbox Sequencer claims incoming message, assigns partition sequence number, writes to `modkit_outbox_outgoing` - `inst-cdc-4`
5. [ ] - `p2` - Outbox Processor dispatches message to RG module's `TenantClosureHandler` - `inst-cdc-5`
6. [ ] - `p2` - Handler computes closure delta from event payload and applies to `tenant_closure` table - `inst-cdc-6`
7. [ ] - `p2` - **IF** handler fails — message is retried per outbox retry policy - `inst-cdc-7`
8. [ ] - `p2` - **IF** handler fails permanently — message moved to dead-letter queue for operator inspection - `inst-cdc-8`

### Initial Seed / Full Resync

- [ ] `p2` - **ID**: `cpt-cf-resource-group-flow-tenant-closure-seed`

**Actor**: Operator / Module Init

**Trigger**: First deployment, data corruption recovery, or explicit resync command

**Steps**:
1. [ ] - `p2` - Trigger full resync (module init flag or operator command) - `inst-seed-1`
2. [ ] - `p2` - Read full tenant hierarchy from tenant-resolver via SDK trait call - `inst-seed-2`
3. [ ] - `p2` - Truncate `tenant_closure` table - `inst-seed-3`
4. [ ] - `p2` - Compute transitive closure from hierarchy data - `inst-seed-4`
5. [ ] - `p2` - Bulk insert all closure rows into `tenant_closure` - `inst-seed-5`
6. [ ] - `p2` - Resume normal CDC processing - `inst-seed-6`

## 3. Processes / Business Logic (CDSL)

### Event Schema

- [ ] `p2` - **ID**: `cpt-cf-resource-group-algo-cdc-event-schema`

**Queue name**: `tenant_hierarchy_changes`

**Partition key**: `ancestor_tenant_id` (changes to the same subtree are processed in order)

**Event payload** (JSON):

```json
{
  "event_type": "TenantHierarchyChanged",
  "version": 1,
  "tenant_id": "uuid",
  "change_type": "created | moved | deleted | status_changed | barrier_changed",
  "ancestor_id": "uuid | null",
  "old_ancestor_id": "uuid | null",
  "status": "active | suspended | ...",
  "barrier": 0
}
```

### Closure Delta Computation

- [ ] `p2` - **ID**: `cpt-cf-resource-group-algo-closure-delta`

**Input**: `TenantHierarchyChanged` event

**Output**: Set of `(ancestor_id, descendant_id, barrier, descendant_status)` rows to insert/delete

**Steps**:
1. [ ] - `p2` - **IF** `change_type == created` - `inst-delta-1`
   1. [ ] - `p2` - Insert self-row: `(tenant_id, tenant_id, 0, status)` - `inst-delta-1a`
   2. [ ] - `p2` - For each ancestor of `ancestor_id` in current closure: insert `(ancestor, tenant_id, max(ancestor_barrier, barrier), status)` - `inst-delta-1b`
2. [ ] - `p2` - **IF** `change_type == deleted` - `inst-delta-2`
   1. [ ] - `p2` - Delete all rows where `descendant_id = tenant_id` - `inst-delta-2a`
   2. [ ] - `p2` - Delete all rows where `ancestor_id = tenant_id` (subtree entries) - `inst-delta-2b`
3. [ ] - `p2` - **IF** `change_type == moved` - `inst-delta-3`
   1. [ ] - `p2` - Delete closure entries from old ancestor path - `inst-delta-3a`
   2. [ ] - `p2` - Recompute closure entries for new ancestor path - `inst-delta-3b`
4. [ ] - `p2` - **IF** `change_type == status_changed` - `inst-delta-4`
   1. [ ] - `p2` - Update `descendant_status` for all rows where `descendant_id = tenant_id` - `inst-delta-4a`
5. [ ] - `p2` - **IF** `change_type == barrier_changed` - `inst-delta-5`
   1. [ ] - `p2` - Recompute barrier values for affected subtree entries - `inst-delta-5a`

## 4. States (CDSL)

### Projection Sync State

| State | Description | Transition |
|-------|-------------|------------|
| `uninitialized` | Table exists but no data (first deployment) | → `syncing` on init/seed |
| `syncing` | Full resync in progress | → `synced` on completion |
| `synced` | Projection matches source within CDC lag | → `stale` on prolonged lag |
| `stale` | CDC consumer behind (>N messages or >T seconds) | → `synced` when caught up |
| `failed` | Dead-letter messages exist; projection may be inconsistent | → `syncing` on manual resync |

## 5. Definitions of Done

### CDC Producer in Tenant Resolver

- [ ] `p2` - **ID**: `cpt-cf-resource-group-dod-cdc-producer`

Tenant Resolver **MUST** enqueue `TenantHierarchyChanged` events to `modkit_outbox_incoming` atomically with every hierarchy mutation (create, move, delete, status change, barrier change). The event **MUST** be enqueued within the same database transaction as the mutation. The partition key **MUST** be the root ancestor of the affected subtree to ensure ordered processing.

### CDC Consumer in Resource Group

- [ ] `p2` - **ID**: `cpt-cf-resource-group-dod-cdc-consumer`

Resource Group module **MUST** register an outbox handler for the `tenant_hierarchy_changes` queue during module init. The handler **MUST** implement `TransactionalHandler` (exactly-once within DB) to update `tenant_closure` atomically with the outbox ack. The handler **MUST** be idempotent — processing the same event twice produces the same projection state.

### Full Resync Capability

- [ ] `p2` - **ID**: `cpt-cf-resource-group-dod-full-resync`

The module **MUST** support a full resync operation that truncates `tenant_closure` and rebuilds it from the tenant-resolver's current state. This **SHOULD** be triggered on first deployment (empty table detection) and **MAY** be triggered by operator command.

## 6. Acceptance Criteria

- [ ] Tenant hierarchy mutations in tenant-resolver produce outbox messages
- [ ] RG module consumes messages and updates `tenant_closure` projection
- [ ] `InTenantSubtree` queries return correct results after CDC propagation
- [ ] Barrier mode filtering works correctly with CDC-populated data
- [ ] Tenant status changes propagate to `descendant_status` column
- [ ] Full resync produces identical projection to incremental CDC
- [ ] Dead-letter messages are created for permanently failing events
- [ ] Handler is idempotent — duplicate delivery produces correct state
- [ ] CDC consumer starts automatically on module init
- [ ] Empty table triggers automatic full resync on first startup

## 7. Dependencies

### Required Infrastructure

| Dependency | Provider | Status |
|-----------|----------|--------|
| `modkit_outbox` tables | modkit-db migration | Available (`preview-outbox` feature) |
| `tenant_closure` table | RG migration `m20260310_000002` | DONE |
| Tenant hierarchy SDK trait | tenant-resolver-sdk | Needs `list_full_hierarchy()` method |
| Outbox `TransactionalHandler` | modkit-db | Available |
| Outbox `QueueBuilder` | modkit-db | Available |

### Cross-Module Coordination

1. **tenant-resolver** must add outbox enqueue calls to all hierarchy mutation paths
2. **tenant-resolver-sdk** must expose a `list_full_hierarchy()` method for initial seed
3. **resource-group** registers outbox consumer during module init (after outbox tables are migrated)

## 8. Non-Applicable Domains

- **REST API changes**: None — CDC is a background data pipeline, no new endpoints
- **AuthZ changes**: None — `InTenantSubtree` SQL is already implemented (Feature 0007); this feature only populates the data
- **MTLS**: Not applicable — CDC is internal module communication via shared database
