#!/usr/bin/env python3
"""
Generate and seed resource_group data into a PostgreSQL Docker container.

Targets: ~20 types, ~100K groups, ~200K memberships.
Maintains closure table integrity and FK constraints.
"""

import subprocess
import sys
import time
import uuid
import random
import string
import io

DB_NAME = "rg_test"
DB_USER = "postgres"
DB_PASS = "postgres"
DB_PORT = "25432"
CONTAINER = "rg-postgres"

MIGRATION_SQL = "migration.sql"
SEEDING_SQL = "seeding.sql"

# ── Target volumes ──────────────────────────────────────────────────────────
NUM_TYPES = 20
NUM_TENANTS = 500
NUM_GROUPS = 100_000
NUM_MEMBERSHIPS = 200_000
RESOURCE_TYPES = ["vm", "disk", "nic", "snapshot", "volume", "ip", "subnet",
                  "lb", "firewall", "dns", "cert", "key", "secret", "policy",
                  "role", "user", "service", "endpoint", "queue", "topic"]


def run(cmd, **kwargs):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True, **kwargs)


def psql(sql, database=DB_NAME):
    cmd = f'docker exec -i {CONTAINER} psql -U {DB_USER} -d {database} -v ON_ERROR_STOP=1'
    r = run(cmd, input=sql)
    if r.returncode != 0:
        print(f"PSQL ERROR:\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    return r.stdout


def wait_for_pg():
    for i in range(30):
        r = run(f'docker exec {CONTAINER} pg_isready -U {DB_USER}')
        if r.returncode == 0:
            return
        time.sleep(1)
    print("PostgreSQL did not start in 30s", file=sys.stderr)
    sys.exit(1)


# ── 1. Start PostgreSQL container ──────────────────────────────────────────
def start_container():
    # Stop old container if exists
    run(f'docker rm -f {CONTAINER} 2>/dev/null')
    print(f"Starting PostgreSQL container ({CONTAINER})...")
    r = run(
        f'docker run -d --name {CONTAINER} '
        f'-e POSTGRES_USER={DB_USER} '
        f'-e POSTGRES_PASSWORD={DB_PASS} '
        f'-e POSTGRES_DB={DB_NAME} '
        f'-p {DB_PORT}:5432 '
        f'postgres:17-alpine'
    )
    if r.returncode != 0:
        print(f"Docker error: {r.stderr}", file=sys.stderr)
        sys.exit(1)
    print("Waiting for PostgreSQL to be ready...")
    wait_for_pg()
    print("PostgreSQL is ready.")


# ── 2. Run migration ──────────────────────────────────────────────────────
def run_migration():
    print("Running migration...")
    with open(MIGRATION_SQL) as f:
        psql(f.read())
    print("Migration complete.")


# ── 3. Run original seed (optional, for reference) ────────────────────────
def run_original_seed():
    print("Running original seed data...")
    with open(SEEDING_SQL) as f:
        psql(f.read())
    print("Original seed complete.")


# ── 4. Generate data ──────────────────────────────────────────────────────
def generate():
    print(f"Generating {NUM_TYPES} types, {NUM_GROUPS} groups, {NUM_MEMBERSHIPS} memberships...")

    # ── Types ─────────────────────────────────────────────────────────────
    # Hierarchy: tenant -> region -> zone -> cluster -> namespace -> ...
    type_names = [
        "tenant", "region", "zone", "cluster", "namespace",
        "project", "environment", "department", "team", "division",
        "organization", "business_unit", "cost_center", "workspace",
        "application", "service_group", "network", "storage_pool",
        "compute_pool", "security_zone",
    ]
    # Define parent relationships as a chain with branches
    type_parents = {
        "tenant":        ['', 'tenant'],
        "region":        ['tenant'],
        "zone":          ['region'],
        "cluster":       ['zone'],
        "namespace":     ['cluster'],
        "project":       ['tenant'],
        "environment":   ['project'],
        "department":    ['tenant'],
        "team":          ['department'],
        "division":      ['tenant'],
        "organization":  ['', 'organization'],
        "business_unit": ['organization'],
        "cost_center":   ['business_unit'],
        "workspace":     ['tenant'],
        "application":   ['namespace', 'environment'],
        "service_group": ['application'],
        "network":       ['zone'],
        "storage_pool":  ['zone'],
        "compute_pool":  ['cluster'],
        "security_zone": ['network'],
    }

    # ── Types SQL ─────────────────────────────────────────────────────────
    # Delete the 3 types from original seed first, then re-insert all
    types_buf = io.StringIO()
    types_buf.write("DELETE FROM resource_group_membership;\n")
    types_buf.write("DELETE FROM resource_group_closure;\n")
    types_buf.write("DELETE FROM resource_group;\n")
    types_buf.write("DELETE FROM resource_group_type;\n")
    types_buf.write("INSERT INTO resource_group_type (code, parents) VALUES\n")
    type_rows = []
    for t in type_names:
        parents = type_parents[t]
        pg_arr = '{' + ','.join(f'"{p}"' if p else '""' for p in parents) + '}'
        type_rows.append(f"  ('{t}', '{pg_arr}')")
    types_buf.write(",\n".join(type_rows))
    types_buf.write(";\n")

    # ── Groups ────────────────────────────────────────────────────────────
    # Strategy: build a tree. Root nodes are tenants/organizations.
    # Then fill remaining groups by picking a random parent and assigning
    # a compatible type.

    groups = []          # list of (id, parent_id, group_type, name, tenant_id, external_id)
    id_by_type = {}      # type -> [uuid, ...]
    all_ids = []

    def new_uuid():
        return str(uuid.uuid4())

    def add_group(gtype, parent_id, tenant_id, name=None, ext_id=None):
        gid = new_uuid()
        if name is None:
            suffix = ''.join(random.choices(string.ascii_lowercase + string.digits, k=6))
            name = f"{gtype}-{suffix}"
        if ext_id is None:
            ext_id = f"ext-{gid[:8]}"
        groups.append((gid, parent_id, gtype, name, tenant_id, ext_id))
        id_by_type.setdefault(gtype, []).append(gid)
        all_ids.append(gid)
        return gid

    # Which types can be children of which
    children_of_type = {}  # parent_type -> [child_types]
    for t, parents in type_parents.items():
        for p in parents:
            if p == '':
                continue
            children_of_type.setdefault(p, []).append(t)

    root_types = [t for t, ps in type_parents.items() if '' in ps]  # tenant, organization

    # Create tenants
    tenant_ids = []
    for i in range(NUM_TENANTS):
        tid = new_uuid()
        gid = add_group("tenant", None, tid, name=f"tenant-{i:04d}")
        tenant_ids.append((gid, tid))  # group_id == tenant_id for root tenants

    # Create a few organizations
    for i in range(5):
        tid = new_uuid()
        add_group("organization", None, tid, name=f"org-{i:04d}")

    # Fill remaining groups
    remaining = NUM_GROUPS - len(groups)
    # Build list of (group_id, group_type, tenant_id) for parent selection
    def parent_pool():
        pool = []
        for g in groups:
            gid, pid, gtype, name, tid, eid = g
            if gtype in children_of_type:
                for ct in children_of_type[gtype]:
                    pool.append((gid, ct, tid))
        return pool

    batch = 0
    while len(groups) < NUM_GROUPS:
        pool = parent_pool()
        if not pool:
            break
        # Sample from pool
        to_add = min(remaining, len(pool) * 3, NUM_GROUPS - len(groups))
        for _ in range(to_add):
            if len(groups) >= NUM_GROUPS:
                break
            parent_gid, child_type, tenant_id = random.choice(pool)
            add_group(child_type, parent_gid, tenant_id)
        batch += 1
        if batch > 20:
            break

    actual_groups = len(groups)
    print(f"  Generated {actual_groups} groups (target {NUM_GROUPS})")

    # ── Closure table ─────────────────────────────────────────────────────
    # Build parent map, then compute transitive closure
    parent_map = {}  # child_id -> parent_id
    for gid, pid, *_ in groups:
        if pid is not None:
            parent_map[gid] = pid

    closure_rows = []  # (ancestor_id, descendant_id, depth)

    # Self-links
    for gid, *_ in groups:
        closure_rows.append((gid, gid, 0))

    # Ancestry chains
    for gid, *_ in groups:
        current = gid
        depth = 0
        while current in parent_map:
            depth += 1
            ancestor = parent_map[current]
            closure_rows.append((ancestor, gid, depth))
            current = ancestor

    print(f"  Generated {len(closure_rows)} closure rows")

    # ── Memberships ───────────────────────────────────────────────────────
    membership_set = set()
    memberships = []
    group_tenant = {gid: tid for gid, _, _, _, tid, _ in groups}

    attempts = 0
    while len(memberships) < NUM_MEMBERSHIPS and attempts < NUM_MEMBERSHIPS * 3:
        attempts += 1
        gid = random.choice(all_ids)
        rtype = random.choice(RESOURCE_TYPES)
        rid = f"res-{''.join(random.choices(string.ascii_lowercase + string.digits, k=8))}"
        key = (gid, rtype, rid)
        if key not in membership_set:
            membership_set.add(key)
            memberships.append((gid, rtype, rid, group_tenant[gid]))

    print(f"  Generated {len(memberships)} memberships")

    # ── Build SQL ─────────────────────────────────────────────────────────
    buf = io.StringIO()
    buf.write("BEGIN;\n")

    # Types
    buf.write(types_buf.getvalue())

    # Groups - use COPY format via multi-row VALUES in batches
    BATCH = 500
    buf.write("\n-- Groups\n")
    for i in range(0, len(groups), BATCH):
        batch = groups[i:i+BATCH]
        buf.write("INSERT INTO resource_group (id, parent_id, group_type, name, tenant_id, external_id) VALUES\n")
        rows = []
        for gid, pid, gtype, name, tid, eid in batch:
            pid_sql = f"'{pid}'" if pid else "NULL"
            # Escape single quotes in name
            safe_name = name.replace("'", "''")
            safe_eid = eid.replace("'", "''")
            rows.append(f"  ('{gid}', {pid_sql}, '{gtype}', '{safe_name}', '{tid}', '{safe_eid}')")
        buf.write(",\n".join(rows))
        buf.write(";\n")

    # Closure
    buf.write("\n-- Closure\n")
    for i in range(0, len(closure_rows), BATCH):
        batch = closure_rows[i:i+BATCH]
        buf.write("INSERT INTO resource_group_closure (ancestor_id, descendant_id, depth) VALUES\n")
        rows = []
        for aid, did, depth in batch:
            rows.append(f"  ('{aid}', '{did}', {depth})")
        buf.write(",\n".join(rows))
        buf.write(";\n")

    # Memberships
    buf.write("\n-- Memberships\n")
    for i in range(0, len(memberships), BATCH):
        batch = memberships[i:i+BATCH]
        buf.write("INSERT INTO resource_group_membership (group_id, resource_type, resource_id, tenant_id) VALUES\n")
        rows = []
        for gid, rtype, rid, tid in batch:
            rows.append(f"  ('{gid}', '{rtype}', '{rid}', '{tid}')")
        buf.write(",\n".join(rows))
        buf.write(";\n")

    buf.write("COMMIT;\n")
    return buf.getvalue()


# ── 5. Verify ─────────────────────────────────────────────────────────────
def verify():
    print("\n── Verification ──")
    queries = [
        ("Types",       "SELECT count(*) FROM resource_group_type"),
        ("Groups",      "SELECT count(*) FROM resource_group"),
        ("Closure",     "SELECT count(*) FROM resource_group_closure"),
        ("Memberships", "SELECT count(*) FROM resource_group_membership"),
    ]
    for label, q in queries:
        out = psql(f"\\t\n{q};")
        count = out.strip().split('\n')[-1].strip()
        print(f"  {label:15s}: {count}")

    print("\n── Types breakdown ──")
    out = psql("\\t\nSELECT group_type, count(*) FROM resource_group GROUP BY group_type ORDER BY count(*) DESC LIMIT 10;")
    for line in out.strip().split('\n'):
        line = line.strip()
        if line and '|' in line:
            print(f"  {line}")

    print("\n── Closure depth distribution ──")
    out = psql("\\t\nSELECT depth, count(*) FROM resource_group_closure GROUP BY depth ORDER BY depth;")
    for line in out.strip().split('\n'):
        line = line.strip()
        if line and '|' in line:
            print(f"  {line}")

    print("\n── Sample queries from queries.sql ──")

    # Test a few representative queries
    print("\n  Query 4a: List groups (LIMIT 5)")
    out = psql("SELECT g.id AS group_id, g.group_type, g.name FROM resource_group g ORDER BY g.id LIMIT 5;")
    print(out)

    print("  Query 5a: List memberships (LIMIT 5)")
    out = psql("SELECT m.group_id, m.resource_type, m.resource_id FROM resource_group_membership m ORDER BY m.group_id, m.resource_type, m.resource_id LIMIT 5;")
    print(out)

    # Run EXPLAIN ANALYZE on a few queries to see if indexes kick in at scale
    print("  EXPLAIN: group_type eq filter")
    out = psql("EXPLAIN ANALYZE SELECT g.id, g.group_type, g.name FROM resource_group g WHERE g.group_type = 'tenant' ORDER BY g.id LIMIT 10;")
    print(out)

    print("  EXPLAIN: closure depth filter")
    out = psql("EXPLAIN ANALYZE SELECT g.id, g.group_type, c.depth FROM resource_group g JOIN resource_group_closure c ON c.descendant_id = g.id WHERE c.depth = 1 ORDER BY g.id LIMIT 10;")
    print(out)

    print("  EXPLAIN: membership group_id eq")
    out = psql("EXPLAIN ANALYZE SELECT m.group_id, m.resource_type, m.resource_id FROM resource_group_membership m WHERE m.group_id = (SELECT id FROM resource_group LIMIT 1) ORDER BY m.resource_type, m.resource_id LIMIT 10;")
    print(out)


# ── Main ──────────────────────────────────────────────────────────────────
def main():
    start_container()
    run_migration()

    sql = generate()

    print(f"\nInserting generated data ({len(sql) // 1024} KB SQL)...")
    psql(sql)
    print("Data inserted.")

    print("Running ANALYZE...")
    psql("ANALYZE;")
    print("ANALYZE complete.")

    verify()

    print(f"\n── Done! Container '{CONTAINER}' is running on port {DB_PORT} ──")
    print(f"Connect: psql -h localhost -p {DB_PORT} -U {DB_USER} -d {DB_NAME}")


if __name__ == "__main__":
    main()
