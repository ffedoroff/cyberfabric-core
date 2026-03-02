#!/usr/bin/env python3
"""
Run every query from queries.sql against the seeded 100K database.
Collect EXPLAIN ANALYZE output and timing, then rewrite queries.sql
with updated comments.
"""

import subprocess
import sys
import re

DB_NAME = "rg_test"
DB_USER = "postgres"
CONTAINER = "rg-postgres"

TOP = 10
SKIP = 0


def psql(sql):
    cmd = f'docker exec -i {CONTAINER} psql -U {DB_USER} -d {DB_NAME} -v ON_ERROR_STOP=1'
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, input=sql)
    if r.returncode != 0:
        print(f"PSQL ERROR for query:\n{sql[:200]}\n{r.stderr}", file=sys.stderr)
        return f"ERROR: {r.stderr.strip()}"
    return r.stdout


def psql_val(sql):
    """Run query, return single scalar value."""
    cmd = f'docker exec -i {CONTAINER} psql -U {DB_USER} -d {DB_NAME} -t -A -v ON_ERROR_STOP=1'
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, input=sql)
    if r.returncode != 0:
        return None
    val = r.stdout.strip()
    # -t -A gives raw value without headers
    return val if val else None


# ── Fetch sample values from the database ──────────────────────────────────
print("Fetching sample data...")

SAMPLE_TYPE = psql_val("SELECT code FROM resource_group_type LIMIT 1;")
SAMPLE_TYPE2 = psql_val("SELECT code FROM resource_group_type WHERE code <> %s LIMIT 1;" % f"'{SAMPLE_TYPE}'") or "department"
SAMPLE_TYPES_ARR = psql_val("SELECT string_agg(code, ',') FROM (SELECT code FROM resource_group_type LIMIT 3) t;") or "tenant,region,zone"

SAMPLE_GROUP_ID = psql_val("SELECT id FROM resource_group LIMIT 1;")
SAMPLE_GROUP_ID2 = psql_val("SELECT id FROM resource_group WHERE id <> %s LIMIT 1;" % f"'{SAMPLE_GROUP_ID}'")
SAMPLE_GROUP_IDS_ARR = psql_val("SELECT string_agg(id::text, ',') FROM (SELECT id FROM resource_group LIMIT 3) t;")

SAMPLE_PARENT_ID = psql_val("SELECT parent_id FROM resource_group WHERE parent_id IS NOT NULL LIMIT 1;")
SAMPLE_PARENT_ID2 = psql_val("SELECT parent_id FROM resource_group WHERE parent_id IS NOT NULL AND parent_id <> %s LIMIT 1;" % f"'{SAMPLE_PARENT_ID}'")
SAMPLE_PARENT_IDS_ARR = psql_val("SELECT string_agg(parent_id::text, ',') FROM (SELECT DISTINCT parent_id FROM resource_group WHERE parent_id IS NOT NULL LIMIT 3) t;")

SAMPLE_NAME = psql_val("SELECT name FROM resource_group LIMIT 1;")
SAMPLE_EXT_ID = psql_val("SELECT external_id FROM resource_group WHERE external_id IS NOT NULL LIMIT 1;")

SAMPLE_TENANT_ID = psql_val("SELECT tenant_id FROM resource_group LIMIT 1;")

SAMPLE_ANCESTOR_ID = psql_val("SELECT ancestor_id FROM resource_group_closure WHERE depth > 0 LIMIT 1;")

SAMPLE_RES_TYPE = psql_val("SELECT resource_type FROM resource_group_membership LIMIT 1;")
SAMPLE_RES_TYPE2 = psql_val("SELECT resource_type FROM resource_group_membership WHERE resource_type <> %s LIMIT 1;" % f"'{SAMPLE_RES_TYPE}'") or "vm"
SAMPLE_RES_TYPES_ARR = psql_val("SELECT string_agg(DISTINCT resource_type, ',') FROM (SELECT DISTINCT resource_type FROM resource_group_membership LIMIT 3) t;")

SAMPLE_RES_ID = psql_val("SELECT resource_id FROM resource_group_membership LIMIT 1;")
SAMPLE_RES_ID2 = psql_val("SELECT resource_id FROM resource_group_membership WHERE resource_id <> %s LIMIT 1;" % f"'{SAMPLE_RES_ID}'")
SAMPLE_RES_IDS_ARR = psql_val("SELECT string_agg(resource_id, ',') FROM (SELECT resource_id FROM resource_group_membership LIMIT 3) t;")

SAMPLE_MEM_GROUP_ID = psql_val("SELECT group_id FROM resource_group_membership LIMIT 1;")
SAMPLE_MEM_GROUP_ID2 = psql_val("SELECT group_id FROM resource_group_membership WHERE group_id <> %s LIMIT 1;" % f"'{SAMPLE_MEM_GROUP_ID}'")
SAMPLE_MEM_GROUP_IDS_ARR = psql_val("SELECT string_agg(group_id::text, ',') FROM (SELECT DISTINCT group_id FROM resource_group_membership LIMIT 3) t;")

# For exact triple match in 5p
SAMPLE_MEM_TRIPLE = psql_val(
    "SELECT group_id || '|' || resource_type || '|' || resource_id "
    "FROM resource_group_membership LIMIT 1;"
)
if SAMPLE_MEM_TRIPLE:
    _parts = SAMPLE_MEM_TRIPLE.split('|')
    SAMPLE_MEM_GID = _parts[0].strip()
    SAMPLE_MEM_RTYPE = _parts[1].strip()
    SAMPLE_MEM_RID = _parts[2].strip()
else:
    SAMPLE_MEM_GID = SAMPLE_MEM_GROUP_ID
    SAMPLE_MEM_RTYPE = SAMPLE_RES_TYPE
    SAMPLE_MEM_RID = SAMPLE_RES_ID

# Name prefix for startswith (first 3 chars)
NAME_PREFIX = SAMPLE_NAME[:3] if SAMPLE_NAME else "ten"
NAME_CONTAINS = SAMPLE_NAME[2:5] if SAMPLE_NAME and len(SAMPLE_NAME) > 4 else "ant"
NAME_SUFFIX = SAMPLE_NAME[-3:] if SAMPLE_NAME else "ant"

EXT_PREFIX = SAMPLE_EXT_ID[:3] if SAMPLE_EXT_ID else "ext"
EXT_CONTAINS = SAMPLE_EXT_ID[2:5] if SAMPLE_EXT_ID and len(SAMPLE_EXT_ID) > 4 else "t-"
EXT_SUFFIX = SAMPLE_EXT_ID[-3:] if SAMPLE_EXT_ID else "001"

RES_ID_PREFIX = SAMPLE_RES_ID[:4] if SAMPLE_RES_ID else "res-"
RES_ID_CONTAINS = SAMPLE_RES_ID[2:5] if SAMPLE_RES_ID and len(SAMPLE_RES_ID) > 4 else "s-a"
RES_ID_SUFFIX = SAMPLE_RES_ID[-3:] if SAMPLE_RES_ID else "abc"

TYPE_PREFIX = SAMPLE_TYPE[:3] if SAMPLE_TYPE else "ten"
TYPE_CONTAINS = SAMPLE_TYPE[1:4] if SAMPLE_TYPE and len(SAMPLE_TYPE) > 3 else "ena"
TYPE_SUFFIX = SAMPLE_TYPE[-3:] if SAMPLE_TYPE else "ant"

print(f"  type={SAMPLE_TYPE}, group_id={SAMPLE_GROUP_ID}")
print(f"  parent_id={SAMPLE_PARENT_ID}, name={SAMPLE_NAME}")
print(f"  ancestor_id={SAMPLE_ANCESTOR_ID}")
print(f"  res_type={SAMPLE_RES_TYPE}, res_id={SAMPLE_RES_ID}")

# ── Define all queries ─────────────────────────────────────────────────────
# Each entry: (query_id, sql_with_real_params)
# We run both the query (for timing) and EXPLAIN ANALYZE (for plan).

QUERIES = [
    # Section 1: GET /types/{code}
    ("1", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code = '{SAMPLE_TYPE}';"""),

    # Section 2: GET /types
    ("2a", f"""SELECT code, parents
  FROM resource_group_type
 ORDER BY code
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("2b", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code = '{SAMPLE_TYPE}'
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("2c", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code <> '{SAMPLE_TYPE}'
 ORDER BY code
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("2d", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code = ANY(ARRAY[{','.join("'" + t.strip() + "'" for t in SAMPLE_TYPES_ARR.split(','))}])
 ORDER BY code
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("2e", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code LIKE '{TYPE_PREFIX}' || '%'
 ORDER BY code
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("2f", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || '{TYPE_CONTAINS}' || '%'
 ORDER BY code
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("2g", f"""SELECT code, parents
  FROM resource_group_type
 WHERE code ILIKE '%' || '{TYPE_SUFFIX}'
 ORDER BY code
 LIMIT {TOP} OFFSET {SKIP};"""),

    # Section 3: GET /groups/{group_id}
    ("3", f"""SELECT g.id          AS group_id,
       g.parent_id,
       g.group_type,
       g.name,
       g.tenant_id,
       g.external_id
  FROM resource_group g
 WHERE g.id = '{SAMPLE_GROUP_ID}';"""),

    # Section 4: GET /groups
    ("4a", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4b", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = '{SAMPLE_GROUP_ID}'
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4c", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id <> '{SAMPLE_GROUP_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4d", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.id = ANY(ARRAY[{','.join("'" + i.strip() + "'" for i in SAMPLE_GROUP_IDS_ARR.split(','))}]::uuid[])
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4e", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = '{SAMPLE_TYPE}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4f", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type <> '{SAMPLE_TYPE}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4g", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = ANY(ARRAY[{','.join("'" + t.strip() + "'" for t in SAMPLE_TYPES_ARR.split(','))}])
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4h", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = '{SAMPLE_PARENT_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4i", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id <> '{SAMPLE_PARENT_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4j", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.parent_id = ANY(ARRAY[{','.join("'" + i.strip() + "'" for i in SAMPLE_PARENT_IDS_ARR.split(','))}]::uuid[])
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4k", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = '{SAMPLE_NAME}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4l", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name <> '{SAMPLE_NAME}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4m", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name = ANY(ARRAY['{SAMPLE_NAME}'])
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4n", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name LIKE '{NAME_PREFIX}' || '%'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4o", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || '{NAME_CONTAINS}' || '%'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4p", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.name ILIKE '%' || '{NAME_SUFFIX}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4q", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = '{SAMPLE_EXT_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4r", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id <> '{SAMPLE_EXT_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4s", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id = ANY(ARRAY['{SAMPLE_EXT_ID}'])
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4t", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id LIKE '{EXT_PREFIX}' || '%'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4u", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || '{EXT_CONTAINS}' || '%'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4v", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.external_id ILIKE '%' || '{EXT_SUFFIX}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    # Depth queries
    ("4w", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth = 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4x", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <> 0
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4y_gt", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth > 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4y_ge", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4y_lt", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth < 2
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4y_le", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth <= 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4z", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.depth >= 1 AND c.depth <= 2
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    # Combined
    ("4aa", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = '{SAMPLE_TYPE}'
   AND g.parent_id = '{SAMPLE_PARENT_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4ab", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = '{SAMPLE_TYPE}'
   AND g.name LIKE '{NAME_PREFIX}' || '%'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4ac", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id
  FROM resource_group g
 WHERE g.group_type = '{SAMPLE_TYPE}'
   AND g.external_id = '{SAMPLE_EXT_ID}'
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4ad", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.ancestor_id = '{SAMPLE_ANCESTOR_ID}'
   AND c.depth = 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4ae", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE g.group_type = '{SAMPLE_TYPE}'
   AND c.depth = 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("4af", f"""SELECT g.id AS group_id, g.parent_id, g.group_type, g.name,
       g.tenant_id, g.external_id,
       c.depth
  FROM resource_group g
  JOIN resource_group_closure c ON c.descendant_id = g.id
 WHERE c.ancestor_id = '{SAMPLE_ANCESTOR_ID}'
   AND g.group_type = '{SAMPLE_TYPE}'
   AND c.depth = 1
 ORDER BY g.id
 LIMIT {TOP} OFFSET {SKIP};"""),

    # Section 5: memberships
    ("5a", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5b", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = '{SAMPLE_MEM_GROUP_ID}'
 ORDER BY m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5c", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id <> '{SAMPLE_MEM_GROUP_ID}'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5d", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = ANY(ARRAY[{','.join("'" + i.strip() + "'" for i in SAMPLE_MEM_GROUP_IDS_ARR.split(','))}]::uuid[])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5e", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = '{SAMPLE_RES_TYPE}'
 ORDER BY m.group_id, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5f", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type <> '{SAMPLE_RES_TYPE}'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5g", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = ANY(ARRAY[{','.join("'" + t.strip() + "'" for t in SAMPLE_RES_TYPES_ARR.split(','))}])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5h", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = '{SAMPLE_RES_ID}'
 ORDER BY m.group_id, m.resource_type
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5i", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id <> '{SAMPLE_RES_ID}'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5j", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id = ANY(ARRAY[{','.join("'" + i.strip() + "'" for i in SAMPLE_RES_IDS_ARR.split(','))}])
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5k", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id LIKE '{RES_ID_PREFIX}' || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5l", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || '{RES_ID_CONTAINS}' || '%'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5m", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_id ILIKE '%' || '{RES_ID_SUFFIX}'
 ORDER BY m.group_id, m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    # Combined membership filters
    ("5n", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = '{SAMPLE_MEM_GROUP_ID}'
   AND m.resource_type = '{SAMPLE_RES_TYPE}'
 ORDER BY m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5o", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = '{SAMPLE_MEM_GROUP_ID}'
   AND m.resource_id = '{SAMPLE_RES_ID}'
 ORDER BY m.resource_type
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5p", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = '{SAMPLE_MEM_GID}'
   AND m.resource_type = '{SAMPLE_MEM_RTYPE}'
   AND m.resource_id = '{SAMPLE_MEM_RID}';"""),

    ("5q", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.resource_type = '{SAMPLE_RES_TYPE}'
   AND m.resource_id = '{SAMPLE_RES_ID}'
 ORDER BY m.group_id
 LIMIT {TOP} OFFSET {SKIP};"""),

    ("5r", f"""SELECT m.group_id, m.resource_type, m.resource_id, m.tenant_id
  FROM resource_group_membership m
 WHERE m.group_id = '{SAMPLE_MEM_GROUP_ID}'
   AND m.resource_id LIKE '{RES_ID_PREFIX}' || '%'
 ORDER BY m.resource_type, m.resource_id
 LIMIT {TOP} OFFSET {SKIP};"""),
]


# ── Run benchmarks ─────────────────────────────────────────────────────────
# Warm up the cache
psql("SELECT count(*) FROM resource_group; SELECT count(*) FROM resource_group_closure; SELECT count(*) FROM resource_group_membership;")

results = {}  # query_id -> {plan: str, exec_time: str, scan_type: str}

print(f"\nRunning {len(QUERIES)} queries with EXPLAIN ANALYZE...\n")

for qid, sql in QUERIES:
    # Run EXPLAIN ANALYZE
    explain_sql = "EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) " + sql
    explain_out = psql(explain_sql)

    # Extract execution time
    exec_time = ""
    plan_lines = []
    for line in explain_out.strip().split('\n'):
        line = line.rstrip()
        if 'QUERY PLAN' in line or line.startswith('---') or line.startswith('(') and line.endswith(')') and 'rows' not in line.lower():
            continue
        plan_lines.append(line)
        m = re.search(r'Execution Time:\s+([\d.]+)\s+ms', line)
        if m:
            exec_time = m.group(1) + " ms"

    # Determine scan types used
    plan_text = '\n'.join(plan_lines)
    scan_types = set()
    if 'Index Only Scan' in plan_text:
        scan_types.add('Index Only Scan')
    if 'Index Scan' in plan_text and 'Index Only Scan' not in plan_text:
        scan_types.add('Index Scan')
    # Check for "Index Scan" lines that aren't "Index Only Scan"
    for line in plan_lines:
        if 'Index Scan' in line and 'Index Only' not in line:
            scan_types.add('Index Scan')
        if 'Index Only Scan' in line:
            scan_types.add('Index Only Scan')
    if 'Seq Scan' in plan_text:
        scan_types.add('Seq Scan')
    if 'Parallel Seq Scan' in plan_text:
        scan_types.discard('Seq Scan')
        scan_types.add('Parallel Seq Scan')
    if 'Bitmap' in plan_text:
        scan_types.add('Bitmap Scan')

    # Determine join type
    join_types = set()
    for jt in ['Hash Join', 'Merge Join', 'Nested Loop']:
        if jt in plan_text:
            join_types.add(jt)

    # Index names used
    idx_names = set()
    for m in re.finditer(r'using\s+(\S+)', plan_text, re.IGNORECASE):
        idx_names.add(m.group(1))

    scan_summary = ', '.join(sorted(scan_types))
    if join_types:
        scan_summary += ' + ' + ', '.join(sorted(join_types))
    if idx_names:
        scan_summary += f" (indexes: {', '.join(sorted(idx_names))})"

    results[qid] = {
        'plan': plan_text,
        'exec_time': exec_time,
        'scan_summary': scan_summary,
    }

    status = "OK" if "ERROR" not in plan_text else "ERR"
    print(f"  [{status}] {qid:5s}  {exec_time:>10s}  {scan_summary}")

# ── Print summary table ───────────────────────────────────────────────────
print("\n" + "=" * 90)
print(f"{'Query':6s} {'Time':>10s}  {'Plan Summary'}")
print("-" * 90)
for qid, _ in QUERIES:
    r = results[qid]
    print(f"{qid:6s} {r['exec_time']:>10s}  {r['scan_summary']}")
print("=" * 90)

# ── Rewrite queries.sql ──────────────────────────────────────────────────
print("\nRewriting queries.sql with benchmark results...")

with open("queries.sql") as f:
    original = f.read()

# Map query IDs to their comment block markers
# We'll find each query section and inject the EXPLAIN result after the section header comment

# Build a map: query_id -> explain comment block
def make_comment(qid):
    r = results[qid]
    lines = []
    lines.append(f"-- EXPLAIN ANALYZE (100K groups, 300K closure, 200K memberships):")
    # Extract key plan lines (skip buffers detail, keep structure)
    plan_lines = r['plan'].strip().split('\n')
    for pl in plan_lines:
        pl = pl.strip()
        if not pl:
            continue
        if pl.startswith('QUERY PLAN'):
            continue
        if pl.startswith('------'):
            continue
        lines.append(f"--   {pl}")
    lines.append(f"-- Summary: {r['scan_summary']}")
    lines.append(f"-- Execution Time: {r['exec_time']}")
    return '\n'.join(lines)


# We need to map section IDs in the file to our query IDs.
# The file uses patterns like "-- 2a. No filter" or "-- 4w. depth eq"
# Strategy: find each query block, insert explain comment before the SELECT.

# Parse the file into blocks. Each block starts with "-- Nx." pattern.
# We'll match our query IDs to these labels.

# Normalize query IDs: "4y_gt" -> match "4y. depth gt"
qid_to_file_label = {
    "1": "1",
    "2a": "2a", "2b": "2b", "2c": "2c", "2d": "2d", "2e": "2e", "2f": "2f", "2g": "2g",
    "3": "3",
    "4a": "4a", "4b": "4b", "4c": "4c", "4d": "4d", "4e": "4e", "4f": "4f", "4g": "4g",
    "4h": "4h", "4i": "4i", "4j": "4j", "4k": "4k", "4l": "4l", "4m": "4m", "4n": "4n",
    "4o": "4o", "4p": "4p", "4q": "4q", "4r": "4r", "4s": "4s", "4t": "4t", "4u": "4u",
    "4v": "4v", "4w": "4w", "4x": "4x",
    "4y_gt": "4y_gt", "4y_ge": "4y_ge", "4y_lt": "4y_lt", "4y_le": "4y_le",
    "4z": "4z",
    "4aa": "4aa", "4ab": "4ab", "4ac": "4ac", "4ad": "4ad", "4ae": "4ae", "4af": "4af",
    "5a": "5a", "5b": "5b", "5c": "5c", "5d": "5d", "5e": "5e", "5f": "5f", "5g": "5g",
    "5h": "5h", "5i": "5i", "5j": "5j", "5k": "5k", "5l": "5l", "5m": "5m",
    "5n": "5n", "5o": "5o", "5p": "5p", "5q": "5q", "5r": "5r",
}

# Strategy: process file line by line. Find comment lines that match query labels.
# After each such comment block (before the SELECT), insert the EXPLAIN result.
# Replace old "-- EXPLAIN:" comments.

lines = original.split('\n')
output_lines = []
i = 0
current_qid = None

# We'll detect query blocks by the "-- Na." pattern or section headers
# and insert explain results.

# Map of patterns to query IDs. The file uses various formats:
# "-- 2a. No filter"
# "-- 4y. depth gt"  (multiple 4y entries)
# Section headers like "-- 1. GET /types/{code}"

# Build regex patterns for each section
section_patterns = {}

# For numbered sub-queries like "2a", "4ab", etc.
for qid in results:
    if qid == "1":
        section_patterns[qid] = re.compile(r'^--\s+############.*\n--\s+1\.\s+GET\s+/types/\{code\}', re.MULTILINE)
    elif qid == "3":
        section_patterns[qid] = re.compile(r'^--\s+############.*\n--\s+3\.\s+GET\s+/groups/\{group_id\}', re.MULTILINE)

# Simpler approach: find each SELECT statement and match it to our query by looking
# at the preceding comment. Then replace the old EXPLAIN comment with new one.

# Even simpler: process line by line, detect old "-- EXPLAIN:" comments and replace,
# then insert new explain before each SELECT.

# Let's use a regex-based approach on the full text.
# For each query block in the file, there's a pattern:
# -- <label comment>
# -- EXPLAIN: <old explain>  (one or more lines starting with --)
# SELECT ...
#
# We want to replace the EXPLAIN block with our new one.

# Actually the cleanest approach: rebuild the file section by section.
# Let me identify each query by its SELECT pattern and the comment before it.

# Map each query ID to a regex that uniquely identifies the SELECT in the file
select_patterns = {
    "1":  r"(-- \d+[a-z]*\.\s+.*\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code = \$1;)",

    "2a": r"(-- 2a\. No filter\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+ORDER BY code\n\s+LIMIT \$top OFFSET \$skip;)",
    "2b": r"(-- 2b\. code eq\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code = \$1\n\s+LIMIT \$top OFFSET \$skip;)",
    "2c": r"(-- 2c\. code ne\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code <> \$1\n\s+ORDER BY code\n\s+LIMIT \$top OFFSET \$skip;)",
    "2d": r"(-- 2d\. code in\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code = ANY\(\$1::text\[\]\)\n\s+ORDER BY code\n\s+LIMIT \$top OFFSET \$skip;)",
    "2e": r"(-- 2e\. code startswith\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code LIKE \$1 \|\| '%'\n\s+ORDER BY code\n\s+LIMIT \$top OFFSET \$skip;)",
    "2f": r"(-- 2f\. code contains\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code ILIKE '%' \|\| \$1 \|\| '%'\n\s+ORDER BY code\n\s+LIMIT \$top OFFSET \$skip;)",
    "2g": r"(-- 2g\. code endswith\n(?:-- .*\n)*)(SELECT code, parents\n\s+FROM resource_group_type\n\s+WHERE code ILIKE '%' \|\| \$1\n\s+ORDER BY code\n\s+LIMIT \$top OFFSET \$skip;)",
}

# This regex approach is getting too complex. Let me use a simpler line-by-line approach.

# Strategy: split file into blocks separated by blank lines.
# For each block that starts with "-- Nx." find the matching query ID.
# Replace old EXPLAIN comments, insert new ones.

# Actually the simplest: just find the old EXPLAIN comment lines and replace them,
# plus add timing info. Let me do it differently.

# Split by the query sub-section markers (-- 2a., -- 4ab., etc.)
# and rebuild with new EXPLAIN comments.

# FINAL APPROACH: line-by-line scan. When we see "-- EXPLAIN:" at start of a comment
# block before a SELECT, we replace it. When we find a SELECT, we look up which
# query it corresponds to.

# Map: find "-- Na." label -> qid
label_to_qid = {}
# Standard labels
for qid in ["2a","2b","2c","2d","2e","2f","2g",
            "4a","4b","4c","4d","4e","4f","4g","4h","4i","4j",
            "4k","4l","4m","4n","4o","4p","4q","4r","4s","4t","4u","4v",
            "4w","4x",
            "4aa","4ab","4ac","4ad","4ae","4af",
            "5a","5b","5c","5d","5e","5f","5g","5h","5i","5j",
            "5k","5l","5m","5n","5o","5p","5q","5r"]:
    label_to_qid[qid] = qid

# Handle the depth queries with duplicate "4y." labels
# In the file: "-- 4y. depth gt", "-- 4y. depth ge", "-- 4y. depth lt", "-- 4y. depth le"
depth_map = {"gt": "4y_gt", "ge": "4y_ge", "lt": "4y_lt", "le": "4y_le"}

# Also section headers for query 1 and 3
# "-- 1. GET /types/{code}" -> qid "1"
# "-- 3. GET /groups/{group_id}" -> qid "3"


output_lines = []
i = 0
skip_old_explain = False
pending_qid = None

while i < len(lines):
    line = lines[i]

    # Detect sub-query labels like "-- 2a. No filter"
    m = re.match(r'^-- (\d+[a-z]*)\.\s+(.*)', line)
    if m:
        label = m.group(1)
        desc = m.group(2).strip()
        if label in label_to_qid:
            pending_qid = label_to_qid[label]
        elif label == "4y":
            # Disambiguate by description
            for key, mapped_qid in depth_map.items():
                if f"depth {key}" in desc:
                    pending_qid = mapped_qid
                    break

    # Detect section headers for queries 1 and 3
    if re.match(r'^-- 1\.\s+GET\s+/types/', line):
        pending_qid = "1"
    elif re.match(r'^-- 3\.\s+GET\s+/groups/', line):
        pending_qid = "3"

    # Skip old EXPLAIN comment lines (replace them)
    if line.startswith('-- EXPLAIN') and not line.startswith('-- EXPLAIN ANALYZE'):
        # Skip this and subsequent "-- " lines that are part of the old explain
        output_lines.append(line)  # keep the original EXPLAIN label
        i += 1
        # Skip continuation lines like "--   → Seq Scan..." or "-- On small data..."
        while i < len(lines) and lines[i].startswith('--') and not re.match(r'^-- \d+[a-z]*\.', lines[i]) and not lines[i].startswith('-- ####') and not lines[i].startswith('-- ----'):
            output_lines.append(lines[i])
            i += 1
        # Insert new EXPLAIN ANALYZE results
        if pending_qid and pending_qid in results:
            output_lines.append(make_comment(pending_qid))
        continue

    # If we hit a SELECT and there was no old EXPLAIN comment, inject before SELECT
    if line.startswith('SELECT') and pending_qid and pending_qid in results:
        # Check if we already inserted (look back for our marker)
        already = any('EXPLAIN ANALYZE (100K' in ol for ol in output_lines[-5:])
        if not already:
            output_lines.append(make_comment(pending_qid))
        pending_qid = None

    output_lines.append(line)
    i += 1

# Replace the SUMMARY section at the end
new_content = '\n'.join(output_lines)

# Also update the header comment about data size
new_content = new_content.replace(
    "EXPLAIN results from PostgreSQL 17.9 with seed data (5 groups, 9 closure, 6 memberships).",
    "EXPLAIN ANALYZE results from PostgreSQL 17 with generated data (100K groups, 307K closure, 200K memberships)."
)
new_content = new_content.replace(
    "On small data PG planner always prefers Seq Scan over index access — this is expected.",
    "At this scale PG planner uses indexes where available; Seq Scans indicate missing indexes."
)

with open("queries.sql", "w") as f:
    f.write(new_content)

print("queries.sql updated with EXPLAIN ANALYZE results.")
print("Done!")
