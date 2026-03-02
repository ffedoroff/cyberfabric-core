-- ============================================================================
-- Resource Group API — SQL queries for every GET endpoint
-- ============================================================================
--
-- Existing indexes (from constraints only):
--   resource_group_type:       PK(code)
--   resource_group:            PK(id)
--   resource_group_closure:    — NONE —
--   resource_group_membership: UNIQUE(group_id, resource_type, resource_id)
--
-- Legend:
--   [PK]    — uses primary key
--   [UQ]    — uses unique constraint index
--   [UQ-P]  — uses unique constraint (leftmost prefix)
--   [SCAN]  — seq scan, needs index
--   [TINY]  — seq scan acceptable (справочник, <100 rows)
--   [TRGM]  — needs pg_trgm GIN for index usage, otherwise seq scan
--


-- ############################################################################
-- 1. GET /types/{code}
-- ############################################################################

-- [PK]
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1;


-- ############################################################################
-- 2. GET /types  (listTypes)
--    Filters: code (eq, ne, in, contains, startswith, endswith)
-- ############################################################################

-- 2a. No filter [TINY]
SELECT code, parents
  FROM resource_group_type
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2b. code eq [PK]
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1
 LIMIT $top OFFSET $skip;

-- 2c. code ne [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code <> $1
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2d. code in [PK]
SELECT code, parents
  FROM resource_group_type
 WHERE code = ANY($1::text[])
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2e. code startswith [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code LIKE $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2f. code contains [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2g. code endswith [TINY]
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1
 ORDER BY code
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- 3. GET /groups/{group_id}
-- ############################################################################

-- [PK]
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

-- 4a. No filter [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4b. group_id eq [PK]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = $1
 LIMIT $top OFFSET $skip;

-- 4c. group_id ne [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4d. group_id in [PK]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4e. group_type eq [SCAN — needs idx_rg_group_type]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4f. group_type ne [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4g. group_type in [SCAN — needs idx_rg_group_type]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4h. parent_id eq [SCAN — needs idx_rg_parent_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4i. parent_id ne [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4j. parent_id in [SCAN — needs idx_rg_parent_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4k. name eq [SCAN — needs idx_rg_name]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4l. name ne [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4m. name in [SCAN — needs idx_rg_name]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4n. name startswith [SCAN — needs idx_rg_name_pattern (text_pattern_ops)]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4o. name contains [TRGM — needs gin_trgm_ops or seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4p. name endswith [TRGM — needs gin_trgm_ops or seq scan]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4q. external_id eq [SCAN — needs idx_rg_external_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4r. external_id ne [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4s. external_id in [SCAN — needs idx_rg_external_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4t. external_id startswith [SCAN — needs idx_rg_external_id_pattern]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4u. external_id contains [TRGM]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4v. external_id endswith [TRGM]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- ---- depth filters (require JOIN to closure) ------------------------------
-- closure has NO indexes — all these are SCAN on closure

-- 4w. depth eq [SCAN on closure — needs PK(ancestor_id, descendant_id) + idx on depth]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4x. depth ne [SCAN on closure]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4y. depth gt / ge / lt / le [SCAN on closure]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth > $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth < $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <= $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4z. depth range: ge $1 AND le $2 [SCAN on closure]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= $1 AND c.depth <= $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters ----------------------------------------------------

-- 4aa. group_type + parent_id [SCAN — needs idx_rg_group_type or idx_rg_parent_id]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.parent_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ab. group_type + name startswith [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.name LIKE $2 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ac. group_type + external_id eq [SCAN]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.external_id = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ad. parent_id + depth (children of X at specific depth) [SCAN on closure]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.ancestor_id = $1
   AND c.depth = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ae. group_type + depth [SCAN on both]
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE g.group_type = $1
   AND c.depth = $2
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4af. parent_id + group_type + depth [SCAN on closure]
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

-- 5a. No filter [SCAN]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5b. group_id eq [UQ-P — uses leftmost column of UNIQUE index]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5c. group_id ne [SCAN]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5d. group_id in [UQ-P]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = ANY($1::uuid[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5e. resource_type eq [SCAN — needs idx_rgm_resource_type]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
 ORDER BY m.group_id, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5f. resource_type ne [SCAN]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5g. resource_type in [SCAN — needs idx_rgm_resource_type]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5h. resource_id eq [SCAN — needs idx_rgm_resource_id]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = $1
 ORDER BY m.group_id, m.resource_type
 LIMIT $top OFFSET $skip;

-- 5i. resource_id ne [SCAN]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5j. resource_id in [SCAN — needs idx_rgm_resource_id]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5k. resource_id startswith [SCAN — needs idx_rgm_resource_id_pattern]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id LIKE $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5l. resource_id contains [TRGM]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5m. resource_id endswith [TRGM]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters -----------------------------------------------------

-- 5n. group_id + resource_type [UQ-P — uses first two columns of UNIQUE]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
 ORDER BY m.resource_id
 LIMIT $top OFFSET $skip;

-- 5o. group_id + resource_id [UQ-P partial — group_id prefix, then filter]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id = $2
 ORDER BY m.resource_type
 LIMIT $top OFFSET $skip;

-- 5p. group_id + resource_type + resource_id [UQ — exact match]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
   AND m.resource_id = $3;

-- 5q. resource_type + resource_id (no group_id) [SCAN — needs composite index]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
   AND m.resource_id = $2
 ORDER BY m.group_id
 LIMIT $top OFFSET $skip;

-- 5r. group_id + resource_id startswith [UQ-P partial]
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id LIKE $2 || '%'
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- SCAN SUMMARY — indexes needed for full coverage
-- ############################################################################
--
-- resource_group_closure (critical — zero indexes):
--   PK (ancestor_id, descendant_id)        — all closure JOINs
--   (descendant_id, ancestor_id)           — reverse lookups
--   (ancestor_id, depth)                   — depth filters from ancestor
--   (descendant_id, depth)                 — depth filters from descendant
--
-- resource_group:
--   (group_type)                           — 4e, 4g, 4aa, 4ab, 4ac, 4ae, 4af
--   (parent_id)                            — 4h, 4j, 4aa
--   (external_id)                          — 4q, 4s, 4ac
--   (name text_pattern_ops)                — 4n (startswith)
--   (external_id text_pattern_ops)         — 4t (startswith)
--   GIN (name gin_trgm_ops)               — 4o, 4p (contains/endswith)
--   GIN (external_id gin_trgm_ops)         — 4u, 4v (contains/endswith)
--
-- resource_group_membership:
--   (resource_type)                        — 5e, 5g
--   (resource_id)                          — 5h, 5j, 5q
--   (resource_id text_pattern_ops)         — 5k (startswith)
--   GIN (resource_id gin_trgm_ops)         — 5l, 5m (contains/endswith)
--
-- Queries that will ALWAYS seq scan regardless of indexes:
--   ne (not-equal) — low selectivity, planner prefers seq scan
--   No-filter full listings — no WHERE clause
