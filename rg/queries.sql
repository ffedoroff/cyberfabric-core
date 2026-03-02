-- ============================================================================
-- Resource Group API — SQL queries for every GET endpoint
-- ============================================================================
--
-- EXPLAIN results from PostgreSQL 17.9 with seed data (5 groups, 9 closure, 6 memberships).
-- On small data PG planner always prefers Seq Scan over index access — this is expected.
-- Comments mark what WILL happen at scale (thousands+ rows) without proper indexes.
--
-- Existing indexes (from constraints only):
--   resource_group_type:       PK(code), UNIQUE(lower(code))
--   resource_group:            PK(id)
--   resource_group_closure:    — NONE —
--   resource_group_membership: UNIQUE(group_id, resource_type, resource_id)
--
-- Legend:
--   [PK]      — primary key lookup, fast at any scale
--   [UQ]      — unique index exact match
--   [UQ-P]    — unique index leftmost prefix
--   [SCAN]    — Seq Scan now AND at scale (no index exists)
--   [TINY]    — Seq Scan always (table is a small reference, <100 rows)
--   [SCAN-OK] — Seq Scan now (small data), index exists but planner skips it
--   [TRGM]    — needs pg_trgm GIN, otherwise Seq Scan at any scale
--


-- ############################################################################
-- 1. GET /types/{code}
-- ############################################################################

-- EXPLAIN: Seq Scan, Filter: (code = 'tenant')
-- On small data PK is skipped. At scale → Index Scan using resource_group_type_pkey [PK]
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1;


-- ############################################################################
-- 2. GET /types  (listTypes)
--    Filters: code (eq, ne, in, contains, startswith, endswith)
-- ############################################################################

-- 2a. No filter
-- EXPLAIN: Seq Scan → Sort → Limit [TINY]
SELECT code, parents
  FROM resource_group_type
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2b. code eq
-- EXPLAIN: Seq Scan, Filter: (code = 'tenant') [TINY, at scale → PK]
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1
 LIMIT $top OFFSET $skip;

-- 2c. code ne
-- EXPLAIN: Seq Scan, Filter: (code <> 'tenant') → Sort [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code <> $1
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2d. code in
-- EXPLAIN: Seq Scan, Filter: (code = ANY(...)) → Sort [TINY, at scale → PK]
SELECT code, parents
  FROM resource_group_type
 WHERE code = ANY($1::text[])
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2e. code startswith
-- EXPLAIN: Seq Scan, Filter: (code ~~ 'ten%') → Sort [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code LIKE $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2f. code contains
-- EXPLAIN: Seq Scan, Filter: (code ~~* '%en%') → Sort [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2g. code endswith
-- EXPLAIN: Seq Scan, Filter: (code ~~* '%ant') → Sort [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1
 ORDER BY code
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- 3. GET /groups/{group_id}
-- ############################################################################

-- EXPLAIN: Seq Scan, Filter: (id = '...')
-- On small data PK skipped. At scale → Index Scan using resource_group_pkey [PK]
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
-- EXPLAIN: Seq Scan → Sort(id) → Limit [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4b. group_id eq
-- EXPLAIN: Seq Scan, Filter: (id = '...') [PK at scale]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = $1
 LIMIT $top OFFSET $skip;

-- 4c. group_id ne
-- EXPLAIN: Seq Scan, Filter: (id <> '...') → Sort [SCAN — ne always seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4d. group_id in
-- EXPLAIN: Seq Scan, Filter: (id = ANY(...)) → Sort [PK at scale]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4e. group_type eq
-- EXPLAIN: Seq Scan, Filter: (group_type = 'tenant') → Sort [SCAN — needs idx_rg_group_type]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4f. group_type ne
-- EXPLAIN: Seq Scan, Filter: (group_type <> 'tenant') → Sort [SCAN — ne always seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4g. group_type in
-- EXPLAIN: Seq Scan, Filter: (group_type = ANY(...)) → Sort [SCAN — needs idx_rg_group_type]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4h. parent_id eq
-- EXPLAIN: Seq Scan, Filter: (parent_id = '...') → Sort [SCAN — needs idx_rg_parent_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4i. parent_id ne
-- EXPLAIN: Seq Scan, Filter: (parent_id <> '...') → Sort [SCAN — ne always seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4j. parent_id in
-- EXPLAIN: Seq Scan, Filter: (parent_id = ANY(...)) → Sort [SCAN — needs idx_rg_parent_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4k. name eq
-- EXPLAIN: Seq Scan, Filter: (name = 'D2') → Sort [SCAN — needs idx_rg_name]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4l. name ne
-- EXPLAIN: Seq Scan, Filter: (name <> 'D2') → Sort [SCAN — ne always seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4m. name in
-- EXPLAIN: Seq Scan, Filter: (name = ANY(...)) → Sort [SCAN — needs idx_rg_name]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4n. name startswith
-- EXPLAIN: Seq Scan, Filter: (name ~~ 'T%') → Sort [SCAN — needs idx_rg_name_pattern]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4o. name contains
-- EXPLAIN: Seq Scan, Filter: (name ~~* '%2%') → Sort [TRGM — needs gin_trgm_ops]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4p. name endswith
-- EXPLAIN: Seq Scan, Filter: (name ~~* '%1') → Sort [TRGM — needs gin_trgm_ops]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4q. external_id eq
-- EXPLAIN: Seq Scan, Filter: (external_id = 'D2') → Sort [SCAN — needs idx_rg_external_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4r. external_id ne
-- EXPLAIN: Seq Scan, Filter: (external_id <> 'D2') → Sort [SCAN — ne always seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4s. external_id in
-- EXPLAIN: Seq Scan, Filter: (external_id = ANY(...)) → Sort [SCAN — needs idx_rg_external_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4t. external_id startswith
-- EXPLAIN: Seq Scan, Filter: (external_id ~~ 'T%') → Sort [SCAN — needs idx_rg_external_id_pattern]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4u. external_id contains
-- EXPLAIN: Seq Scan, Filter: (external_id ~~* '%2%') → Sort [TRGM — needs gin_trgm_ops]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4v. external_id endswith
-- EXPLAIN: Seq Scan, Filter: (external_id ~~* '%1') → Sort [TRGM — needs gin_trgm_ops]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- ---- depth filters (require JOIN to closure) ------------------------------
-- closure has NO indexes — all JOINs are Seq Scan on both sides

-- 4w. depth eq
-- EXPLAIN: Hash Join (c.descendant_id = g.id)
--   → Seq Scan on closure, Filter: (depth = 1)
--   → Hash → Seq Scan on resource_group
-- [SCAN on closure — needs idx on (descendant_id) + (depth)]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4x. depth ne
-- EXPLAIN: Hash Join → Seq Scan on closure, Filter: (depth <> 0)
-- [SCAN on closure — ne always seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth gt
-- EXPLAIN: Nested Loop (join filter: g.id = c.descendant_id)
--   → Seq Scan on closure, Filter: (depth > 1)
--   → Seq Scan on resource_group
-- [SCAN on closure — needs idx]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth > $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth ge
-- EXPLAIN: Hash Join → Seq Scan on closure, Filter: (depth >= 1) [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth lt
-- EXPLAIN: Hash Join → Seq Scan on closure, Filter: (depth < 2) [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth < $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth le
-- EXPLAIN: Hash Join → Seq Scan on closure, Filter: (depth <= 1) [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4z. depth range: ge AND le
-- EXPLAIN: Hash Join → Seq Scan on closure, Filter: (depth >= 1 AND depth <= 2) [SCAN]
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
-- EXPLAIN: Seq Scan, Filter: (group_type = '...' AND parent_id = '...') → Sort
-- [SCAN — needs idx_rg_group_type or idx_rg_parent_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.parent_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ab. group_type + name startswith
-- EXPLAIN: Seq Scan, Filter: (name ~~ 'T%' AND group_type = 'tenant') → Sort [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.name LIKE $2 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ac. group_type + external_id eq
-- EXPLAIN: Seq Scan, Filter: (group_type = '...' AND external_id = '...') → Sort [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.external_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ad. ancestor_id + depth (children of X at specific depth)
-- EXPLAIN: Nested Loop (join filter: g.id = c.descendant_id)
--   → Seq Scan on closure, Filter: (ancestor_id = '...' AND depth = 1)
--   → Seq Scan on resource_group
-- [SCAN on closure — needs PK(ancestor_id, descendant_id) + idx(ancestor_id, depth)]
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
-- EXPLAIN: Nested Loop (join filter: g.id = c.descendant_id)
--   → Seq Scan on resource_group, Filter: (group_type = 'department')
--   → Seq Scan on closure, Filter: (depth = 1)
-- [SCAN on both — needs idx_rg_group_type + idx on closure]
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
-- EXPLAIN: Nested Loop (join filter: g.id = c.descendant_id)
--   → Seq Scan on resource_group, Filter: (group_type = 'department')
--   → Seq Scan on closure, Filter: (ancestor_id = '...' AND depth = 1)
-- [SCAN on both — needs idx_rg_group_type + idx on closure(ancestor_id, depth)]
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
-- EXPLAIN: Seq Scan → Sort(group_id, resource_type, resource_id) → Limit [SCAN]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5b. group_id eq
-- EXPLAIN: Seq Scan, Filter: (group_id = '...') → Sort
-- At scale → Index Scan using uq_resource_group_membership_unique (leftmost prefix)
-- [UQ-P at scale]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5c. group_id ne
-- EXPLAIN: Seq Scan, Filter: (group_id <> '...') → Sort [SCAN — ne always seq scan]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5d. group_id in
-- EXPLAIN: Seq Scan, Filter: (group_id = ANY(...)) → Sort [UQ-P at scale]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = ANY($1::uuid[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5e. resource_type eq
-- EXPLAIN: Seq Scan, Filter: (resource_type = 'resource') → Sort
-- [SCAN — needs idx_rgm_resource_type]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
 ORDER BY m.group_id, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5f. resource_type ne
-- EXPLAIN: Seq Scan, Filter: (resource_type <> 'resource') → Sort [SCAN — ne always seq scan]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5g. resource_type in
-- EXPLAIN: Seq Scan, Filter: (resource_type = ANY(...)) → Sort [SCAN — needs idx_rgm_resource_type]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5h. resource_id eq
-- EXPLAIN: Seq Scan, Filter: (resource_id = 'R4') → Sort [SCAN — needs idx_rgm_resource_id]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = $1
 ORDER BY m.group_id, m.resource_type
 LIMIT $top OFFSET $skip;

-- 5i. resource_id ne
-- EXPLAIN: Seq Scan, Filter: (resource_id <> 'R4') → Sort [SCAN — ne always seq scan]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5j. resource_id in
-- EXPLAIN: Seq Scan, Filter: (resource_id = ANY(...)) → Sort [SCAN — needs idx_rgm_resource_id]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5k. resource_id startswith
-- EXPLAIN: Seq Scan, Filter: (resource_id ~~ 'R%') → Sort [SCAN — needs idx_rgm_resource_id_pattern]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id LIKE $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5l. resource_id contains
-- EXPLAIN: Seq Scan, Filter: (resource_id ~~* '%4%') → Sort [TRGM — needs gin_trgm_ops]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5m. resource_id endswith
-- EXPLAIN: Seq Scan, Filter: (resource_id ~~* '%4') → Sort [TRGM — needs gin_trgm_ops]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters -----------------------------------------------------

-- 5n. group_id + resource_type
-- EXPLAIN: Seq Scan, Filter: (group_id = '...' AND resource_type = 'resource') → Sort
-- At scale → Index Scan using uq_resource_group_membership_unique (first 2 cols)
-- [UQ-P at scale]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
 ORDER BY m.resource_id
 LIMIT $top OFFSET $skip;

-- 5o. group_id + resource_id
-- EXPLAIN: Seq Scan, Filter: (group_id = '...' AND resource_id = 'R4') → Sort
-- At scale → Index Scan on uq (group_id prefix) + filter
-- [UQ-P partial at scale]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id = $2
 ORDER BY m.resource_type
 LIMIT $top OFFSET $skip;

-- 5p. group_id + resource_type + resource_id (exact match)
-- EXPLAIN: Seq Scan, Filter: (group_id = '...' AND resource_type = '...' AND resource_id = '...')
-- At scale → Index Scan using uq_resource_group_membership_unique (all 3 cols, exact)
-- [UQ at scale]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
   AND m.resource_id = $3;

-- 5q. resource_type + resource_id (no group_id)
-- EXPLAIN: Seq Scan, Filter: (resource_type = '...' AND resource_id = '...') → Sort
-- UNIQUE index starts with group_id so it cannot be used here
-- [SCAN — needs idx_rgm_resource_type_resource_id or separate indexes]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
   AND m.resource_id = $2
 ORDER BY m.group_id
 LIMIT $top OFFSET $skip;

-- 5r. group_id + resource_id startswith
-- EXPLAIN: Seq Scan, Filter: (resource_id ~~ 'R%' AND group_id = '...') → Sort
-- At scale → Index Scan on uq (group_id prefix) + filter on resource_id
-- [UQ-P partial at scale, LIKE filter applied after index scan]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id LIKE $2 || '%'
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- SUMMARY: All queries use Seq Scan on seed data (expected for <100 rows).
--
-- At scale, queries that WILL remain Seq Scan without new indexes:
--
-- resource_group_closure (0 indexes — CRITICAL):
--   All depth queries (4w-4z, 4ad-4af) — Seq Scan on closure for JOIN + filter
--   Needs: PK(ancestor_id, descendant_id), idx(descendant_id), idx(ancestor_id, depth)
--
-- resource_group (only PK exists):
--   group_type eq/in (4e, 4g)          — needs btree(group_type)
--   parent_id eq/in (4h, 4j)           — needs btree(parent_id)
--   name eq/in (4k, 4m)                — needs btree(name)
--   name startswith (4n)               — needs btree(name text_pattern_ops)
--   name contains/endswith (4o, 4p)    — needs GIN(name gin_trgm_ops)
--   external_id eq/in (4q, 4s)         — needs btree(external_id)
--   external_id startswith (4t)        — needs btree(external_id text_pattern_ops)
--   external_id contains/endswith      — needs GIN(external_id gin_trgm_ops)
--
-- resource_group_membership (only UNIQUE(group_id, resource_type, resource_id)):
--   resource_type alone (5e, 5g)       — needs btree(resource_type)
--   resource_id alone (5h, 5j)         — needs btree(resource_id)
--   resource_id startswith (5k)        — needs btree(resource_id text_pattern_ops)
--   resource_id contains/endswith      — needs GIN(resource_id gin_trgm_ops)
--   resource_type + resource_id (5q)   — needs composite or separate indexes
--
-- Queries that ALWAYS Seq Scan regardless of indexes:
--   ne (not-equal) — low selectivity, planner prefers seq scan
--   No-filter listings (4a, 5a) — no WHERE clause
-- ############################################################################
