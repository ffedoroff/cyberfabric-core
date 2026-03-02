# Resource Group — Database Size Analysis

## Test Environment

PostgreSQL 17 (Docker), generated data:

| Table | Rows |
|---|---|
| `resource_group_type` | 20 |
| `resource_group` | 100,000 |
| `resource_group_closure` | 306,175 |
| `resource_group_membership` | 200,000 |

---

## Table Sizes (with all indexes)

| Table | Data | Indexes+TOAST | Total | Avg Row |
|---|---|---|---|---|
| `resource_group_membership` | 17 MB | 32 MB | **49 MB** | 91 B |
| `resource_group` | 12 MB | 15 MB | **27 MB** | 125 B |
| `resource_group_closure` | 20 MB | 16 MB | **36 MB** | 68 B |
| `resource_group_type` | 8 KB | 40 KB | **48 KB** | 409 B |
| **Total** | **49 MB** | **63 MB** | **112 MB** | — |

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

## Index Sizes (12 indexes, 63 MB total)

| Index | Table | Size | B/row |
|---|---|---|---|
| `uq_resource_group_membership_unique` (btree) | membership | 14 MB | 74 |
| `idx_rgm_resource_type_id` (btree) | membership | 9.8 MB | 50 |
| `idx_rgc_ancestor_depth` (btree) | closure | 8.6 MB | 29 |
| `idx_rgm_resource_id` (btree) | membership | 8.3 MB | 43 |
| `idx_rgc_descendant_id` (btree) | closure | 7.8 MB | 26 |
| `idx_rg_name` (btree) | groups | 4.6 MB | 47 |
| `resource_group_pkey` (btree) | groups | 4.3 MB | 44 |
| `idx_rg_external_id` (btree) | groups | 4.3 MB | 44 |
| `idx_rg_parent_id` (btree) | groups | 1.8 MB | 18 |
| `idx_rg_group_type` (btree) | groups | 712 KB | 7 |
| `resource_group_type_pkey` (btree) | types | 16 KB | — |
| `idx_resource_group_type_code_lower` (btree) | types | 16 KB | — |

### Index overhead ratio

| Table | Data | Indexes | Ratio (idx/data) |
|---|---|---|---|
| `resource_group` | 12 MB | 15 MB | **1.25×** |
| `resource_group_closure` | 20 MB | 16 MB | **0.80×** |
| `resource_group_membership` | 17 MB | 32 MB | **1.88×** |

---

## Query Performance (before → after indexes)

| Query | Before | After | Speedup | Index Used |
|---|---|---|---|---|
| 4h parent_id eq | 4.3 ms | 0.31 ms | **14×** | idx_rg_parent_id |
| 4j parent_id in | 5.1 ms | 0.09 ms | **57×** | idx_rg_parent_id |
| 4k name eq | 6.2 ms | 0.14 ms | **44×** | idx_rg_name |
| 4m name in | 6.0 ms | 0.07 ms | **86×** | idx_rg_name |
| 4q external_id eq | 7.5 ms | 0.09 ms | **83×** | idx_rg_external_id |
| 4s external_id in | 7.7 ms | 0.06 ms | **128×** | idx_rg_external_id |
| 4w depth eq | 47.7 ms | 0.43 ms | **111×** | idx_rgc_descendant_id |
| 4y_le depth le | 121.2 ms | 0.09 ms | **1347×** | idx_rgc_descendant_id |
| 4ad ancestor+depth | 8.2 ms | 0.48 ms | **17×** | idx_rgc_ancestor_depth |
| 5h resource_id eq | 8.7 ms | 0.09 ms | **97×** | idx_rgm_resource_id |
| 5j resource_id in | 12.6 ms | 0.14 ms | **90×** | idx_rgm_resource_id |
| 5q type+resource_id | 8.0 ms | 0.09 ms | **89×** | idx_rgm_resource_type_id |

---

## Production Extrapolation

### Assumptions

- **1.5M tenants** (each tenant is a resource_group row)
- **303.5M users** (each user = 1-2 group memberships → **~455M memberships**)
- Groups hierarchy: ~1.5M tenants + organizational subgroups
  - Estimate **~5M total groups** (tenants + departments/teams/regions/etc.)
  - Average hierarchy depth ~3 → **~15.4M closure rows** (self-links + ancestry chains)

### Projected Table Data

| Table | Rows | Data Size | Calc |
|---|---|---|---|
| `resource_group` | 5,000,000 | **625 MB** | 5M × 125 B |
| `resource_group_closure` | 15,400,000 | **1.05 GB** | 15.4M × 68 B |
| `resource_group_membership` | 455,000,000 | **41.4 GB** | 455M × 91 B |
| `resource_group_type` | ~50 | **~8 KB** | negligible |
| **Total data** | — | **~43 GB** | — |

### Projected Index Sizes

| Index | Table | Rows | Projected Size | Calc |
|---|---|---|---|---|
| `resource_group_pkey` | groups | 5M | **220 MB** | 5M × 44 B |
| `idx_rg_parent_id` | groups | 5M | **90 MB** | 5M × 18 B |
| `idx_rg_name` | groups | 5M | **235 MB** | 5M × 47 B |
| `idx_rg_external_id` | groups | 5M | **220 MB** | 5M × 44 B |
| `idx_rg_group_type` | groups | 5M | **35 MB** | 5M × 7 B |
| `idx_rgc_descendant_id` | closure | 15.4M | **400 MB** | 15.4M × 26 B |
| `idx_rgc_ancestor_depth` | closure | 15.4M | **447 MB** | 15.4M × 29 B |
| `uq_resource_group_membership_unique` | membership | 455M | **33.7 GB** | 455M × 74 B |
| `idx_rgm_resource_id` | membership | 455M | **19.6 GB** | 455M × 43 B |
| `idx_rgm_resource_type_id` | membership | 455M | **22.8 GB** | 455M × 50 B |
| **Total indexes** | — | — | **~77.6 GB** | — |

### Total Projected Storage

| Component | Size |
|---|---|
| Table data | ~43 GB |
| All indexes | ~78 GB |
| **Grand total** | **~121 GB** |

### Breakdown by Table (data + indexes)

| Table | Data | Indexes | Total | % |
|---|---|---|---|---|
| `resource_group_membership` | 41.4 GB | 76.1 GB | **117.5 GB** | 97% |
| `resource_group_closure` | 1.05 GB | 0.85 GB | **1.9 GB** | 1.6% |
| `resource_group` | 625 MB | 800 MB | **1.4 GB** | 1.2% |
| `resource_group_type` | ~8 KB | ~32 KB | **~40 KB** | ~0% |

### Key Observations

1. **Membership table dominates** — 455M rows, ~117 GB (data + 3 indexes).
   This is 97% of the total database. Any optimization here has the biggest impact.

2. **Index-to-data ratio is 1.81×** — indexes (78 GB) are 1.81× bigger than data (43 GB).
   This is reasonable for btree-only indexes with UUID keys.

3. **Closure table is very manageable** — 1.9 GB total. Indexes turned 50-121ms queries into <0.5ms.

4. **Memory requirements** — at 121 GB total:
   - Minimum: 24 GB RAM (shared_buffers = 6 GB, OS cache handles the rest)
   - Recommended: 48 GB RAM (shared_buffers = 12 GB) to keep hot indexes in memory
   - The membership UQ index (33.7 GB) is the most critical for caching

5. **Partitioning candidate** — `resource_group_membership` by `tenant_id`:
   - 1.5M tenants × ~300 memberships each
   - Hash partitioning (e.g., 128 partitions) → ~920 MB per partition
   - Enables partition pruning for tenant-scoped queries
   - Reduces index sizes per partition (faster maintenance, better cache hit rates)
