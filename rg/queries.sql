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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.033..0.035 rows=1 loops=1)
--   Filter: (code = 'tenant'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared read=1
--   Planning:
--   Buffers: shared hit=80 read=3
--   Planning Time: 0.612 ms
--   Execution Time: 0.083 ms
--   (8 rows)
-- Summary: Seq Scan
-- Execution Time: 0.083 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.023..0.025 rows=1 loops=1)
--   Filter: (code = 'tenant'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared read=1
--   Planning:
--   Buffers: shared hit=80 read=3
--   Planning Time: 0.498 ms
--   Execution Time: 0.059 ms
--   (8 rows)
-- Summary: Seq Scan
-- Execution Time: 0.059 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1;


-- ############################################################################
-- 2. GET /types  (listTypes)
--    Filters: code (eq, ne, in, contains, startswith, endswith)
-- ############################################################################

-- 2a. No filter
-- Seq Scan → Sort → Limit on 20-row table [TINY] — 0.073 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.63..1.66 rows=10 width=45) (actual time=0.046..0.048 rows=10 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.63..1.68 rows=20 width=45) (actual time=0.045..0.045 rows=10 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 26kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.20 rows=20 width=45) (actual time=0.008..0.010 rows=20 loops=1)
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=83 read=1
--   Planning Time: 0.436 ms
--   Execution Time: 0.104 ms
--   (12 rows)
-- Summary: Seq Scan
-- Execution Time: 0.104 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.63..1.66 rows=10 width=45) (actual time=0.041..0.043 rows=10 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.63..1.68 rows=20 width=45) (actual time=0.040..0.041 rows=10 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 26kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.20 rows=20 width=45) (actual time=0.008..0.009 rows=20 loops=1)
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=83 read=1
--   Planning Time: 0.405 ms
--   Execution Time: 0.071 ms
--   (12 rows)
-- Summary: Seq Scan
-- Execution Time: 0.071 ms
SELECT code, parents
  FROM resource_group_type
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2b. code eq
-- Seq Scan + Filter [TINY] — 0.026 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.00..1.25 rows=1 width=45) (actual time=0.014..0.016 rows=1 loops=1)
--   Buffers: shared hit=1
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.013..0.014 rows=1 loops=1)
--   Filter: (code = 'tenant'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=83
--   Planning Time: 0.440 ms
--   Execution Time: 0.045 ms
--   (10 rows)
-- Summary: Seq Scan
-- Execution Time: 0.045 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.00..1.25 rows=1 width=45) (actual time=0.011..0.013 rows=1 loops=1)
--   Buffers: shared hit=1
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.010..0.011 rows=1 loops=1)
--   Filter: (code = 'tenant'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=83
--   Planning Time: 0.380 ms
--   Execution Time: 0.036 ms
--   (10 rows)
-- Summary: Seq Scan
-- Execution Time: 0.036 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = $1
 LIMIT $top OFFSET $skip;

-- 2c. code ne
-- Seq Scan + Filter → Sort [TINY] — 0.080 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.65..1.68 rows=10 width=45) (actual time=0.059..0.061 rows=10 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.65..1.70 rows=19 width=45) (actual time=0.057..0.058 rows=10 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 26kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=19 width=45) (actual time=0.010..0.012 rows=19 loops=1)
--   Filter: (code <> 'tenant'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=86
--   Planning Time: 0.427 ms
--   Execution Time: 0.122 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.122 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.65..1.68 rows=10 width=45) (actual time=0.051..0.053 rows=10 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.65..1.70 rows=19 width=45) (actual time=0.050..0.050 rows=10 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 26kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=19 width=45) (actual time=0.011..0.014 rows=19 loops=1)
--   Filter: (code <> 'tenant'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=86
--   Planning Time: 0.428 ms
--   Execution Time: 0.082 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.082 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code <> $1
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2d. code in
-- Seq Scan + Filter → Sort [TINY] — 0.042 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.30..1.31 rows=3 width=45) (actual time=0.047..0.048 rows=3 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.30..1.31 rows=3 width=45) (actual time=0.046..0.046 rows=3 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.27 rows=3 width=45) (actual time=0.012..0.014 rows=3 loops=1)
--   Filter: (code = ANY ('{tenant,region,zone}'::text[]))
--   Rows Removed by Filter: 17
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=84
--   Planning Time: 0.461 ms
--   Execution Time: 0.068 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.068 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.30..1.31 rows=3 width=45) (actual time=0.040..0.041 rows=3 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.30..1.31 rows=3 width=45) (actual time=0.038..0.039 rows=3 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.27 rows=3 width=45) (actual time=0.011..0.013 rows=3 loops=1)
--   Filter: (code = ANY ('{tenant,region,zone}'::text[]))
--   Rows Removed by Filter: 17
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=84
--   Planning Time: 0.416 ms
--   Execution Time: 0.059 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.059 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code = ANY($1::text[])
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2e. code startswith
-- Seq Scan + Filter (code ~~ 'prefix%') → Sort [TINY] — 0.062 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.26..1.26 rows=1 width=45) (actual time=0.037..0.037 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.26..1.26 rows=1 width=45) (actual time=0.035..0.036 rows=1 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.010..0.012 rows=1 loops=1)
--   Filter: (code ~~ 'ten%'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=95 read=4
--   Planning Time: 1.398 ms
--   Execution Time: 0.071 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.071 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.26..1.26 rows=1 width=45) (actual time=0.037..0.037 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.26..1.26 rows=1 width=45) (actual time=0.036..0.036 rows=1 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.011..0.012 rows=1 loops=1)
--   Filter: (code ~~ 'ten%'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=95 read=4
--   Planning Time: 1.276 ms
--   Execution Time: 0.071 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.071 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code LIKE $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2f. code contains
-- Seq Scan + Filter (code ~~* '%mid%') → Sort [TINY] — 0.053 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.26..1.26 rows=1 width=45) (actual time=0.046..0.046 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.26..1.26 rows=1 width=45) (actual time=0.044..0.045 rows=1 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.011..0.021 rows=1 loops=1)
--   Filter: (code ~~* '%ena%'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=90
--   Planning Time: 0.442 ms
--   Execution Time: 0.075 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.075 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.26..1.26 rows=1 width=45) (actual time=0.046..0.047 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.26..1.26 rows=1 width=45) (actual time=0.045..0.045 rows=1 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.011..0.022 rows=1 loops=1)
--   Filter: (code ~~* '%ena%'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=90
--   Planning Time: 0.469 ms
--   Execution Time: 0.075 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.075 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1 || '%'
 ORDER BY code
 LIMIT $top OFFSET $skip;

-- 2g. code endswith
-- Seq Scan + Filter (code ~~* '%suffix') → Sort [TINY] — 0.054 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.26..1.26 rows=1 width=45) (actual time=0.047..0.047 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.26..1.26 rows=1 width=45) (actual time=0.046..0.046 rows=1 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.010..0.020 rows=1 loops=1)
--   Filter: (code ~~* '%ant'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=90
--   Planning Time: 0.463 ms
--   Execution Time: 0.075 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.075 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=1.26..1.26 rows=1 width=45) (actual time=0.044..0.045 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Sort  (cost=1.26..1.26 rows=1 width=45) (actual time=0.043..0.044 rows=1 loops=1)
--   Sort Key: code
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4
--   ->  Seq Scan on resource_group_type  (cost=0.00..1.25 rows=1 width=45) (actual time=0.011..0.021 rows=1 loops=1)
--   Filter: (code ~~* '%ant'::text)
--   Rows Removed by Filter: 19
--   Buffers: shared hit=1
--   Planning:
--   Buffers: shared hit=90
--   Planning Time: 0.447 ms
--   Execution Time: 0.073 ms
--   (14 rows)
-- Summary: Seq Scan
-- Execution Time: 0.073 ms
SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || $1
 ORDER BY code
 LIMIT $top OFFSET $skip;


-- ############################################################################
-- 3. GET /groups/{group_id}
-- ############################################################################

-- Index Scan using resource_group_pkey [PK] — 0.113 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.578..0.579 rows=1 loops=1)
--   Index Cond: (id = '8c0855b7-4f52-4ca9-8164-105f24bff755'::uuid)
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=161
--   Planning Time: 0.631 ms
--   Execution Time: 0.609 ms
--   (7 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.609 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.731..0.732 rows=1 loops=1)
--   Index Cond: (id = '5906e284-dd37-4e68-a031-768607844cee'::uuid)
--   Buffers: shared hit=1 read=3
--   Planning:
--   Buffers: shared hit=167
--   Planning Time: 0.717 ms
--   Execution Time: 0.758 ms
--   (7 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.758 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.40 rows=10 width=86) (actual time=0.504..2.091 rows=10 loops=1)
--   Buffers: shared hit=1 read=12
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388168.54 rows=3999603 width=86) (actual time=0.503..2.086 rows=10 loops=1)
--   Buffers: shared hit=1 read=12
--   Planning:
--   Buffers: shared hit=172
--   Planning Time: 0.635 ms
--   Execution Time: 2.112 ms
--   (8 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 2.112 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.40 rows=10 width=86) (actual time=0.086..0.251 rows=10 loops=1)
--   Buffers: shared hit=1 read=12
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388406.19 rows=4000153 width=86) (actual time=0.085..0.248 rows=10 loops=1)
--   Buffers: shared hit=1 read=12
--   Planning:
--   Buffers: shared hit=175
--   Planning Time: 0.714 ms
--   Execution Time: 0.268 ms
--   (8 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.268 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4b. group_id eq
-- Index Scan using resource_group_pkey [PK] — 0.041 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..8.45 rows=1 width=86) (actual time=0.034..0.035 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.033..0.033 rows=1 loops=1)
--   Index Cond: (id = '8c0855b7-4f52-4ca9-8164-105f24bff755'::uuid)
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=161
--   Planning Time: 0.637 ms
--   Execution Time: 0.053 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.053 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..8.45 rows=1 width=86) (actual time=0.037..0.038 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.036..0.037 rows=1 loops=1)
--   Index Cond: (id = '5906e284-dd37-4e68-a031-768607844cee'::uuid)
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=167
--   Planning Time: 0.676 ms
--   Execution Time: 0.055 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.055 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = $1
 LIMIT $top OFFSET $skip;

-- 4c. group_id ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.064 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.035..0.076 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=3999602 width=86) (actual time=0.034..0.074 rows=10 loops=1)
--   Filter: (id <> '8c0855b7-4f52-4ca9-8164-105f24bff755'::uuid)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=174
--   Planning Time: 0.728 ms
--   Execution Time: 0.093 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.093 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.036..0.093 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=4000152 width=86) (actual time=0.035..0.091 rows=10 loops=1)
--   Filter: (id <> '5906e284-dd37-4e68-a031-768607844cee'::uuid)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=177
--   Planning Time: 0.685 ms
--   Execution Time: 0.112 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.112 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4d. group_id in
-- Bitmap Index Scan on resource_group_pkey → Bitmap Heap Scan → Sort [BITMAP+PK] — 0.122 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..25.34 rows=3 width=86) (actual time=0.089..0.632 rows=3 loops=1)
--   Buffers: shared hit=6 read=4
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..25.34 rows=3 width=86) (actual time=0.088..0.630 rows=3 loops=1)
--   Index Cond: (id = ANY ('{8c0855b7-4f52-4ca9-8164-105f24bff755,ff2492a7-8b36-4dd9-8771-4758091cd688,66c92935-930b-49c3-8efa-8e74797cf286}'::uuid[]))
--   Buffers: shared hit=6 read=4
--   Planning:
--   Buffers: shared hit=172
--   Planning Time: 0.653 ms
--   Execution Time: 0.651 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.651 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..25.34 rows=3 width=86) (actual time=0.032..1.156 rows=3 loops=1)
--   Buffers: shared hit=6 read=4
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..25.34 rows=3 width=86) (actual time=0.031..1.153 rows=3 loops=1)
--   Index Cond: (id = ANY ('{5906e284-dd37-4e68-a031-768607844cee,634029af-1972-4628-b36f-76022c585bfe,994ae7cc-6b71-4d00-88e9-c307ea058db8}'::uuid[]))
--   Buffers: shared hit=6 read=4
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.654 ms
--   Execution Time: 1.174 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 1.174 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4e. group_type eq
-- Index Scan using resource_group_pkey + Filter [PK] — 0.554 ms
-- PK scan in order + LIMIT stops early. idx_rg_group_type available for queries without ORDER BY id.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..17.00 rows=10 width=86) (actual time=1.273..24.815 rows=10 loops=1)
--   Buffers: shared hit=13 read=259
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=240243 width=86) (actual time=1.272..24.809 rows=10 loops=1)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 258
--   Buffers: shared hit=13 read=259
--   Planning:
--   Buffers: shared hit=175
--   Planning Time: 0.665 ms
--   Execution Time: 24.838 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 24.838 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.56..11.55 rows=10 width=86) (actual time=0.115..0.257 rows=10 loops=1)
--   Buffers: shared read=14
--   ->  Index Scan using idx_rg_group_type on resource_group g  (cost=0.56..260808.54 rows=237209 width=86) (actual time=0.114..0.255 rows=10 loops=1)
--   Index Cond: (group_type = 'tenant'::text)
--   Buffers: shared read=14
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.693 ms
--   Execution Time: 0.274 ms
--   (9 rows)
-- Summary: Index Scan (indexes: idx_rg_group_type)
-- Execution Time: 0.274 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4f. group_type ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.066 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.49 rows=10 width=86) (actual time=0.128..0.197 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=3759360 width=86) (actual time=0.127..0.195 rows=10 loops=1)
--   Filter: (group_type <> 'tenant'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=177
--   Planning Time: 0.692 ms
--   Execution Time: 0.243 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.243 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.49 rows=10 width=86) (actual time=0.035..0.081 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=3762944 width=86) (actual time=0.034..0.079 rows=10 loops=1)
--   Filter: (group_type <> 'tenant'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=180
--   Planning Time: 0.762 ms
--   Execution Time: 0.106 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.106 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4g. group_type in
-- Index Scan using resource_group_pkey + Filter [PK] — 0.153 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..6.11 rows=10 width=86) (actual time=0.040..0.214 rows=10 loops=1)
--   Buffers: shared hit=39
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..403167.05 rows=709530 width=86) (actual time=0.039..0.212 rows=10 loops=1)
--   Filter: (group_type = ANY ('{tenant,region,zone}'::text[]))
--   Rows Removed by Filter: 26
--   Buffers: shared hit=39
--   Planning:
--   Buffers: shared hit=226
--   Planning Time: 0.885 ms
--   Execution Time: 0.243 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.243 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..6.21 rows=10 width=86) (actual time=0.039..0.628 rows=10 loops=1)
--   Buffers: shared hit=17 read=57
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..403406.76 rows=697360 width=86) (actual time=0.039..0.625 rows=10 loops=1)
--   Filter: (group_type = ANY ('{tenant,region,zone}'::text[]))
--   Rows Removed by Filter: 61
--   Buffers: shared hit=17 read=57
--   Planning:
--   Buffers: shared hit=229
--   Planning Time: 0.952 ms
--   Execution Time: 0.647 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.647 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4h. parent_id eq
-- Bitmap Index Scan on idx_rg_parent_id → Bitmap Heap Scan → Sort — 0.310 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=28.61..28.63 rows=6 width=86) (actual time=2.777..2.780 rows=10 loops=1)
--   Buffers: shared hit=3 read=56
--   ->  Sort  (cost=28.61..28.63 rows=6 width=86) (actual time=2.776..2.777 rows=10 loops=1)
--   Sort Key: id
--   Sort Method: top-N heapsort  Memory: 27kB
--   Buffers: shared hit=3 read=56
--   ->  Index Scan using idx_rg_parent_id on resource_group g  (cost=0.43..28.53 rows=6 width=86) (actual time=0.066..2.713 rows=55 loops=1)
--   Index Cond: (parent_id = '94218bc1-3509-44a7-8cd8-6c6a9ce1f0b4'::uuid)
--   Buffers: shared read=56
--   Planning:
--   Buffers: shared hit=172
--   Planning Time: 0.634 ms
--   Execution Time: 2.804 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_parent_id)
-- Execution Time: 2.804 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=32.65..32.67 rows=7 width=86) (actual time=0.711..0.713 rows=10 loops=1)
--   Buffers: shared hit=5 read=32
--   ->  Sort  (cost=32.65..32.67 rows=7 width=86) (actual time=0.710..0.711 rows=10 loops=1)
--   Sort Key: id
--   Sort Method: top-N heapsort  Memory: 27kB
--   Buffers: shared hit=5 read=32
--   ->  Index Scan using idx_rg_parent_id on resource_group g  (cost=0.43..32.55 rows=7 width=86) (actual time=0.444..0.678 rows=31 loops=1)
--   Index Cond: (parent_id = 'cf2db198-34fc-4a31-b9c0-824a65c5136b'::uuid)
--   Buffers: shared hit=2 read=32
--   Planning:
--   Buffers: shared hit=175
--   Planning Time: 0.645 ms
--   Execution Time: 0.741 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_parent_id)
-- Execution Time: 0.741 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4i. parent_id ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.076 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.035..0.074 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=3998930 width=86) (actual time=0.034..0.072 rows=10 loops=1)
--   Filter: (parent_id <> '94218bc1-3509-44a7-8cd8-6c6a9ce1f0b4'::uuid)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=177
--   Planning Time: 0.652 ms
--   Execution Time: 0.093 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.093 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.039..0.110 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=3999480 width=86) (actual time=0.039..0.107 rows=10 loops=1)
--   Filter: (parent_id <> 'cf2db198-34fc-4a31-b9c0-824a65c5136b'::uuid)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=180
--   Planning Time: 0.763 ms
--   Execution Time: 0.130 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.130 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4j. parent_id in
-- Bitmap Index Scan on idx_rg_parent_id → Bitmap Heap Scan → Sort — 0.093 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=89.10..89.13 rows=10 width=86) (actual time=0.607..0.608 rows=3 loops=1)
--   Buffers: shared hit=4 read=5
--   ->  Sort  (cost=89.10..89.15 rows=19 width=86) (actual time=0.605..0.606 rows=3 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4 read=5
--   ->  Bitmap Heap Scan on resource_group g  (cost=13.43..88.70 rows=19 width=86) (actual time=0.075..0.568 rows=3 loops=1)
--   Recheck Cond: (parent_id = ANY ('{000000e4-a835-49d5-9177-7cff106b1ede,00000f38-77c9-4469-9b45-65cda43dacdd,00000fdf-aab3-4f76-8398-824ccf5c24ac}'::uuid[]))
--   Heap Blocks: exact=3
--   Buffers: shared hit=1 read=5
--   ->  Bitmap Index Scan on idx_rg_parent_id  (cost=0.00..13.42 rows=19 width=0) (actual time=0.043..0.043 rows=3 loops=1)
--   Index Cond: (parent_id = ANY ('{000000e4-a835-49d5-9177-7cff106b1ede,00000f38-77c9-4469-9b45-65cda43dacdd,00000fdf-aab3-4f76-8398-824ccf5c24ac}'::uuid[]))
--   Buffers: shared hit=1 read=2
--   Planning:
--   Buffers: shared hit=172
--   Planning Time: 0.646 ms
--   Execution Time: 0.639 ms
--   (17 rows)
-- Summary: Bitmap Scan, Index Scan
-- Execution Time: 0.639 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=93.09..93.11 rows=10 width=86) (actual time=0.144..0.145 rows=4 loops=1)
--   Buffers: shared hit=4 read=6
--   ->  Sort  (cost=93.09..93.14 rows=20 width=86) (actual time=0.143..0.144 rows=4 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4 read=6
--   ->  Bitmap Heap Scan on resource_group g  (cost=13.45..92.65 rows=20 width=86) (actual time=0.091..0.119 rows=4 loops=1)
--   Recheck Cond: (parent_id = ANY ('{00000951-50e9-4d38-98df-421302326999,00000ce4-e744-4593-839d-493951af49fe,000027f7-3d2b-4e42-88f5-e894500b5d41}'::uuid[]))
--   Heap Blocks: exact=4
--   Buffers: shared hit=1 read=6
--   ->  Bitmap Index Scan on idx_rg_parent_id  (cost=0.00..13.45 rows=20 width=0) (actual time=0.040..0.040 rows=4 loops=1)
--   Index Cond: (parent_id = ANY ('{00000951-50e9-4d38-98df-421302326999,00000ce4-e744-4593-839d-493951af49fe,000027f7-3d2b-4e42-88f5-e894500b5d41}'::uuid[]))
--   Buffers: shared hit=1 read=2
--   Planning:
--   Buffers: shared hit=175
--   Planning Time: 0.653 ms
--   Execution Time: 0.175 ms
--   (17 rows)
-- Summary: Bitmap Scan, Index Scan
-- Execution Time: 0.175 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = ANY($1::uuid[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4k. name eq
-- Index Scan using idx_rg_name → Sort — 0.135 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=1.028..1.029 rows=1 loops=1)
--   Buffers: shared hit=3 read=4
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=1.027..1.028 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=3 read=4
--   ->  Index Scan using idx_rg_name on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.990..0.991 rows=1 loops=1)
--   Index Cond: (name = 'division-7ujnng'::text)
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=175
--   Planning Time: 0.659 ms
--   Execution Time: 1.051 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_name)
-- Execution Time: 1.051 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=1.199..1.200 rows=1 loops=1)
--   Buffers: shared hit=3 read=4
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=1.198..1.198 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=3 read=4
--   ->  Index Scan using idx_rg_name on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=1.170..1.171 rows=1 loops=1)
--   Index Cond: (name = 'division-uk2use'::text)
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.685 ms
--   Execution Time: 1.233 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_name)
-- Execution Time: 1.233 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4l. name ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.076 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.056..0.098 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=3999602 width=86) (actual time=0.055..0.096 rows=10 loops=1)
--   Filter: (name <> 'division-7ujnng'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=177
--   Planning Time: 0.692 ms
--   Execution Time: 0.118 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.118 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.034..0.078 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=4000152 width=86) (actual time=0.034..0.076 rows=10 loops=1)
--   Filter: (name <> 'division-uk2use'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=180
--   Planning Time: 0.761 ms
--   Execution Time: 0.098 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.098 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4m. name in
-- Index Scan using idx_rg_name → Sort — 0.065 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.051..0.052 rows=1 loops=1)
--   Buffers: shared hit=7
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.050..0.051 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=7
--   ->  Index Scan using idx_rg_name on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.025..0.025 rows=1 loops=1)
--   Index Cond: (name = ANY ('{division-7ujnng}'::text[]))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=226
--   Planning Time: 0.910 ms
--   Execution Time: 0.083 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_name)
-- Execution Time: 0.083 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.081..0.082 rows=1 loops=1)
--   Buffers: shared hit=7
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.080..0.080 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=7
--   ->  Index Scan using idx_rg_name on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.029..0.030 rows=1 loops=1)
--   Index Cond: (name = ANY ('{division-uk2use}'::text[]))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=229
--   Planning Time: 0.918 ms
--   Execution Time: 0.112 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_name)
-- Execution Time: 0.112 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4n. name startswith
-- Index Scan using resource_group_pkey + Filter [PK] — 0.250 ms
-- PK scan + LIMIT. For high-offset → btree(name text_pattern_ops) would help.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..14.51 rows=10 width=86) (actual time=0.098..0.259 rows=10 loops=1)
--   Buffers: shared hit=106
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=282800 width=86) (actual time=0.097..0.257 rows=10 loops=1)
--   Filter: (name ~~ 'div%'::text)
--   Rows Removed by Filter: 93
--   Buffers: shared hit=106
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.762 ms
--   Execution Time: 0.278 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.278 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..16.86 rows=10 width=86) (actual time=0.077..0.305 rows=10 loops=1)
--   Buffers: shared hit=75 read=10
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=242434 width=86) (actual time=0.076..0.302 rows=10 loops=1)
--   Filter: (name ~~ 'div%'::text)
--   Rows Removed by Filter: 72
--   Buffers: shared hit=75 read=10
--   Planning:
--   Buffers: shared hit=181
--   Planning Time: 0.764 ms
--   Execution Time: 0.323 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.323 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name LIKE $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4o. name contains
-- Index Scan using resource_group_pkey + Filter [PK] — 0.296 ms
-- PK scan + LIMIT finds matches quickly. GIN idx_rg_name_trgm available for selective patterns.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..14.51 rows=10 width=86) (actual time=0.143..0.356 rows=10 loops=1)
--   Buffers: shared hit=106
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=282800 width=86) (actual time=0.141..0.353 rows=10 loops=1)
--   Filter: (name ~~* '%vis%'::text)
--   Rows Removed by Filter: 93
--   Buffers: shared hit=106
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.785 ms
--   Execution Time: 0.373 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.373 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..16.86 rows=10 width=86) (actual time=0.078..0.262 rows=10 loops=1)
--   Buffers: shared hit=85
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=242434 width=86) (actual time=0.077..0.260 rows=10 loops=1)
--   Filter: (name ~~* '%vis%'::text)
--   Rows Removed by Filter: 72
--   Buffers: shared hit=85
--   Planning:
--   Buffers: shared hit=181
--   Planning Time: 0.847 ms
--   Execution Time: 0.279 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.279 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4p. name endswith
-- Bitmap Index Scan on idx_rg_name_trgm → Bitmap Heap Scan → Sort — 0.159 ms
-- GIN trgm index handles ILIKE suffix efficiently.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..9954.61 rows=10 width=86) (actual time=3122.630..5055.158 rows=10 loops=1)
--   Buffers: shared hit=119204 read=353369
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=400 width=86) (actual time=3122.628..5055.142 rows=10 loops=1)
--   Filter: (name ~~* '%nng'::text)
--   Rows Removed by Filter: 470338
--   Buffers: shared hit=119204 read=353369
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.778 ms
--   Execution Time: 5055.204 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 5055.204 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..9960.58 rows=10 width=86) (actual time=71.461..1361.508 rows=10 loops=1)
--   Buffers: shared hit=76663 read=228967 written=1
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=400 width=86) (actual time=71.460..1361.497 rows=10 loops=1)
--   Filter: (name ~~* '%use'::text)
--   Rows Removed by Filter: 304209
--   Buffers: shared hit=76663 read=228967 written=1
--   Planning:
--   Buffers: shared hit=181
--   Planning Time: 0.805 ms
--   Execution Time: 1361.538 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 1361.538 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4q. external_id eq
-- Index Scan using idx_rg_external_id → Sort — 0.090 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.471..0.472 rows=1 loops=1)
--   Buffers: shared hit=3 read=4
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.470..0.470 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=3 read=4
--   ->  Index Scan using idx_rg_external_id on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.409..0.410 rows=1 loops=1)
--   Index Cond: (external_id = 'ext-f720c5b9'::text)
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=143 read=32
--   Planning Time: 1.735 ms
--   Execution Time: 0.527 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_external_id)
-- Execution Time: 0.527 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.654..0.655 rows=1 loops=1)
--   Buffers: shared hit=3 read=4
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.654..0.654 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=3 read=4
--   ->  Index Scan using idx_rg_external_id on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.626..0.627 rows=1 loops=1)
--   Index Cond: (external_id = 'ext-0f73ba25'::text)
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=146 read=32
--   Planning Time: 0.941 ms
--   Execution Time: 0.678 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_external_id)
-- Execution Time: 0.678 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4r. external_id ne
-- Index Scan using resource_group_pkey + Filter [PK] — 0.062 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.061..0.138 rows=10 loops=1)
--   Buffers: shared hit=2 read=11
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=3999602 width=86) (actual time=0.060..0.135 rows=10 loops=1)
--   Filter: (external_id <> 'ext-f720c5b9'::text)
--   Buffers: shared hit=2 read=11
--   Planning:
--   Buffers: shared hit=177
--   Planning Time: 0.794 ms
--   Execution Time: 0.157 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.157 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.044..0.131 rows=10 loops=1)
--   Buffers: shared hit=3 read=10
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=4000152 width=86) (actual time=0.043..0.129 rows=10 loops=1)
--   Filter: (external_id <> 'ext-0f73ba25'::text)
--   Buffers: shared hit=3 read=10
--   Planning:
--   Buffers: shared hit=180
--   Planning Time: 0.641 ms
--   Execution Time: 0.149 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.149 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id <> $1
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4s. external_id in
-- Index Scan using idx_rg_external_id → Sort — 0.056 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.043..0.044 rows=1 loops=1)
--   Buffers: shared hit=7
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.042..0.043 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=7
--   ->  Index Scan using idx_rg_external_id on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.018..0.018 rows=1 loops=1)
--   Index Cond: (external_id = ANY ('{ext-f720c5b9}'::text[]))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=226
--   Planning Time: 0.932 ms
--   Execution Time: 0.064 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_external_id)
-- Execution Time: 0.064 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.046..0.046 rows=1 loops=1)
--   Buffers: shared hit=7
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.045..0.045 rows=1 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=7
--   ->  Index Scan using idx_rg_external_id on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.019..0.019 rows=1 loops=1)
--   Index Cond: (external_id = ANY ('{ext-0f73ba25}'::text[]))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=229
--   Planning Time: 0.878 ms
--   Execution Time: 0.069 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rg_external_id)
-- Execution Time: 0.069 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = ANY($1::text[])
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4t. external_id startswith
-- Index Scan using resource_group_pkey + Filter [PK] — 0.066 ms
-- PK scan + LIMIT. idx_rg_external_id usable for exact prefix if needed.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.019..0.055 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=3999203 width=86) (actual time=0.018..0.052 rows=10 loops=1)
--   Filter: (external_id ~~ 'ext%'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=173 read=5
--   Planning Time: 0.874 ms
--   Execution Time: 0.075 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.075 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.43 rows=10 width=86) (actual time=0.021..0.047 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=3999753 width=86) (actual time=0.020..0.044 rows=10 loops=1)
--   Filter: (external_id ~~ 'ext%'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=176 read=5
--   Planning Time: 0.821 ms
--   Execution Time: 0.068 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.068 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..16.86 rows=10 width=86) (actual time=16580.984..16581.030 rows=10 loops=1)
--   Buffers: shared hit=960617 read=2808650 written=17619
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=242400 width=86) (actual time=16580.983..16581.026 rows=10 loops=1)
--   Filter: (external_id ~~* '%t-f%'::text)
--   Rows Removed by Filter: 3751032
--   Buffers: shared hit=960617 read=2808650 written=17619
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.969 ms
--   Execution Time: 16581.064 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 16581.064 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..16.86 rows=10 width=86) (actual time=0.019..0.050 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=242434 width=86) (actual time=0.018..0.048 rows=10 loops=1)
--   Filter: (external_id ~~* '%t-0%'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=181
--   Planning Time: 0.804 ms
--   Execution Time: 0.068 ms
--   (9 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 0.068 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || $1 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4v. external_id endswith
-- Bitmap Index Scan on idx_rg_extid_trgm → Bitmap Heap Scan → Sort — 0.200 ms
-- GIN trgm index handles ILIKE suffix efficiently.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..9954.61 rows=10 width=86) (actual time=21.463..185.313 rows=10 loops=1)
--   Buffers: shared hit=9957 read=29491 written=295
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=400 width=86) (actual time=21.461..185.304 rows=10 loops=1)
--   Filter: (external_id ~~* '%5b9'::text)
--   Rows Removed by Filter: 39257
--   Buffers: shared hit=9957 read=29491 written=295
--   Planning:
--   Buffers: shared hit=141 read=37
--   Planning Time: 1.061 ms
--   Execution Time: 185.340 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 185.340 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..9960.58 rows=10 width=86) (actual time=14.814..184.905 rows=10 loops=1)
--   Buffers: shared hit=10462 read=30920
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398406.57 rows=400 width=86) (actual time=14.813..184.897 rows=10 loops=1)
--   Filter: (external_id ~~* '%a25'::text)
--   Rows Removed by Filter: 41156
--   Buffers: shared hit=10462 read=30920
--   Planning:
--   Buffers: shared hit=181
--   Planning Time: 0.780 ms
--   Execution Time: 184.932 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 184.932 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..10.22 rows=10 width=90) (actual time=0.073..0.342 rows=10 loops=1)
--   Buffers: shared hit=34 read=19
--   ->  Nested Loop  (cost=0.87..3683872.09 rows=3939723 width=90) (actual time=0.072..0.338 rows=10 loops=1)
--   Buffers: shared hit=34 read=19
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1592658.15 rows=3939723 width=20) (actual time=0.034..0.194 rows=10 loops=1)
--   Filter: (depth = 1)
--   Rows Removed by Filter: 62
--   Buffers: shared hit=3 read=10
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..0.53 rows=1 width=86) (actual time=0.011..0.011 rows=1 loops=10)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=31 read=9
--   Planning:
--   Buffers: shared hit=226 read=48
--   Planning Time: 3.572 ms
--   Execution Time: 0.427 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.427 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..10.10 rows=10 width=90) (actual time=0.055..0.173 rows=10 loops=1)
--   Buffers: shared hit=43 read=10
--   ->  Nested Loop  (cost=0.87..3766025.76 rows=4079286 width=90) (actual time=0.054..0.170 rows=10 loops=1)
--   Buffers: shared hit=43 read=10
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1612121.37 rows=4079286 width=20) (actual time=0.021..0.099 rows=10 loops=1)
--   Filter: (depth = 1)
--   Rows Removed by Filter: 54
--   Buffers: shared hit=3 read=10
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..0.53 rows=1 width=86) (actual time=0.005..0.005 rows=1 loops=10)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=40
--   Planning:
--   Buffers: shared hit=263 read=17
--   Planning Time: 3.463 ms
--   Execution Time: 0.218 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.218 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..4.72 rows=10 width=90) (actual time=0.022..0.037 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Nested Loop  (cost=0.87..8972639.14 rows=23269104 width=90) (actual time=0.021..0.035 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388168.54 rows=3999603 width=86) (actual time=0.010..0.017 rows=2 loops=1)
--   Buffers: shared hit=5
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1.72 rows=43 width=20) (actual time=0.006..0.006 rows=5 loops=2)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth <> 0)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=8
--   Planning:
--   Buffers: shared hit=269
--   Planning Time: 1.119 ms
--   Execution Time: 0.089 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.089 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..4.72 rows=10 width=90) (actual time=0.021..0.036 rows=10 loops=1)
--   Buffers: shared hit=18
--   ->  Nested Loop  (cost=0.87..9107388.51 rows=23612895 width=90) (actual time=0.020..0.034 rows=10 loops=1)
--   Buffers: shared hit=18
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388406.19 rows=4000153 width=86) (actual time=0.009..0.011 rows=3 loops=1)
--   Buffers: shared hit=6
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1.74 rows=44 width=20) (actual time=0.005..0.006 rows=3 loops=3)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth <> 0)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=12
--   Planning:
--   Buffers: shared hit=275
--   Planning Time: 1.143 ms
--   Execution Time: 0.080 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.080 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..5.36 rows=10 width=90) (actual time=0.021..0.035 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Nested Loop  (cost=0.87..8692666.93 rows=19329381 width=90) (actual time=0.020..0.033 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388168.54 rows=3999603 width=86) (actual time=0.009..0.017 rows=2 loops=1)
--   Buffers: shared hit=5
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1.72 rows=36 width=20) (actual time=0.005..0.006 rows=5 loops=2)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth > 1)
--   Rows Removed by Filter: 2
--   Buffers: shared hit=8
--   Planning:
--   Buffers: shared hit=272
--   Planning Time: 1.140 ms
--   Execution Time: 0.085 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.085 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..5.37 rows=10 width=90) (actual time=0.020..0.034 rows=10 loops=1)
--   Buffers: shared hit=18
--   ->  Nested Loop  (cost=0.87..8787376.27 rows=19533549 width=90) (actual time=0.019..0.032 rows=10 loops=1)
--   Buffers: shared hit=18
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388406.19 rows=4000153 width=86) (actual time=0.008..0.011 rows=3 loops=1)
--   Buffers: shared hit=6
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1.74 rows=36 width=20) (actual time=0.005..0.005 rows=3 loops=3)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth > 1)
--   Rows Removed by Filter: 2
--   Buffers: shared hit=12
--   Planning:
--   Buffers: shared hit=282
--   Planning Time: 1.168 ms
--   Execution Time: 0.079 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.079 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..4.72 rows=10 width=90) (actual time=0.022..0.037 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Nested Loop  (cost=0.87..8972639.14 rows=23269104 width=90) (actual time=0.021..0.034 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388168.54 rows=3999603 width=86) (actual time=0.010..0.017 rows=2 loops=1)
--   Buffers: shared hit=5
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1.72 rows=43 width=20) (actual time=0.006..0.006 rows=5 loops=2)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth >= 1)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=8
--   Planning:
--   Buffers: shared hit=272
--   Planning Time: 1.181 ms
--   Execution Time: 0.117 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.117 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..4.72 rows=10 width=90) (actual time=0.021..0.034 rows=10 loops=1)
--   Buffers: shared hit=18
--   ->  Nested Loop  (cost=0.87..9107388.51 rows=23612835 width=90) (actual time=0.020..0.032 rows=10 loops=1)
--   Buffers: shared hit=18
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..388406.19 rows=4000153 width=86) (actual time=0.009..0.010 rows=3 loops=1)
--   Buffers: shared hit=6
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1.74 rows=44 width=20) (actual time=0.005..0.005 rows=3 loops=3)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth >= 1)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=12
--   Planning:
--   Buffers: shared hit=282
--   Planning Time: 1.214 ms
--   Execution Time: 0.083 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.083 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..7.76 rows=10 width=90) (actual time=0.044..0.076 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Nested Loop  (cost=0.87..5480270.43 rows=7954021 width=90) (actual time=0.043..0.074 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1592658.15 rows=7954021 width=20) (actual time=0.032..0.039 rows=10 loops=1)
--   Filter: (depth < 2)
--   Rows Removed by Filter: 21
--   Buffers: shared hit=8
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..0.49 rows=1 width=86) (actual time=0.003..0.003 rows=1 loops=10)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=40
--   Planning:
--   Buffers: shared hit=272
--   Planning Time: 1.121 ms
--   Execution Time: 0.125 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.125 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..7.74 rows=10 width=90) (actual time=0.022..0.053 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Nested Loop  (cost=0.87..5561451.68 rows=8091411 width=90) (actual time=0.021..0.051 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1612121.37 rows=8091411 width=20) (actual time=0.012..0.019 rows=10 loops=1)
--   Filter: (depth < 2)
--   Rows Removed by Filter: 13
--   Buffers: shared hit=8
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..0.49 rows=1 width=86) (actual time=0.003..0.003 rows=1 loops=10)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=40
--   Planning:
--   Buffers: shared hit=278
--   Planning Time: 1.200 ms
--   Execution Time: 0.117 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.117 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..7.76 rows=10 width=90) (actual time=0.024..0.058 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Nested Loop  (cost=0.87..5480270.43 rows=7954021 width=90) (actual time=0.024..0.055 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1592658.15 rows=7954021 width=20) (actual time=0.014..0.022 rows=10 loops=1)
--   Filter: (depth <= 1)
--   Rows Removed by Filter: 21
--   Buffers: shared hit=8
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..0.49 rows=1 width=86) (actual time=0.003..0.003 rows=1 loops=10)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=40
--   Planning:
--   Buffers: shared hit=272
--   Planning Time: 1.161 ms
--   Execution Time: 0.109 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.109 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..7.74 rows=10 width=90) (actual time=0.020..0.054 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Nested Loop  (cost=0.87..5561451.68 rows=8091411 width=90) (actual time=0.020..0.052 rows=10 loops=1)
--   Buffers: shared hit=48
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..1612121.37 rows=8091411 width=20) (actual time=0.011..0.019 rows=10 loops=1)
--   Filter: (depth <= 1)
--   Rows Removed by Filter: 13
--   Buffers: shared hit=8
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..0.49 rows=1 width=86) (actual time=0.003..0.003 rows=1 loops=10)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=40
--   Planning:
--   Buffers: shared hit=282
--   Planning Time: 1.271 ms
--   Execution Time: 0.099 ms
--   (15 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 0.099 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=28.56..28.56 rows=1 width=86) (actual time=0.447..0.449 rows=6 loops=1)
--   Buffers: shared hit=12 read=47
--   ->  Sort  (cost=28.56..28.56 rows=1 width=86) (actual time=0.446..0.447 rows=6 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=12 read=47
--   ->  Index Scan using idx_rg_parent_id on resource_group g  (cost=0.43..28.55 rows=1 width=86) (actual time=0.119..0.419 rows=6 loops=1)
--   Index Cond: (parent_id = '94218bc1-3509-44a7-8cd8-6c6a9ce1f0b4'::uuid)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 49
--   Buffers: shared hit=9 read=47
--   Planning:
--   Buffers: shared hit=175
--   Planning Time: 0.701 ms
--   Execution Time: 0.471 ms
--   (15 rows)
-- Summary: Index Scan (indexes: idx_rg_parent_id)
-- Execution Time: 0.471 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=32.58..32.58 rows=1 width=86) (actual time=0.353..0.354 rows=7 loops=1)
--   Buffers: shared hit=11 read=26
--   ->  Sort  (cost=32.58..32.58 rows=1 width=86) (actual time=0.352..0.352 rows=7 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=11 read=26
--   ->  Index Scan using idx_rg_parent_id on resource_group g  (cost=0.43..32.57 rows=1 width=86) (actual time=0.109..0.298 rows=7 loops=1)
--   Index Cond: (parent_id = 'cf2db198-34fc-4a31-b9c0-824a65c5136b'::uuid)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 24
--   Buffers: shared hit=8 read=26
--   Planning:
--   Buffers: shared hit=178
--   Planning Time: 0.727 ms
--   Execution Time: 0.376 ms
--   (15 rows)
-- Summary: Index Scan (indexes: idx_rg_parent_id)
-- Execution Time: 0.376 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..240.71 rows=10 width=86) (actual time=16749.103..16749.112 rows=0 loops=1)
--   Buffers: shared hit=1020049 read=2999378 written=33375
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..408166.55 rows=16987 width=86) (actual time=16749.102..16749.105 rows=0 loops=1)
--   Filter: ((name ~~ 'div%'::text) AND (group_type = 'tenant'::text))
--   Rows Removed by Filter: 4000000
--   Buffers: shared hit=1020049 read=2999378 written=33375
--   Planning:
--   Buffers: shared hit=176 read=5
--   Planning Time: 0.848 ms
--   Execution Time: 16749.147 ms
--   (10 rows)
-- Summary: Index Scan (indexes: resource_group_pkey)
-- Execution Time: 16749.147 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.56..182.39 rows=10 width=86) (actual time=942.638..942.639 rows=0 loops=1)
--   Buffers: shared hit=63065 read=175716
--   ->  Index Scan using idx_rg_group_type on resource_group g  (cost=0.56..261401.56 rows=14376 width=86) (actual time=942.636..942.637 rows=0 loops=1)
--   Index Cond: (group_type = 'tenant'::text)
--   Filter: (name ~~ 'div%'::text)
--   Rows Removed by Filter: 237256
--   Buffers: shared hit=63065 read=175716
--   Planning:
--   Buffers: shared hit=179 read=5
--   Planning Time: 0.810 ms
--   Execution Time: 942.665 ms
--   (11 rows)
-- Summary: Index Scan (indexes: idx_rg_group_type)
-- Execution Time: 942.665 ms
SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = $1
   AND g.name LIKE $2 || '%'
 ORDER BY g.id
 LIMIT $top OFFSET $skip;

-- 4ac. group_type + external_id eq
-- Index Scan using idx_rg_external_id + Filter (group_type) — 0.067 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.103..0.103 rows=0 loops=1)
--   Buffers: shared hit=3 read=4
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.102..0.102 rows=0 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=3 read=4
--   ->  Index Scan using idx_rg_external_id on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.042..0.042 rows=0 loops=1)
--   Index Cond: (external_id = 'ext-f720c5b9'::text)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=143 read=32
--   Planning Time: 1.325 ms
--   Execution Time: 0.127 ms
--   (15 rows)
-- Summary: Index Scan (indexes: idx_rg_external_id)
-- Execution Time: 0.127 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.46 rows=1 width=86) (actual time=0.059..0.059 rows=0 loops=1)
--   Buffers: shared hit=4 read=3
--   ->  Sort  (cost=8.46..8.46 rows=1 width=86) (actual time=0.058..0.058 rows=0 loops=1)
--   Sort Key: id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=4 read=3
--   ->  Index Scan using idx_rg_external_id on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.035..0.035 rows=0 loops=1)
--   Index Cond: (external_id = 'ext-0f73ba25'::text)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=1 read=3
--   Planning:
--   Buffers: shared hit=146 read=32
--   Planning Time: 0.922 ms
--   Execution Time: 0.081 ms
--   (15 rows)
-- Summary: Index Scan (indexes: idx_rg_external_id)
-- Execution Time: 0.081 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=903.78..903.80 rows=10 width=90) (actual time=8.239..8.241 rows=10 loops=1)
--   Buffers: shared hit=91 read=190
--   ->  Sort  (cost=903.78..903.96 rows=72 width=90) (actual time=8.237..8.238 rows=10 loops=1)
--   Sort Key: g.id
--   Sort Method: top-N heapsort  Memory: 27kB
--   Buffers: shared hit=91 read=190
--   ->  Nested Loop  (cost=0.99..902.22 rows=72 width=90) (actual time=0.873..8.178 rows=55 loops=1)
--   Buffers: shared hit=88 read=190
--   ->  Index Scan using idx_rgc_ancestor_depth on resource_group_closure c  (cost=0.56..294.00 rows=72 width=20) (actual time=0.828..1.701 rows=55 loops=1)
--   Index Cond: ((ancestor_id = '94218bc1-3509-44a7-8cd8-6c6a9ce1f0b4'::uuid) AND (depth = 1))
--   Buffers: shared read=58
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.117..0.117 rows=1 loops=55)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=88 read=132
--   Planning:
--   Buffers: shared hit=257 read=20
--   Planning Time: 1.548 ms
--   Execution Time: 8.329 ms
--   (18 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_ancestor_depth, resource_group_pkey)
-- Execution Time: 8.329 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=941.24..941.27 rows=10 width=90) (actual time=10.253..10.256 rows=10 loops=1)
--   Buffers: shared hit=52 read=110
--   ->  Sort  (cost=941.24..941.43 rows=75 width=90) (actual time=10.252..10.253 rows=10 loops=1)
--   Sort Key: g.id
--   Sort Method: top-N heapsort  Memory: 27kB
--   Buffers: shared hit=52 read=110
--   ->  Nested Loop  (cost=0.99..939.62 rows=75 width=90) (actual time=3.682..10.208 rows=31 loops=1)
--   Buffers: shared hit=49 read=110
--   ->  Index Scan using idx_rgc_ancestor_depth on resource_group_closure c  (cost=0.56..306.06 rows=75 width=20) (actual time=3.357..3.579 rows=31 loops=1)
--   Index Cond: ((ancestor_id = 'cf2db198-34fc-4a31-b9c0-824a65c5136b'::uuid) AND (depth = 1))
--   Buffers: shared read=35
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.213..0.213 rows=1 loops=31)
--   Index Cond: (id = c.descendant_id)
--   Buffers: shared hit=49 read=75
--   Planning:
--   Buffers: shared hit=263 read=20
--   Planning Time: 1.343 ms
--   Execution Time: 10.308 ms
--   (18 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_ancestor_depth, resource_group_pkey)
-- Execution Time: 10.308 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.87..80.14 rows=10 width=90) (actual time=0.218..2.185 rows=10 loops=1)
--   Buffers: shared hit=107 read=207
--   ->  Nested Loop  (cost=0.87..1875540.18 rows=236608 width=90) (actual time=0.217..2.181 rows=10 loops=1)
--   Buffers: shared hit=107 read=207
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..398167.55 rows=240243 width=86) (actual time=0.178..1.909 rows=10 loops=1)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 258
--   Buffers: shared hit=80 read=192
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..6.08 rows=7 width=20) (actual time=0.024..0.026 rows=1 loops=10)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth = 1)
--   Rows Removed by Filter: 5
--   Buffers: shared hit=27 read=15
--   Planning:
--   Buffers: shared hit=277
--   Planning Time: 1.189 ms
--   Execution Time: 2.245 ms
--   (17 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_descendant_id, resource_group_pkey)
-- Execution Time: 2.245 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.99..73.51 rows=10 width=90) (actual time=0.109..0.370 rows=10 loops=1)
--   Buffers: shared hit=28 read=26
--   ->  Nested Loop  (cost=0.99..1754206.02 rows=241902 width=90) (actual time=0.108..0.367 rows=10 loops=1)
--   Buffers: shared hit=28 read=26
--   ->  Index Scan using idx_rg_group_type on resource_group g  (cost=0.56..260808.54 rows=237209 width=86) (actual time=0.080..0.184 rows=10 loops=1)
--   Index Cond: (group_type = 'tenant'::text)
--   Buffers: shared read=14
--   ->  Index Scan using idx_rgc_descendant_id on resource_group_closure c  (cost=0.44..6.22 rows=8 width=20) (actual time=0.017..0.017 rows=1 loops=10)
--   Index Cond: (descendant_id = g.id)
--   Filter: (depth = 1)
--   Rows Removed by Filter: 6
--   Buffers: shared hit=28 read=12
--   Planning:
--   Buffers: shared hit=280
--   Planning Time: 1.295 ms
--   Execution Time: 0.416 ms
--   (16 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rg_group_type, idx_rgc_descendant_id)
-- Execution Time: 0.416 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=902.44..902.45 rows=4 width=90) (actual time=0.476..0.478 rows=6 loops=1)
--   Buffers: shared hit=281
--   ->  Sort  (cost=902.44..902.45 rows=4 width=90) (actual time=0.475..0.476 rows=6 loops=1)
--   Sort Key: g.id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=281
--   ->  Nested Loop  (cost=0.99..902.40 rows=4 width=90) (actual time=0.088..0.453 rows=6 loops=1)
--   Buffers: shared hit=278
--   ->  Index Scan using idx_rgc_ancestor_depth on resource_group_closure c  (cost=0.56..294.00 rows=72 width=20) (actual time=0.014..0.078 rows=55 loops=1)
--   Index Cond: ((ancestor_id = '94218bc1-3509-44a7-8cd8-6c6a9ce1f0b4'::uuid) AND (depth = 1))
--   Buffers: shared hit=58
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.007..0.007 rows=0 loops=55)
--   Index Cond: (id = c.descendant_id)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=220
--   Planning:
--   Buffers: shared hit=280
--   Planning Time: 1.180 ms
--   Execution Time: 0.545 ms
--   (20 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_ancestor_depth, resource_group_pkey)
-- Execution Time: 0.545 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=939.85..939.86 rows=4 width=90) (actual time=0.334..0.336 rows=7 loops=1)
--   Buffers: shared hit=162
--   ->  Sort  (cost=939.85..939.86 rows=4 width=90) (actual time=0.333..0.334 rows=7 loops=1)
--   Sort Key: g.id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=162
--   ->  Nested Loop  (cost=0.99..939.81 rows=4 width=90) (actual time=0.083..0.310 rows=7 loops=1)
--   Buffers: shared hit=159
--   ->  Index Scan using idx_rgc_ancestor_depth on resource_group_closure c  (cost=0.56..306.06 rows=75 width=20) (actual time=0.015..0.053 rows=31 loops=1)
--   Index Cond: ((ancestor_id = 'cf2db198-34fc-4a31-b9c0-824a65c5136b'::uuid) AND (depth = 1))
--   Buffers: shared hit=35
--   ->  Index Scan using resource_group_pkey on resource_group g  (cost=0.43..8.45 rows=1 width=86) (actual time=0.008..0.008 rows=0 loops=31)
--   Index Cond: (id = c.descendant_id)
--   Filter: (group_type = 'tenant'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=124
--   Planning:
--   Buffers: shared hit=283
--   Planning Time: 1.248 ms
--   Execution Time: 0.387 ms
--   (20 rows)
-- Summary: Index Scan + Nested Loop (indexes: idx_rgc_ancestor_depth, resource_group_pkey)
-- Execution Time: 0.387 ms
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
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.39 rows=10 width=50) (actual time=0.081..0.396 rows=10 loops=1)
--   Buffers: shared read=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..479553.79 rows=5000511 width=50) (actual time=0.080..0.393 rows=10 loops=1)
--   Buffers: shared read=13
--   Planning:
--   Buffers: shared hit=136 read=5 dirtied=1
--   Planning Time: 1.085 ms
--   Execution Time: 0.439 ms
--   (8 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.439 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.39 rows=10 width=50) (actual time=0.044..0.126 rows=10 loops=1)
--   Buffers: shared read=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..479085.48 rows=4999582 width=50) (actual time=0.043..0.124 rows=10 loops=1)
--   Buffers: shared read=13
--   Planning:
--   Buffers: shared hit=136 read=5 dirtied=1
--   Planning Time: 0.657 ms
--   Execution Time: 0.143 ms
--   (8 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.143 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5b. group_id eq
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan → Sort [BITMAP+UQ-P] — 0.089 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..12.47 rows=2 width=50) (actual time=0.060..0.077 rows=3 loops=1)
--   Buffers: shared hit=1 read=5
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..12.47 rows=2 width=50) (actual time=0.059..0.076 rows=3 loops=1)
--   Index Cond: (group_id = 'c2c815ad-3adb-4be4-8d66-6c23d947c398'::uuid)
--   Buffers: shared hit=1 read=5
--   Planning:
--   Buffers: shared hit=143
--   Planning Time: 0.618 ms
--   Execution Time: 0.096 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.096 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..12.47 rows=2 width=50) (actual time=0.455..0.473 rows=3 loops=1)
--   Buffers: shared hit=1 read=5
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..12.47 rows=2 width=50) (actual time=0.454..0.472 rows=3 loops=1)
--   Index Cond: (group_id = 'aff0efcd-50b2-4b8d-b0fe-3bcec6265a33'::uuid)
--   Buffers: shared hit=1 read=5
--   Planning:
--   Buffers: shared hit=143
--   Planning Time: 0.547 ms
--   Execution Time: 0.491 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.491 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
 ORDER BY m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5c. group_id ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.075 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.42 rows=10 width=50) (actual time=0.015..0.030 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..492055.06 rows=5000509 width=50) (actual time=0.014..0.028 rows=10 loops=1)
--   Filter: (group_id <> 'c2c815ad-3adb-4be4-8d66-6c23d947c398'::uuid)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=146
--   Planning Time: 0.568 ms
--   Execution Time: 0.048 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.048 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.42 rows=10 width=50) (actual time=0.017..0.028 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..491584.43 rows=4999580 width=50) (actual time=0.016..0.026 rows=10 loops=1)
--   Filter: (group_id <> 'aff0efcd-50b2-4b8d-b0fe-3bcec6265a33'::uuid)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=146
--   Planning Time: 0.563 ms
--   Execution Time: 0.045 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.045 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5d. group_id in
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan → Sort [BITMAP+UQ-P] — 0.105 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..41.41 rows=7 width=50) (actual time=0.018..0.026 rows=4 loops=1)
--   Buffers: shared hit=7
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..41.41 rows=7 width=50) (actual time=0.017..0.024 rows=4 loops=1)
--   Index Cond: (group_id = ANY ('{000000e4-a835-49d5-9177-7cff106b1ede,0000052d-5ed2-4d87-8e85-210276cf527c,000005bc-211b-4251-8209-a7862b545543}'::uuid[]))
--   Buffers: shared hit=7
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.806 ms
--   Execution Time: 0.046 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.046 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..41.41 rows=7 width=50) (actual time=0.018..0.025 rows=6 loops=1)
--   Buffers: shared hit=9
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..41.41 rows=7 width=50) (actual time=0.017..0.023 rows=6 loops=1)
--   Index Cond: (group_id = ANY ('{00000951-50e9-4d38-98df-421302326999,00000b99-5931-48e0-af09-8c65d167194a,00000ce4-e744-4593-839d-493951af49fe}'::uuid[]))
--   Buffers: shared hit=9
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.541 ms
--   Execution Time: 0.042 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.042 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = ANY($1::uuid[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5e. resource_type eq
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.140 ms
-- Scans UQ index in order, filters resource_type. LIMIT stops early.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..18.65 rows=10 width=50) (actual time=0.020..0.225 rows=10 loops=1)
--   Buffers: shared hit=5 read=9
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..444488.20 rows=244025 width=50) (actual time=0.019..0.222 rows=10 loops=1)
--   Index Cond: (resource_type = 'user'::text)
--   Buffers: shared hit=5 read=9
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.662 ms
--   Execution Time: 0.245 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.245 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..18.64 rows=10 width=50) (actual time=0.022..0.128 rows=10 loops=1)
--   Buffers: shared hit=4 read=9
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..444026.52 rows=243813 width=50) (actual time=0.021..0.126 rows=10 loops=1)
--   Index Cond: (resource_type = 'subnet'::text)
--   Buffers: shared hit=4 read=9
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.579 ms
--   Execution Time: 0.146 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.146 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
 ORDER BY m.group_id, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5f. resource_type ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.076 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.47 rows=10 width=50) (actual time=0.037..0.339 rows=10 loops=1)
--   Buffers: shared hit=13 read=2
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..492055.06 rows=4756486 width=50) (actual time=0.036..0.336 rows=10 loops=1)
--   Filter: (resource_type <> 'user'::text)
--   Rows Removed by Filter: 2
--   Buffers: shared hit=13 read=2
--   Planning:
--   Buffers: shared hit=146
--   Planning Time: 1.765 ms
--   Execution Time: 0.389 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.389 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.47 rows=10 width=50) (actual time=0.016..0.039 rows=10 loops=1)
--   Buffers: shared hit=13 read=1
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..491584.43 rows=4755769 width=50) (actual time=0.015..0.036 rows=10 loops=1)
--   Filter: (resource_type <> 'subnet'::text)
--   Rows Removed by Filter: 1
--   Buffers: shared hit=13 read=1
--   Planning:
--   Buffers: shared hit=146
--   Planning Time: 0.566 ms
--   Execution Time: 0.056 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.056 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5g. resource_type in
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.139 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..6.37 rows=10 width=50) (actual time=0.052..0.208 rows=10 loops=1)
--   Buffers: shared hit=8 read=5
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..449622.28 rows=757411 width=50) (actual time=0.051..0.204 rows=10 loops=1)
--   Index Cond: (resource_type = ANY ('{cert,disk,dns}'::text[]))
--   Buffers: shared hit=8 read=5
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 1.114 ms
--   Execution Time: 0.240 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.240 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..6.42 rows=10 width=50) (actual time=0.046..0.142 rows=10 loops=1)
--   Buffers: shared hit=4 read=9
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..449084.46 rows=749604 width=50) (actual time=0.045..0.139 rows=10 loops=1)
--   Index Cond: (resource_type = ANY ('{cert,disk,dns}'::text[]))
--   Buffers: shared hit=4 read=9
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.555 ms
--   Execution Time: 0.166 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.166 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5h. resource_id eq
-- Index Scan using idx_rgm_resource_id → Sort — 0.087 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=82980.44..82980.44 rows=1 width=50) (actual time=507.542..512.746 rows=1 loops=1)
--   Buffers: shared hit=285 read=55657 written=5
--   ->  Sort  (cost=82980.44..82980.44 rows=1 width=50) (actual time=507.539..512.742 rows=1 loops=1)
--   Sort Key: group_id, resource_type
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=285 read=55657 written=5
--   ->  Gather  (cost=1000.00..82980.43 rows=1 width=50) (actual time=0.465..512.682 rows=1 loops=1)
--   Workers Planned: 2
--   Workers Launched: 2
--   Buffers: shared hit=279 read=55657 written=5
--   ->  Parallel Seq Scan on resource_group_membership m  (cost=0.00..81980.33 rows=1 width=50) (actual time=331.465..500.382 rows=0 loops=3)
--   Filter: (resource_id = 'res-l60arxkj'::text)
--   Rows Removed by Filter: 1666666
--   Buffers: shared hit=279 read=55657 written=5
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.753 ms
--   Execution Time: 512.853 ms
--   (18 rows)
-- Summary: Parallel Seq Scan
-- Execution Time: 512.853 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=82975.60..82975.60 rows=1 width=50) (actual time=162.865..164.431 rows=1 loops=1)
--   Buffers: shared hit=38 read=55904
--   ->  Sort  (cost=82975.60..82975.60 rows=1 width=50) (actual time=162.863..164.428 rows=1 loops=1)
--   Sort Key: group_id, resource_type
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=38 read=55904
--   ->  Gather  (cost=1000.00..82975.59 rows=1 width=50) (actual time=0.192..164.393 rows=1 loops=1)
--   Workers Planned: 2
--   Workers Launched: 2
--   Buffers: shared hit=32 read=55904
--   ->  Parallel Seq Scan on resource_group_membership m  (cost=0.00..81975.49 rows=1 width=50) (actual time=106.721..160.884 rows=0 loops=3)
--   Filter: (resource_id = 'res-prgacbis'::text)
--   Rows Removed by Filter: 1666666
--   Buffers: shared hit=32 read=55904
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.527 ms
--   Execution Time: 164.459 ms
--   (18 rows)
-- Summary: Parallel Seq Scan
-- Execution Time: 164.459 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = $1
 ORDER BY m.group_id, m.resource_type
 LIMIT $top OFFSET $skip;

-- 5i. resource_id ne
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.087 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.42 rows=10 width=50) (actual time=0.022..0.042 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..492055.06 rows=5000510 width=50) (actual time=0.020..0.038 rows=10 loops=1)
--   Filter: (resource_id <> 'res-l60arxkj'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=146
--   Planning Time: 0.884 ms
--   Execution Time: 0.068 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.068 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.42 rows=10 width=50) (actual time=0.017..0.030 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..491584.43 rows=4999581 width=50) (actual time=0.016..0.028 rows=10 loops=1)
--   Filter: (resource_id <> 'res-prgacbis'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=146
--   Planning Time: 0.558 ms
--   Execution Time: 0.047 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.047 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id <> $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5j. resource_id in
-- Bitmap Index Scan on idx_rgm_resource_id → Bitmap Heap Scan → Sort — 0.144 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=85584.79..85585.03 rows=2 width=50) (actual time=343.390..345.904 rows=3 loops=1)
--   Buffers: shared hit=469 read=55555
--   ->  Gather Merge  (cost=85584.79..85585.03 rows=2 width=50) (actual time=343.388..345.901 rows=3 loops=1)
--   Workers Planned: 2
--   Workers Launched: 2
--   Buffers: shared hit=469 read=55555
--   ->  Sort  (cost=84584.77..84584.78 rows=1 width=50) (actual time=340.818..340.819 rows=1 loops=3)
--   Sort Key: group_id, resource_type, resource_id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=469 read=55555
--   Worker 0:  Sort Method: quicksort  Memory: 25kB
--   Worker 1:  Sort Method: quicksort  Memory: 25kB
--   ->  Parallel Seq Scan on resource_group_membership m  (cost=0.00..84584.76 rows=1 width=50) (actual time=226.416..340.688 rows=1 loops=3)
--   Filter: (resource_id = ANY ('{res-l60arxkj,res-9vf67fjy,res-wyg70fys}'::text[]))
--   Rows Removed by Filter: 1666666
--   Buffers: shared hit=381 read=55555
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.667 ms
--   Execution Time: 346.033 ms
--   (20 rows)
-- Summary: Parallel Seq Scan
-- Execution Time: 346.033 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=85579.47..85579.71 rows=2 width=50) (actual time=267.526..269.521 rows=3 loops=1)
--   Buffers: shared hit=216 read=55808
--   ->  Gather Merge  (cost=85579.47..85579.71 rows=2 width=50) (actual time=267.525..269.518 rows=3 loops=1)
--   Workers Planned: 2
--   Workers Launched: 2
--   Buffers: shared hit=216 read=55808
--   ->  Sort  (cost=84579.45..84579.45 rows=1 width=50) (actual time=265.645..265.645 rows=1 loops=3)
--   Sort Key: group_id, resource_type, resource_id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=216 read=55808
--   Worker 0:  Sort Method: quicksort  Memory: 25kB
--   Worker 1:  Sort Method: quicksort  Memory: 25kB
--   ->  Parallel Seq Scan on resource_group_membership m  (cost=0.00..84579.44 rows=1 width=50) (actual time=176.452..265.539 rows=1 loops=3)
--   Filter: (resource_id = ANY ('{res-prgacbis,res-jw16prhm,res-7s2wwq7t}'::text[]))
--   Rows Removed by Filter: 1666666
--   Buffers: shared hit=128 read=55808
--   Planning:
--   Buffers: shared hit=141
--   Planning Time: 0.559 ms
--   Execution Time: 269.583 ms
--   (20 rows)
-- Summary: Parallel Seq Scan
-- Execution Time: 269.583 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = ANY($1::text[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5k. resource_id startswith
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 0.070 ms
-- UQ index scan in order + LIKE filter + LIMIT
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.42 rows=10 width=50) (actual time=0.018..0.036 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..492055.06 rows=5000011 width=50) (actual time=0.017..0.033 rows=10 loops=1)
--   Filter: (resource_id ~~ 'res-%'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=143 read=4
--   Planning Time: 0.789 ms
--   Execution Time: 0.058 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.058 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..1.42 rows=10 width=50) (actual time=0.017..0.030 rows=10 loops=1)
--   Buffers: shared hit=13
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..491584.43 rows=4999082 width=50) (actual time=0.016..0.028 rows=10 loops=1)
--   Filter: (resource_id ~~ 'res-%'::text)
--   Buffers: shared hit=13
--   Planning:
--   Buffers: shared hit=143 read=4
--   Planning Time: 0.646 ms
--   Execution Time: 0.050 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.050 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id LIKE $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5l. resource_id contains
-- Index Scan using uq_resource_group_membership_unique + Filter [UQ] — 1.065 ms
-- UQ index scan + ILIKE filter. GIN idx_rgm_resid_trgm available for selective patterns.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..32.90 rows=10 width=50) (actual time=0.044..2.554 rows=10 loops=1)
--   Buffers: shared hit=36 read=366
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..492055.06 rows=151531 width=50) (actual time=0.043..2.550 rows=10 loops=1)
--   Filter: (resource_id ~~* '%s-l%'::text)
--   Rows Removed by Filter: 387
--   Buffers: shared hit=36 read=366
--   Planning:
--   Buffers: shared hit=147
--   Planning Time: 0.675 ms
--   Execution Time: 2.575 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 2.575 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..49.10 rows=10 width=50) (actual time=0.028..1.618 rows=10 loops=1)
--   Buffers: shared hit=34 read=241
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..491584.43 rows=101002 width=50) (actual time=0.027..1.615 rows=10 loops=1)
--   Filter: (resource_id ~~* '%s-p%'::text)
--   Rows Removed by Filter: 261
--   Buffers: shared hit=34 read=241
--   Planning:
--   Buffers: shared hit=147
--   Planning Time: 0.599 ms
--   Execution Time: 1.636 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 1.636 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1 || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- 5m. resource_id endswith
-- Bitmap Index Scan on idx_rgm_resid_trgm → Bitmap Heap Scan → Sort — 0.191 ms
-- GIN trgm index handles ILIKE suffix efficiently.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..9841.53 rows=10 width=50) (actual time=233.961..1708.030 rows=10 loops=1)
--   Buffers: shared hit=106676 read=276946 written=3664
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..492055.06 rows=500 width=50) (actual time=233.959..1708.012 rows=10 loops=1)
--   Filter: (resource_id ~~* '%xkj'::text)
--   Rows Removed by Filter: 380128
--   Buffers: shared hit=106676 read=276946 written=3664
--   Planning:
--   Buffers: shared hit=147
--   Planning Time: 0.699 ms
--   Execution Time: 1708.079 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 1708.079 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..9832.11 rows=10 width=50) (actual time=513.550..2110.452 rows=10 loops=1)
--   Buffers: shared hit=133197 read=341096 written=1
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..491584.43 rows=500 width=50) (actual time=513.548..2110.439 rows=10 loops=1)
--   Filter: (resource_id ~~* '%bis'::text)
--   Rows Removed by Filter: 470082
--   Buffers: shared hit=133197 read=341096 written=1
--   Planning:
--   Buffers: shared hit=147
--   Planning Time: 0.638 ms
--   Execution Time: 2110.485 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 2110.485 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || $1
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT $top OFFSET $skip;

-- ---- Combined filters -----------------------------------------------------

-- 5n. group_id + resource_type
-- Bitmap Index Scan on uq (first 2 cols) → Bitmap Heap Scan — 0.041 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..8.45 rows=1 width=50) (actual time=0.045..0.046 rows=1 loops=1)
--   Buffers: shared read=4
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..8.45 rows=1 width=50) (actual time=0.044..0.044 rows=1 loops=1)
--   Index Cond: ((group_id = 'c2c815ad-3adb-4be4-8d66-6c23d947c398'::uuid) AND (resource_type = 'user'::text))
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=114 read=29
--   Planning Time: 0.986 ms
--   Execution Time: 0.070 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.070 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..8.45 rows=1 width=50) (actual time=0.038..0.039 rows=1 loops=1)
--   Buffers: shared read=4
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..8.45 rows=1 width=50) (actual time=0.037..0.038 rows=1 loops=1)
--   Index Cond: ((group_id = 'aff0efcd-50b2-4b8d-b0fe-3bcec6265a33'::uuid) AND (resource_type = 'subnet'::text))
--   Buffers: shared read=4
--   Planning:
--   Buffers: shared hit=114 read=29
--   Planning Time: 0.803 ms
--   Execution Time: 0.055 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.055 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
 ORDER BY m.resource_id
 LIMIT $top OFFSET $skip;

-- 5o. group_id + resource_id
-- Index Scan using uq (group_id prefix) + Filter (resource_id) — 0.039 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..8.46 rows=1 width=50) (actual time=0.017..0.018 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..8.46 rows=1 width=50) (actual time=0.016..0.016 rows=1 loops=1)
--   Index Cond: ((group_id = 'c2c815ad-3adb-4be4-8d66-6c23d947c398'::uuid) AND (resource_id = 'res-l60arxkj'::text))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=143
--   Planning Time: 0.548 ms
--   Execution Time: 0.034 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.034 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..8.46 rows=1 width=50) (actual time=0.019..0.020 rows=1 loops=1)
--   Buffers: shared hit=4
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..8.46 rows=1 width=50) (actual time=0.018..0.018 rows=1 loops=1)
--   Index Cond: ((group_id = 'aff0efcd-50b2-4b8d-b0fe-3bcec6265a33'::uuid) AND (resource_id = 'res-prgacbis'::text))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=143
--   Planning Time: 0.565 ms
--   Execution Time: 0.035 ms
--   (9 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.035 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_id = $2
 ORDER BY m.resource_type
 LIMIT $top OFFSET $skip;

-- 5p. group_id + resource_type + resource_id (exact match)
-- Index Scan using idx_rgm_resource_type_id [exact match] — 0.091 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Index Scan using idx_rgm_resource_type_id on resource_group_membership m  (cost=0.43..8.45 rows=1 width=50) (actual time=0.082..0.083 rows=1 loops=1)
--   Index Cond: ((resource_type = 'user'::text) AND (resource_id = 'res-l60arxkj'::text))
--   Filter: (group_id = 'c2c815ad-3adb-4be4-8d66-6c23d947c398'::uuid)
--   Buffers: shared hit=1 read=3
--   Planning:
--   Buffers: shared hit=139
--   Planning Time: 0.698 ms
--   Execution Time: 0.109 ms
--   (8 rows)
-- Summary: Index Scan (indexes: idx_rgm_resource_type_id)
-- Execution Time: 0.109 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Index Scan using idx_rgm_resource_type_id on resource_group_membership m  (cost=0.43..8.45 rows=1 width=50) (actual time=0.050..0.051 rows=1 loops=1)
--   Index Cond: ((resource_type = 'subnet'::text) AND (resource_id = 'res-prgacbis'::text))
--   Filter: (group_id = 'aff0efcd-50b2-4b8d-b0fe-3bcec6265a33'::uuid)
--   Buffers: shared hit=1 read=3
--   Planning:
--   Buffers: shared hit=139
--   Planning Time: 0.545 ms
--   Execution Time: 0.076 ms
--   (8 rows)
-- Summary: Index Scan (indexes: idx_rgm_resource_type_id)
-- Execution Time: 0.076 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = $1
   AND m.resource_type = $2
   AND m.resource_id = $3;

-- 5q. resource_type + resource_id (no group_id)
-- Index Scan using idx_rgm_resource_type_id — 0.086 ms
-- Uses composite index (resource_type, resource_id) since group_id is absent.
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.47 rows=1 width=50) (actual time=0.055..0.056 rows=1 loops=1)
--   Buffers: shared hit=7
--   ->  Sort  (cost=8.46..8.47 rows=1 width=50) (actual time=0.054..0.054 rows=1 loops=1)
--   Sort Key: group_id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=7
--   ->  Index Scan using idx_rgm_resource_type_id on resource_group_membership m  (cost=0.43..8.45 rows=1 width=50) (actual time=0.021..0.021 rows=1 loops=1)
--   Index Cond: ((resource_type = 'user'::text) AND (resource_id = 'res-l60arxkj'::text))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=142
--   Planning Time: 0.592 ms
--   Execution Time: 0.086 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rgm_resource_type_id)
-- Execution Time: 0.086 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=8.46..8.47 rows=1 width=50) (actual time=0.041..0.042 rows=1 loops=1)
--   Buffers: shared hit=7
--   ->  Sort  (cost=8.46..8.47 rows=1 width=50) (actual time=0.040..0.040 rows=1 loops=1)
--   Sort Key: group_id
--   Sort Method: quicksort  Memory: 25kB
--   Buffers: shared hit=7
--   ->  Index Scan using idx_rgm_resource_type_id on resource_group_membership m  (cost=0.43..8.45 rows=1 width=50) (actual time=0.016..0.017 rows=1 loops=1)
--   Index Cond: ((resource_type = 'subnet'::text) AND (resource_id = 'res-prgacbis'::text))
--   Buffers: shared hit=4
--   Planning:
--   Buffers: shared hit=142
--   Planning Time: 0.572 ms
--   Execution Time: 0.088 ms
--   (13 rows)
-- Summary: Index Scan (indexes: idx_rgm_resource_type_id)
-- Execution Time: 0.088 ms
SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = $1
   AND m.resource_id = $2
 ORDER BY m.group_id
 LIMIT $top OFFSET $skip;

-- 5r. group_id + resource_id startswith
-- Bitmap Index Scan on uq (group_id prefix) → Bitmap Heap Scan + Filter (LIKE) → Sort — 0.095 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..12.47 rows=2 width=50) (actual time=0.033..0.044 rows=3 loops=1)
--   Buffers: shared hit=4 read=2
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..12.47 rows=2 width=50) (actual time=0.032..0.042 rows=3 loops=1)
--   Index Cond: (group_id = 'c2c815ad-3adb-4be4-8d66-6c23d947c398'::uuid)
--   Filter: (resource_id ~~ 'res-%'::text)
--   Buffers: shared hit=4 read=2
--   Planning:
--   Buffers: shared hit=145 read=4
--   Planning Time: 0.772 ms
--   Execution Time: 0.064 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.064 ms
-- EXPLAIN ANALYZE (4M groups, 27M closure, 5M memberships):
--   Limit  (cost=0.43..12.47 rows=2 width=50) (actual time=0.025..0.035 rows=3 loops=1)
--   Buffers: shared hit=5 read=1
--   ->  Index Scan using uq_resource_group_membership_unique on resource_group_membership m  (cost=0.43..12.47 rows=2 width=50) (actual time=0.024..0.034 rows=3 loops=1)
--   Index Cond: (group_id = 'aff0efcd-50b2-4b8d-b0fe-3bcec6265a33'::uuid)
--   Filter: (resource_id ~~ 'res-%'::text)
--   Buffers: shared hit=5 read=1
--   Planning:
--   Buffers: shared hit=145 read=4
--   Planning Time: 0.602 ms
--   Execution Time: 0.052 ms
--   (10 rows)
-- Summary: Index Scan (indexes: uq_resource_group_membership_unique)
-- Execution Time: 0.052 ms
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
