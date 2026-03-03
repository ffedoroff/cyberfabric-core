#!/usr/bin/env python3
"""
Generate and seed resource_group data into a PostgreSQL Docker container.

Targets: ~20 types, ~4M groups, ~5M memberships.
Maintains closure table integrity and FK constraints.
"""

import subprocess
import sys
import time
import uuid
import random
import string
import os
import tempfile

DB_NAME = "rg_test"
DB_USER = "postgres"
DB_PASS = "postgres"
DB_PORT = "25432"
CONTAINER = "rg-postgres"

MIGRATION_SQL = "migration.sql"

# ── Target volumes ──────────────────────────────────────────────────────────
NUM_TENANTS = 1000
NUM_GROUPS = 4_000_000
NUM_MEMBERSHIPS = 5_000_000
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


def docker_cp(local_path, container_path):
    r = run(f'docker cp {local_path} {CONTAINER}:{container_path}')
    if r.returncode != 0:
        print(f"Docker cp error: {r.stderr}", file=sys.stderr)
        sys.exit(1)


def wait_for_pg():
    for i in range(30):
        r = run(f'docker exec {CONTAINER} pg_isready -U {DB_USER}')
        if r.returncode == 0:
            # Also verify we can actually run a query
            r2 = run(f'docker exec {CONTAINER} psql -U {DB_USER} -d {DB_NAME} -c "SELECT 1;"')
            if r2.returncode == 0:
                return
        time.sleep(1)
    print("PostgreSQL did not start in 30s", file=sys.stderr)
    sys.exit(1)


# ── 1. Start PostgreSQL container ──────────────────────────────────────────
def start_container():
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


# ── 3. Generate and load data ─────────────────────────────────────────────
def generate_and_seed():
    print(f"Generating ~{NUM_GROUPS:,} groups, ~{NUM_MEMBERSHIPS:,} memberships...")

    # ── Types ─────────────────────────────────────────────────────────────
    type_names = [
        "tenant", "region", "zone", "cluster", "namespace",
        "project", "environment", "department", "team", "division",
        "organization", "business_unit", "cost_center", "workspace",
        "application", "service_group", "network", "storage_pool",
        "compute_pool", "security_zone",
    ]
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

    # Insert types via SQL
    print("  Inserting types...")
    types_sql = "INSERT INTO resource_group_type (code, parents) VALUES\n"
    type_rows = []
    for t in type_names:
        parents = type_parents[t]
        pg_arr = '{' + ','.join(f'"{p}"' if p else '""' for p in parents) + '}'
        type_rows.append(f"  ('{t}', '{pg_arr}')")
    types_sql += ",\n".join(type_rows) + ";\n"
    psql(types_sql)
    print(f"  {len(type_names)} types inserted.")

    # ── Generate groups in memory ──────────────────────────────────────────
    # Which types can be children of which
    children_of_type = {}
    for t, parents in type_parents.items():
        for p in parents:
            if p == '':
                continue
            children_of_type.setdefault(p, []).append(t)

    groups = []       # (id, parent_id, group_type, name, tenant_id, external_id)
    parent_map = {}   # child_id -> parent_id (for closure)
    all_ids = []
    pool = []         # (parent_gid, child_type, tenant_id) — incremental

    def add_group(gtype, parent_id, tenant_id, name=None):
        gid = str(uuid.uuid4())
        if name is None:
            suffix = ''.join(random.choices(string.ascii_lowercase + string.digits, k=6))
            name = f"{gtype}-{suffix}"
        ext_id = f"ext-{gid[:8]}"
        groups.append((gid, parent_id, gtype, name, tenant_id, ext_id))
        all_ids.append(gid)
        if parent_id is not None:
            parent_map[gid] = parent_id
        # Add this group's potential children to the pool
        if gtype in children_of_type:
            for ct in children_of_type[gtype]:
                pool.append((gid, ct, tenant_id))
        return gid

    # Create root tenants
    print("  Creating root groups...")
    for i in range(NUM_TENANTS):
        tid = str(uuid.uuid4())
        add_group("tenant", None, tid, name=f"tenant-{i:04d}")

    for i in range(5):
        tid = str(uuid.uuid4())
        add_group("organization", None, tid, name=f"org-{i:04d}")

    print(f"    Roots: {len(groups)}, pool: {len(pool)}")

    # Fill remaining groups using incremental pool
    print("  Generating group hierarchy (incremental)...")
    while len(groups) < NUM_GROUPS and pool:
        parent_gid, child_type, tenant_id = random.choice(pool)
        add_group(child_type, parent_gid, tenant_id)
        if len(groups) % 500_000 == 0:
            print(f"    {len(groups):,} groups, pool: {len(pool):,}")

    actual_groups = len(groups)
    print(f"  Generated {actual_groups:,} groups")

    # ── Write groups CSV and COPY ──────────────────────────────────────────
    tmpdir = tempfile.mkdtemp()

    print("  Writing groups CSV...")
    groups_csv = os.path.join(tmpdir, "groups.csv")
    with open(groups_csv, 'w') as f:
        for gid, pid, gtype, name, tid, eid in groups:
            pid_val = pid if pid else "\\N"
            f.write(f"{gid}\t{pid_val}\t{gtype}\t{name}\t{tid}\t{eid}\n")

    print(f"  Loading {actual_groups:,} groups via COPY...")
    psql("ALTER TABLE resource_group DISABLE TRIGGER ALL;")
    docker_cp(groups_csv, "/tmp/groups.csv")
    psql("COPY resource_group (id, parent_id, group_type, name, tenant_id, external_id) FROM '/tmp/groups.csv';")
    psql("ALTER TABLE resource_group ENABLE TRIGGER ALL;")
    os.remove(groups_csv)
    print("  Groups loaded.")

    # ── Closure table ──────────────────────────────────────────────────────
    print("  Computing closure table...")
    closure_csv = os.path.join(tmpdir, "closure.csv")
    closure_count = 0
    with open(closure_csv, 'w') as f:
        for gid, *_ in groups:
            # Self-link
            f.write(f"{gid}\t{gid}\t0\n")
            closure_count += 1
            # Walk ancestry chain
            current = gid
            depth = 0
            while current in parent_map:
                depth += 1
                ancestor = parent_map[current]
                f.write(f"{ancestor}\t{gid}\t{depth}\n")
                closure_count += 1
                current = ancestor
            if closure_count % 5_000_000 == 0:
                print(f"    {closure_count:,} closure rows...")

    print(f"  Generated {closure_count:,} closure rows")
    print("  Loading closure via COPY...")
    psql("ALTER TABLE resource_group_closure DISABLE TRIGGER ALL;")
    docker_cp(closure_csv, "/tmp/closure.csv")
    psql("COPY resource_group_closure (ancestor_id, descendant_id, depth) FROM '/tmp/closure.csv';")
    psql("ALTER TABLE resource_group_closure ENABLE TRIGGER ALL;")
    os.remove(closure_csv)
    print("  Closure loaded.")

    # ── Memberships ────────────────────────────────────────────────────────
    print("  Generating memberships...")
    membership_set = set()
    group_tenant = {gid: tid for gid, _, _, _, tid, _ in groups}
    memberships_csv = os.path.join(tmpdir, "memberships.csv")

    with open(memberships_csv, 'w') as f:
        attempts = 0
        count = 0
        while count < NUM_MEMBERSHIPS and attempts < NUM_MEMBERSHIPS * 3:
            attempts += 1
            gid = random.choice(all_ids)
            rtype = random.choice(RESOURCE_TYPES)
            rid = f"res-{''.join(random.choices(string.ascii_lowercase + string.digits, k=8))}"
            key = (gid, rtype, rid)
            if key not in membership_set:
                membership_set.add(key)
                f.write(f"{gid}\t{rtype}\t{rid}\t{group_tenant[gid]}\n")
                count += 1
                if count % 1_000_000 == 0:
                    print(f"    {count:,} memberships...")

    print(f"  Generated {count:,} memberships")
    print("  Loading memberships via COPY...")
    psql("ALTER TABLE resource_group_membership DISABLE TRIGGER ALL;")
    docker_cp(memberships_csv, "/tmp/memberships.csv")
    psql("COPY resource_group_membership (group_id, resource_type, resource_id, tenant_id) FROM '/tmp/memberships.csv';")
    psql("ALTER TABLE resource_group_membership ENABLE TRIGGER ALL;")
    os.remove(memberships_csv)
    print("  Memberships loaded.")

    os.rmdir(tmpdir)
    return actual_groups, closure_count, count


# ── 4. Verify ─────────────────────────────────────────────────────────────
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

    print("\n── Types breakdown (top 10) ──")
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


# ── Main ──────────────────────────────────────────────────────────────────
def main():
    start_container()
    run_migration()

    num_groups, num_closure, num_memberships = generate_and_seed()

    print("\nRunning ANALYZE...")
    psql("ANALYZE;")
    print("ANALYZE complete.")

    verify()

    print(f"\n── Done! Container '{CONTAINER}' is running on port {DB_PORT} ──")
    print(f"Connect: psql -h localhost -p {DB_PORT} -U {DB_USER} -d {DB_NAME}")


if __name__ == "__main__":
    main()
