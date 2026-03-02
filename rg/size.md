# Resource Group — Database Size Analysis

## Test Environment

PostgreSQL 17 (Docker), generated data:

| Table | Rows |
|---|---|
| `resource_group_type` | 20 |
| `resource_group` | 100,000 |
| `resource_group_closure` | 307,321 |
| `resource_group_membership` | 200,000 |

---

## Table Sizes (100K groups, 200K memberships)

| Table | Data | Indexes+TOAST | Total | Avg Row |
|---|---|---|---|---|
| `resource_group_membership` | 17 MB | 14 MB | **32 MB** | 91 B |
| `resource_group_closure` | 20 MB | 32 KB | **20 MB** | 68 B |
| `resource_group` | 12 MB | 4.3 MB | **17 MB** | 125 B |
| `resource_group_type` | 8 KB | 40 KB | **48 KB** | 409 B |
| **Total** | **49 MB** | **18 MB** | **~69 MB** | — |

## Column Widths (avg bytes from pg_stats)

### resource_group (125 B/row)

| Column | Avg Bytes | Type |
|---|---|---|
| `id` | 16 | UUID |
| `parent_id` | 16 | UUID (nullable) |
| `group_type` | 8 | TEXT |
| `name` | 15 | TEXT |
| `tenant_id` | 16 | UUID |
| `external_id` | 13 | TEXT |
| `created` | 8 | TIMESTAMPTZ |
| `modified` | 8 | TIMESTAMPTZ |
| Row overhead | ~25 | (tuple header + alignment) |

### resource_group_closure (68 B/row)

| Column | Avg Bytes | Type |
|---|---|---|
| `ancestor_id` | 16 | UUID |
| `descendant_id` | 16 | UUID |
| `depth` | 4 | INTEGER |
| Row overhead | ~32 | (tuple header + alignment) |

### resource_group_membership (91 B/row)

| Column | Avg Bytes | Type |
|---|---|---|
| `group_id` | 16 | UUID |
| `resource_type` | 5 | TEXT |
| `resource_id` | 13 | TEXT |
| `tenant_id` | 16 | UUID |
| `created` | 8 | TIMESTAMPTZ |
| Row overhead | ~33 | (tuple header + alignment) |

## Index Sizes

| Index | Table | Size | Bytes/Row |
|---|---|---|---|
| `uq_resource_group_membership_unique` | membership | 14 MB | 74 B |
| `resource_group_pkey` | groups | 4.3 MB | 44 B |
| `resource_group_type_pkey` | types | 16 KB | — |
| `idx_resource_group_type_code_lower` | types | 16 KB | — |

Note: `resource_group_closure` has **NO indexes** — the 20 MB is pure heap data.

---

## Production Extrapolation

### Assumptions

- **1.5M tenants** (each tenant is a resource_group row)
- **303.5M users** (each user = 1-2 group memberships → **~455M memberships**)
- Groups hierarchy: ~1.5M tenants + organizational subgroups
  - Estimate **~5M total groups** (tenants + departments/teams/regions/etc.)
  - Average hierarchy depth ~3 → **~20M closure rows** (self-links + ancestry chains)

### Ratios from test data

| Metric | Test (100K) | Per Row | Production (5M groups) |
|---|---|---|---|
| Groups | 100K | 125 B | 5M |
| Closure | 307K (3.07× groups) | 68 B | ~15.4M |
| Memberships | 200K | 91 B | 455M |

### Projected Table Sizes

| Table | Rows | Data Size | Calc |
|---|---|---|---|
| `resource_group` | 5,000,000 | **625 MB** | 5M × 125 B |
| `resource_group_closure` | 15,400,000 | **1.05 GB** | 15.4M × 68 B |
| `resource_group_membership` | 455,000,000 | **41.4 GB** | 455M × 91 B |
| `resource_group_type` | ~50 | **~8 KB** | negligible |
| **Total data** | — | **~43 GB** | — |

### Projected Index Sizes

| Index | Rows | Size | Calc |
|---|---|---|---|
| `resource_group_pkey` (btree, UUID) | 5M | **220 MB** | 5M × 44 B |
| `uq_resource_group_membership_unique` (btree, 3 cols) | 455M | **33.7 GB** | 455M × 74 B |
| Closure indexes (if added) | 15.4M | **~1.2 GB** | ~80 B/row × 15.4M (2-3 indexes) |
| Additional recommended indexes | — | **~5-8 GB** | parent_id, name, external_id, resource_id, trgm |

### Total Projected Storage

| Component | Size |
|---|---|
| Table data | ~43 GB |
| Existing indexes (PK + UQ) | ~34 GB |
| Recommended new indexes | ~6-9 GB |
| **Total** | **~83-86 GB** |

### Breakdown by Table

| Table | Data + Indexes | % of Total |
|---|---|---|
| `resource_group_membership` | **~75 GB** | 88% |
| `resource_group_closure` | **~2.3 GB** | 3% |
| `resource_group` | **~2-4 GB** | 3-5% |
| New indexes (closure, search) | **~6-9 GB** | 7-10% |

### Key Observations

1. **Membership table dominates** — 455M rows at ~166 B/row (data + UQ index) = ~75 GB.
   This is the table that needs the most attention for partitioning/sharding strategy.

2. **Closure table is manageable** — 15.4M rows, ~1 GB data. But without indexes,
   every query does full scan. Adding 2-3 indexes adds ~1.2 GB but makes depth queries
   go from 50-120ms → <1ms.

3. **Groups table is small** — 5M rows, <1 GB. PK index sufficient for most queries.
   Additional indexes (parent_id, name, external_id) add ~1.5 GB.

4. **Memory considerations** — at 83 GB total, this requires dedicated DB instance.
   PostgreSQL shared_buffers should be ~20 GB (25% of 80 GB RAM) for efficient caching.
   The membership UQ index (33.7 GB) alone won't fit in memory on small instances.

5. **Partitioning candidate** — `resource_group_membership` by `tenant_id` (range or hash)
   would help limit scan scope and allow parallel maintenance. Each of 1.5M tenants
   has ~300 memberships on average.
