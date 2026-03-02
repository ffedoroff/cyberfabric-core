-- ============================================================================
-- Resource Group API — SQL queries for every GET endpoint
-- ============================================================================
--
-- EXPLAIN ANALYZE on PostgreSQL 17 with 100K groups, 307K closure, 200K memberships.
-- 20 resource_group_type rows (reference table, always tiny).
--
-- Existing indexes (from constraints only):
--   resource_group_type:       PK(code), UNIQUE(lower(code))
--   resource_group:            PK(id)
--   resource_group_closure:    — NONE —
--   resource_group_membership: UNIQUE(group_id, resource_type, resource_id)
--
-- Legend:
--   [PK]        — primary key lookup
--   [UQ]        — unique index exact match
--   [UQ-P]      — unique index leftmost-prefix match
--   [BITMAP]    — Bitmap Index Scan + Bitmap Heap Scan
--   [SEQ]       — Seq Scan (no usable index)
--   [PAR-SEQ]   — Parallel Seq Scan (no usable index, PG parallelized)
--   [TINY]      — reference table <100 rows, Seq Scan is fine
--


-- ############################################################################
-- 1. GET /types/{code}
-- ############################################################################

-- Index Scan using resource_group_type_pkey [PK] — 0.030 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1;


-- ############################################################################
-- 2. GET /types  (listTypes)
--    Filters: code (eq, ne, in, contains, startswith, endswith)
-- ############################################################################

-- 2a. No filter
-- Index Scan using resource_group_type_pkey (ORDER BY matches PK) [PK] — 0.021 ms
SELECT code, parents
  FROM resource_group_type
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2b. code eq
-- Index Scan using resource_group_type_pkey [PK] — 0.025 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1
 LIMIT $top OFFSET $skip;

-- 2c. code ne
-- Index Scan using resource_group_type_pkey + Filter [PK] — 0.024 ms
-- ne on tiny table still uses PK scan (20 rows)
SELECT code, parents
  FROM resource_group_type
 WHERE code <> $1
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2d. code in
-- Bitmap Index Scan on resource_group_type_pkey → Bitmap Heap Scan → Sort [BITMAP] — 0.056 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = ANY($1::text[])
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2e. code startswith
-- Seq Scan + Filter (code ~~ 'prefix%') → Sort [TINY] — 0.051 ms
-- PK is text_ops, not text_pattern_ops, so LIKE can't use it. Fine for 20-row table.
SELECT code, parents
  FROM resource_group_type
 WHERE code LIKE $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2f. code contains
-- Seq Scan + Filter (code ~~* '%mid%') → Sort [TINY] — 0.058 ms
-- ILIKE needs pg_trgm GIN. Not needed on 20-row reference table.
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2g. code endswith
-- Seq Scan + Filter (code ~~* '%suffix') → Sort [TINY] — 0.048 ms
-- Same as 2f, ILIKE without GIN. Fine for reference table.
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1
 ORDER BY code
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- 3. GET /groups/{group_id}
-- ############################################################################

-- Index Scan using resource_group_pkey [PK] — 0.040 ms
SELECT g.id          AS group_id,
       g.parent_id,
       g.group_type,
       g.name,
       g.tenant_id,
       g.external_id
  FROM resource_group g
 WHERE g.id = $1;


-- ############################################################################
-- 4. GET /groups  (listGroups)
--    Filters: group_id, group_type, parent_id, depth, name, external_id
--
--    `depth` requires JOIN to resource_group_closure.
--    All other filters are on resource_group columns directly.
-- ############################################################################

-- ---- Single-field filters (no depth) --------------------------------------

-- 4a. No filter
-- Index Scan using resource_group_pkey (ORDER BY g.id matches PK) [PK] — 0.059 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4b. group_id eq
-- Index Scan using resource_group_pkey [PK] — 0.035 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = $1
 LIMIT $top OFFSET $skip;

-- 4c. group_id ne
-- Index Scan using resource_group_pkey + Filter (id <>) [PK] — 0.081 ms
-- ORDER BY id allows PK scan even with ne filter (skip matching rows)
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4d. group_id in
-- Bitmap Index Scan on resource_group_pkey → Bitmap Heap Scan → Sort [BITMAP] — 0.073 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4e. group_type eq
-- Index Scan using resource_group_pkey + Filter (group_type =) [PK] — 0.201 ms
-- Scans PK in order, filters rows. Works because LIMIT stops early.
-- Without LIMIT or with low selectivity → needs btree(group_type). ~11K tenant rows out of 100K.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4f. group_type ne
-- Index Scan using resource_group_pkey + Filter (group_type <>) [PK] — 0.071 ms
-- ne is low selectivity, PK scan + filter + LIMIT works fine
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4g. group_type in
-- Index Scan using resource_group_pkey + Filter (group_type = ANY) [PK] — 0.122 ms
-- Same pattern as 4e — PK scan in order, filter, early stop via LIMIT
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4h. parent_id eq
-- Seq Scan + Filter (parent_id =) → Sort [SEQ] — 4.331 ms
-- NO INDEX on parent_id. Full table scan on 100K rows.
-- NEEDS: CREATE INDEX idx_rg_parent_id ON resource_group(parent_id);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4i. parent_id ne
-- Index Scan using resource_group_pkey + Filter (parent_id <>) [PK] — 0.060 ms
-- ne is wide, PK scan with LIMIT stops early
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4j. parent_id in
-- Seq Scan + Filter (parent_id = ANY) → Sort [SEQ] — 5.075 ms
-- NO INDEX on parent_id. Full table scan.
-- NEEDS: CREATE INDEX idx_rg_parent_id ON resource_group(parent_id);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4k. name eq
-- Seq Scan + Filter (name =) → Sort [SEQ] — 6.182 ms
-- NO INDEX on name. Full table scan on 100K rows.
-- NEEDS: CREATE INDEX idx_rg_name ON resource_group(name);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4l. name ne
-- Index Scan using resource_group_pkey + Filter (name <>) [PK] — 0.077 ms
-- ne is wide, PK scan with LIMIT
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4m. name in
-- Seq Scan + Filter (name = ANY) → Sort [SEQ] — 5.954 ms
-- NO INDEX on name. Full table scan.
-- NEEDS: CREATE INDEX idx_rg_name ON resource_group(name);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4n. name startswith
-- Index Scan using resource_group_pkey + Filter (name ~~ 'prefix%') [PK] — 0.169 ms
-- PK scan in order + LIMIT stops early. For high-offset pagination → needs btree(name text_pattern_ops).
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4o. name contains
-- Index Scan using resource_group_pkey + Filter (name ~~* '%mid%') [PK] — 0.192 ms
-- PK scan + LIMIT early stop. Without LIMIT → needs GIN(name gin_trgm_ops).
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4p. name endswith
-- Seq Scan + Filter (name ~~* '%suffix') → Sort [SEQ] — 34.438 ms
-- Full table scan, no early stop possible. ILIKE '%suffix' can't use btree.
-- NEEDS: GIN(name gin_trgm_ops) with pg_trgm extension
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4q. external_id eq
-- Seq Scan + Filter (external_id =) → Sort [SEQ] — 7.496 ms
-- NO INDEX on external_id. Full table scan.
-- NEEDS: CREATE INDEX idx_rg_external_id ON resource_group(external_id);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4r. external_id ne
-- Index Scan using resource_group_pkey + Filter (external_id <>) [PK] — 0.062 ms
-- ne is wide, PK scan with LIMIT
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4s. external_id in
-- Seq Scan + Filter (external_id = ANY) → Sort [SEQ] — 7.669 ms
-- NO INDEX on external_id. Full table scan.
-- NEEDS: CREATE INDEX idx_rg_external_id ON resource_group(external_id);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4t. external_id startswith
-- Index Scan using resource_group_pkey + Filter (external_id ~~ 'prefix%') [PK] — 0.083 ms
-- PK scan + LIMIT early stop
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4u. external_id contains
-- Index Scan using resource_group_pkey + Filter (external_id ~~* '%mid%') [PK] — 55.616 ms
-- PK scan but ILIKE is expensive per-row on 100K rows. Even with LIMIT,
-- if matches are sparse planner scans many pages.
-- NEEDS: GIN(external_id gin_trgm_ops)
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4v. external_id endswith
-- Seq Scan + Filter (external_id ~~* '%suffix') → Sort [SEQ] — 33.324 ms
-- Full table scan. ILIKE '%suffix' can't use btree.
-- NEEDS: GIN(external_id gin_trgm_ops)
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- ---- depth filters (require JOIN to closure) ------------------------------
-- resource_group_closure has NO indexes — all depth queries do Parallel Seq Scan on closure.

-- 4w. depth eq
-- Parallel Seq Scan on closure + Hash Join to resource_group [PAR-SEQ] — 47.692 ms
-- Scans entire 307K closure table, filters depth=1 (~100K rows), joins to groups.
-- NEEDS: CREATE INDEX idx_rgc_descendant_id ON resource_group_closure(descendant_id);
-- NEEDS: CREATE INDEX idx_rgc_depth ON resource_group_closure(depth);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4x. depth ne
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 56.789 ms
-- Same full scan, ne filter keeps most rows
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth gt
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 42.186 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth > $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth ge
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 57.106 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth lt
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 57.651 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth < $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth le
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 121.210 ms
-- Worst closure query: depth<=1 returns ~200K rows (self-links + direct children),
-- Hash Join materializes all of them before Sort + Limit.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4z. depth range: ge AND le
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 53.525 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= $1 AND c.depth <= $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters ----------------------------------------------------

-- 4aa. group_type + parent_id
-- Seq Scan + Filter (group_type AND parent_id) → Sort [SEQ] — 5.799 ms
-- Neither column indexed. Full table scan.
-- NEEDS: btree(parent_id) or composite btree(group_type, parent_id)
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.parent_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ab. group_type + name startswith
-- Index Scan using resource_group_pkey + Filter [PK] — 0.183 ms
-- PK scan in order, filters both conditions, LIMIT stops early
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.name LIKE $2 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ac. group_type + external_id eq
-- Seq Scan + Filter (group_type AND external_id) → Sort [SEQ] — 7.484 ms
-- No index on external_id. Full table scan.
-- NEEDS: btree(external_id) or composite btree(group_type, external_id)
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.external_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ad. ancestor_id + depth (children of X at specific depth)
-- Parallel Seq Scan on closure (filter ancestor_id + depth) → Nested Loop → PK lookup [PAR-SEQ] — 8.195 ms
-- Closure scanned fully (~307K rows), then PK lookup per matching row.
-- NEEDS: CREATE INDEX idx_rgc_ancestor_depth ON resource_group_closure(ancestor_id, depth);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.ancestor_id = $1
   AND c.depth = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ae. group_type + depth
-- Parallel Seq Scan on closure + Hash Join [PAR-SEQ] — 20.027 ms
-- Both closure (no index) and group_type (no index) scanned.
-- NEEDS: btree(group_type) + indexes on closure
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE g.group_type = $1
   AND c.depth = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4af. ancestor_id + group_type + depth
-- Parallel Seq Scan on closure (filter ancestor+depth) → Nested Loop → PK lookup + Filter [PAR-SEQ] — 9.152 ms
-- Closure full scan is the bottleneck. PK lookup on resource_group is fast.
-- NEEDS: CREATE INDEX idx_rgc_ancestor_depth ON resource_group_closure(ancestor_id, depth);
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.ancestor_id = $1
   AND g.group_type = $2
   AND c.depth = $3
 ORDER BY g.id
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- 5. GET /memberships  (listMemberships)
--    Filters: group_id (eq, ne, in), resource_type (eq, ne, in),
--             resource_id (eq, ne, in, contains, startswith, endswith)
-- ############################################################################

-- ---- Single-field filters -------------------------------------------------

-- 5a. No filter
-- Index Scan using uq_resource_group_membership_unique [UQ] — 0.086 ms
-- ORDER BY (group_id, resource_type, resource_id) matches unique index column order exactly.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5b. group_id eq
-- Bitmap Index Scan on uq_resource_group_membership_unique → Bitmap Heap Scan → Sort [BITMAP] — 0.108 ms
-- Uses leftmost prefix of UNIQUE(group_id, resource_type, resource_id)
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5c. group_id ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.070 ms
-- Scans index in order, filters out one group_id, LIMIT stops early
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5d. group_id in
-- Bitmap Index Scan on uq_resource_group_membership_unique → Bitmap Heap Scan → Sort [BITMAP] — 0.111 ms
-- Leftmost prefix match for each group_id in array
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = ANY($1::uuid[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5e. resource_type eq
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.111 ms
-- resource_type is 2nd column in UNIQUE index. Planner scans index in order,
-- filters resource_type per row. LIMIT stops early.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
 ORDER BY m.group_id, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5f. resource_type ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.068 ms
-- ne is wide, index scan + LIMIT
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5g. resource_type in
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.108 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5h. resource_id eq
-- Parallel Seq Scan + Filter (resource_id =) [PAR-SEQ] — 8.675 ms
-- resource_id is 3rd column in UNIQUE index — can't use index without group_id + resource_type prefix.
-- Full table scan on 200K rows.
-- NEEDS: CREATE INDEX idx_rgm_resource_id ON resource_group_membership(resource_id);
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = $1
 ORDER BY m.group_id, m.resource_type
 LIMIT $top OFFSET $skip;

-- 5i. resource_id ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.072 ms
-- ne is wide, index scan + LIMIT
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5j. resource_id in
-- Parallel Seq Scan + Filter (resource_id = ANY) [PAR-SEQ] — 12.586 ms
-- Same as 5h — can't use UNIQUE index for resource_id alone.
-- NEEDS: CREATE INDEX idx_rgm_resource_id ON resource_group_membership(resource_id);
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5k. resource_id startswith
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.084 ms
-- Planner chose index scan in order + LIMIT early stop. LIKE filter applied per row.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id LIKE $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5l. resource_id contains
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.740 ms
-- ILIKE scanned via index order + LIMIT. Matches found quickly due to common substring.
-- Worst case (rare substring) → full scan. NEEDS: GIN(resource_id gin_trgm_ops).
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5m. resource_id endswith
-- Parallel Seq Scan + Filter (resource_id ~~* '%suffix') [PAR-SEQ] — 34.014 ms
-- Full scan, can't use any index for ILIKE suffix.
-- NEEDS: GIN(resource_id gin_trgm_ops)
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters -----------------------------------------------------

-- 5n. group_id + resource_type
-- Index Scan using uq_resource_group_membership_unique (first 2 columns) [UQ-P] — 0.039 ms
-- Exact prefix match on UNIQUE(group_id, resource_type, resource_id)
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
 ORDER BY m.resource_id
 LIMIT $top OFFSET $skip;

-- 5o. group_id + resource_id
-- Index Scan using uq_resource_group_membership_unique (group_id prefix) + Filter [UQ-P] — 0.041 ms
-- Uses group_id prefix, filters resource_id per row
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id = $2
 ORDER BY m.resource_type
 LIMIT $top OFFSET $skip;

-- 5p. group_id + resource_type + resource_id (exact match)
-- Index Scan using uq_resource_group_membership_unique (all 3 columns) [UQ] — 0.047 ms
-- Perfect unique index match, single row lookup
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
   AND m.resource_id = $3;

-- 5q. resource_type + resource_id (no group_id)
-- Parallel Seq Scan + Filter (resource_type AND resource_id) [PAR-SEQ] — 8.005 ms
-- UNIQUE index starts with group_id — useless without it. Full scan.
-- NEEDS: CREATE INDEX idx_rgm_resource_type_id ON resource_group_membership(resource_type, resource_id);
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
   AND m.resource_id = $2
 ORDER BY m.group_id
 LIMIT $top OFFSET $skip;

-- 5r. group_id + resource_id startswith
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan + Filter → Sort [BITMAP] — 0.085 ms
-- Uses group_id prefix of UNIQUE index, applies LIKE filter after
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id LIKE $2 || '%'
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- SUMMARY — queries that need indexes (sorted by severity)
-- ############################################################################
--
-- CRITICAL (>40ms, full scan on 307K closure):
--   4y_le  121.2 ms  Parallel Seq Scan on closure — depth <= N
--   4y_ge   57.1 ms  Parallel Seq Scan on closure — depth >= N
--   4y_lt   57.7 ms  Parallel Seq Scan on closure — depth < N
--   4x      56.8 ms  Parallel Seq Scan on closure — depth <> N
--   4z      53.5 ms  Parallel Seq Scan on closure — depth range
--   4w      47.7 ms  Parallel Seq Scan on closure — depth = N
--   4y_gt   42.2 ms  Parallel Seq Scan on closure — depth > N
--
--   FIX: CREATE INDEX idx_rgc_descendant_id ON resource_group_closure(descendant_id);
--        CREATE INDEX idx_rgc_ancestor_depth ON resource_group_closure(ancestor_id, depth);
--
-- HIGH (>30ms, full scan + ILIKE on 100K/200K rows):
--   4u      55.6 ms  PK scan + ILIKE '%mid%' on external_id (100K rows)
--   4p      34.4 ms  Seq Scan + ILIKE '%suffix' on name (100K rows)
--   4v      33.3 ms  Seq Scan + ILIKE '%suffix' on external_id (100K rows)
--   5m      34.0 ms  Parallel Seq Scan + ILIKE '%suffix' on resource_id (200K rows)
--
--   FIX: CREATE EXTENSION IF NOT EXISTS pg_trgm;
--        CREATE INDEX idx_rg_name_trgm ON resource_group USING GIN(name gin_trgm_ops);
--        CREATE INDEX idx_rg_extid_trgm ON resource_group USING GIN(external_id gin_trgm_ops);
--        CREATE INDEX idx_rgm_resid_trgm ON resource_group_membership USING GIN(resource_id gin_trgm_ops);
--
-- MEDIUM (5-13ms, Seq Scan on equality/IN without index):
--   5j      12.6 ms  Parallel Seq Scan — resource_id IN (200K rows)
--   4af      9.2 ms  Parallel Seq Scan on closure — ancestor + type + depth
--   5h       8.7 ms  Parallel Seq Scan — resource_id = (200K rows)
--   4ad      8.2 ms  Parallel Seq Scan on closure — ancestor + depth
--   5q       8.0 ms  Parallel Seq Scan — resource_type + resource_id (200K rows)
--   4s       7.7 ms  Seq Scan — external_id IN (100K rows)
--   4q       7.5 ms  Seq Scan — external_id = (100K rows)
--   4ac      7.5 ms  Seq Scan — group_type + external_id (100K rows)
--   4k       6.2 ms  Seq Scan — name = (100K rows)
--   4m       6.0 ms  Seq Scan — name IN (100K rows)
--   4aa      5.8 ms  Seq Scan — group_type + parent_id (100K rows)
--   4j       5.1 ms  Seq Scan — parent_id IN (100K rows)
--   4h       4.3 ms  Seq Scan — parent_id = (100K rows)
--
--   FIX: CREATE INDEX idx_rg_parent_id ON resource_group(parent_id);
--        CREATE INDEX idx_rg_name ON resource_group(name);
--        CREATE INDEX idx_rg_external_id ON resource_group(external_id);
--        CREATE INDEX idx_rgm_resource_id ON resource_group_membership(resource_id);
--        CREATE INDEX idx_rgm_resource_type_id ON resource_group_membership(resource_type, resource_id);
--
-- FAST (<1ms, use existing PK or UNIQUE index):
--   All remaining queries — already use PK scan + LIMIT early stop or UNIQUE index prefix.
