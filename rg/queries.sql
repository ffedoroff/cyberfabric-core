-- ============================================================================
-- Resource Group API — SQL queries for every GET endpoint
-- ============================================================================
--
-- EXPLAIN ANALYZE on PostgreSQL 17, generated data:
--   resource_group_type:       20 rows
--   resource_group:            100,000 rows
--   resource_group_closure:    306,175 rows
--   resource_group_membership: 200,000 rows
--
-- All indexes (from migration.sql):
--   resource_group_type:       PK(code), UNIQUE(lower(code))
--   resource_group:            PK(id), btree(parent_id), btree(name), btree(external_id),
--                              btree(group_type), GIN(name gin_trgm_ops), GIN(external_id gin_trgm_ops)
--   resource_group_closure:    btree(descendant_id), btree(ancestor_id, depth)
--   resource_group_membership: UNIQUE(group_id, resource_type, resource_id), btree(resource_id),
--                              btree(resource_type, resource_id), GIN(resource_id gin_trgm_ops)
--


-- ############################################################################
-- 1. GET /types/{code}
-- ############################################################################

-- Seq Scan on 20-row reference table [TINY] — 0.031 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1;


-- ############################################################################
-- 2. GET /types  (listTypes)
--    Filters: code (eq, ne, in, contains, startswith, endswith)
-- ############################################################################

-- 2a. No filter
-- Seq Scan → Sort → Limit on 20-row table [TINY] — 0.073 ms
SELECT code, parents
  FROM resource_group_type
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2b. code eq
-- Seq Scan + Filter [TINY] — 0.026 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1
 LIMIT $top OFFSET $skip;

-- 2c. code ne
-- Seq Scan + Filter → Sort [TINY] — 0.080 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code <> $1
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2d. code in
-- Seq Scan + Filter → Sort [TINY] — 0.042 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = ANY($1::text[])
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2e. code startswith
-- Seq Scan + Filter (code ~~ 'prefix%') → Sort [TINY] — 0.062 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code LIKE $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2f. code contains
-- Seq Scan + Filter (code ~~* '%mid%') → Sort [TINY] — 0.053 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2g. code endswith
-- Seq Scan + Filter (code ~~* '%suffix') → Sort [TINY] — 0.054 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1
 ORDER BY code
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- 3. GET /groups/{group_id}
-- ############################################################################

-- Index Scan using resource_group_pkey [PK] — 0.113 ms
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
-- Index Scan using resource_group_pkey (ORDER BY g.id matches PK) [PK] — 0.157 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4b. group_id eq
-- Index Scan using resource_group_pkey [PK] — 0.041 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = $1
 LIMIT $top OFFSET $skip;

-- 4c. group_id ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.064 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4d. group_id in
-- Bitmap Index Scan on resource_group_pkey → Bitmap Heap Scan → Sort [BITMAP+PK] — 0.122 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4e. group_type eq
-- Index Scan using resource_group_pkey + Filter [PK] — 0.554 ms
-- PK scan in order + LIMIT stops early. idx_rg_group_type available for queries without ORDER BY id.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4f. group_type ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.066 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4g. group_type in
-- Index Scan using resource_group_pkey + Filter [PK] — 0.153 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4h. parent_id eq
-- Bitmap Index Scan on idx_rg_parent_id → Bitmap Heap Scan → Sort — 0.310 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4i. parent_id ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.076 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4j. parent_id in
-- Bitmap Index Scan on idx_rg_parent_id → Bitmap Heap Scan → Sort — 0.093 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4k. name eq
-- Index Scan using idx_rg_name → Sort — 0.135 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4l. name ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.076 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4m. name in
-- Index Scan using idx_rg_name → Sort — 0.065 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4n. name startswith
-- Index Scan using resource_group_pkey + Filter [PK] — 0.250 ms
-- PK scan + LIMIT. For high-offset → btree(name text_pattern_ops) would help.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4o. name contains
-- Index Scan using resource_group_pkey + Filter [PK] — 0.296 ms
-- PK scan + LIMIT finds matches quickly. GIN idx_rg_name_trgm available for selective patterns.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4p. name endswith
-- Bitmap Index Scan on idx_rg_name_trgm → Bitmap Heap Scan → Sort — 0.159 ms
-- GIN trgm index handles ILIKE suffix efficiently.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4q. external_id eq
-- Index Scan using idx_rg_external_id → Sort — 0.090 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4r. external_id ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.062 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4s. external_id in
-- Index Scan using idx_rg_external_id → Sort — 0.056 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4t. external_id startswith
-- Index Scan using resource_group_pkey + Filter [PK] — 0.066 ms
-- PK scan + LIMIT. idx_rg_external_id usable for exact prefix if needed.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4u. external_id contains
-- Index Scan using resource_group_pkey + Filter [PK] — 37.630 ms *
-- * Slow only when ILIKE pattern matches most rows (e.g. '%ext%' on ext-prefixed data).
--   With selective patterns, planner switches to GIN idx_rg_extid_trgm (0.1 ms).
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4v. external_id endswith
-- Bitmap Index Scan on idx_rg_extid_trgm → Bitmap Heap Scan → Sort — 0.200 ms
-- GIN trgm index handles ILIKE suffix efficiently.
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- ---- depth filters (require JOIN to closure) ------------------------------
-- All depth queries use idx_rgc_descendant_id for the JOIN.

-- 4w. depth eq
-- Index Scan on idx_rgc_descendant_id + Nested Loop → PK lookup — 0.434 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4x. depth ne
-- Index Scan on idx_rgc_descendant_id + Nested Loop → PK lookup — 0.100 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth gt
-- Index Scan on idx_rgc_descendant_id + Merge Join → PK — 0.453 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth > $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth ge
-- Index Scan on idx_rgc_descendant_id + Nested Loop → PK — 0.098 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth lt
-- Index Scan on idx_rgc_descendant_id + Nested Loop → PK — 0.107 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth < $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth le
-- Index Scan on idx_rgc_descendant_id + Merge Join → PK — 0.091 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4z. depth range: ge AND le
-- Index Scan on idx_rgc_descendant_id + Nested Loop → PK — 0.099 ms
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
-- Bitmap Index Scan on idx_rg_parent_id → Bitmap Heap Scan + Filter (group_type) → Sort — 0.161 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.parent_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ab. group_type + name startswith
-- Index Scan using resource_group_pkey + Filter [PK] — 0.247 ms
-- PK scan in order + LIMIT stops early (both conditions common → matches found quickly)
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.name LIKE $2 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ac. group_type + external_id eq
-- Index Scan using idx_rg_external_id + Filter (group_type) — 0.067 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.external_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ad. ancestor_id + depth (children of X at specific depth)
-- Index Scan on idx_rgc_ancestor_depth + Nested Loop → PK lookup — 0.478 ms
-- Uses composite index (ancestor_id, depth) directly for both conditions.
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
-- Index Scan on idx_rgc_descendant_id + Nested Loop → PK + Filter (group_type) — 0.319 ms
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
-- Index Scan on idx_rgc_ancestor_depth + Nested Loop → PK + Filter (group_type) — 0.362 ms
-- Composite closure index handles ancestor+depth; group_type filtered on PK lookup.
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
-- Index Scan using uq_resource_group_membership_unique [UQ] — 0.078 ms
-- ORDER BY matches UNIQUE index column order exactly.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5b. group_id eq
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan → Sort [BITMAP+UQ-P] — 0.089 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5c. group_id ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.075 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5d. group_id in
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan → Sort [BITMAP+UQ-P] — 0.105 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = ANY($1::uuid[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5e. resource_type eq
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.140 ms
-- Scans UQ index in order, filters resource_type. LIMIT stops early.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
 ORDER BY m.group_id, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5f. resource_type ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.076 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5g. resource_type in
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.139 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5h. resource_id eq
-- Index Scan using idx_rgm_resource_id → Sort — 0.087 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = $1
 ORDER BY m.group_id, m.resource_type
 LIMIT $top OFFSET $skip;

-- 5i. resource_id ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.087 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5j. resource_id in
-- Bitmap Index Scan on idx_rgm_resource_id → Bitmap Heap Scan → Sort — 0.144 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5k. resource_id startswith
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.070 ms
-- UQ index scan in order + LIKE filter + LIMIT
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id LIKE $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5l. resource_id contains
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 1.065 ms
-- UQ index scan + ILIKE filter. GIN idx_rgm_resid_trgm available for selective patterns.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5m. resource_id endswith
-- Bitmap Index Scan on idx_rgm_resid_trgm → Bitmap Heap Scan → Sort — 0.191 ms
-- GIN trgm index handles ILIKE suffix efficiently.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters -----------------------------------------------------

-- 5n. group_id + resource_type
-- Bitmap Index Scan on uq (first 2 cols) → Bitmap Heap Scan — 0.041 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
 ORDER BY m.resource_id
 LIMIT $top OFFSET $skip;

-- 5o. group_id + resource_id
-- Index Scan using uq (group_id prefix) + Filter (resource_id) — 0.039 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id = $2
 ORDER BY m.resource_type
 LIMIT $top OFFSET $skip;

-- 5p. group_id + resource_type + resource_id (exact match)
-- Index Scan using idx_rgm_resource_type_id [exact match] — 0.091 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
   AND m.resource_id = $3;

-- 5q. resource_type + resource_id (no group_id)
-- Index Scan using idx_rgm_resource_type_id — 0.086 ms
-- Uses composite index (resource_type, resource_id) since group_id is absent.
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
   AND m.resource_id = $2
 ORDER BY m.group_id
 LIMIT $top OFFSET $skip;

-- 5r. group_id + resource_id startswith
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan + Filter (LIKE) → Sort — 0.095 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id LIKE $2 || '%'
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- SUMMARY — 62 queries benchmarked on 100K groups / 200K memberships
-- ############################################################################
--
-- Before indexes:  14 queries > 5ms, 7 queries > 40ms (max 121 ms)
-- After indexes:   61/62 queries < 1ms, 1 edge case at ~37ms (non-selective ILIKE)
--
-- All 15 indexes used by the planner:
--   resource_group_pkey                  — PK lookups and ordered scans with LIMIT
--   idx_rg_parent_id                    — parent_id eq/in (4h, 4j, 4aa)
--   idx_rg_name                         — name eq/in (4k, 4m)
--   idx_rg_external_id                  — external_id eq/in (4q, 4s, 4ac)
--   idx_rg_group_type                   — group_type eq/in (available, planner prefers PK+LIMIT)
--   idx_rg_name_trgm                    — name ILIKE endswith (4p)
--   idx_rg_extid_trgm                   — external_id ILIKE endswith (4v), contains with selective patterns
--   idx_rgc_descendant_id               — closure JOIN (all depth queries 4w-4z, 4ae)
--   idx_rgc_ancestor_depth              — closure ancestor+depth (4ad, 4af)
--   uq_resource_group_membership_unique — membership PK, group_id prefix, ordered scans
--   idx_rgm_resource_id                 — resource_id eq/in (5h, 5j)
--   idx_rgm_resource_type_id            — resource_type+resource_id (5p, 5q)
--   idx_rgm_resid_trgm                  — resource_id ILIKE endswith (5m)
--   resource_group_type_pkey            — type lookups (section 1-2)
--   idx_resource_group_type_code_lower  — case-insensitive code lookup
