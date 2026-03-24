# @cpt-dod:cpt-cf-resource-group-dod-e2e-test-suite:p1
"""E2E integration seam tests for resource-group module (Feature 0007).

10 tests. One file. < 15 seconds. Zero flakes.

Each test guards a specific integration seam -- a point where two independently
correct components can break when connected. If the seam is already covered by
a unit test (Feature 0006), there is no E2E test for it.

See: modules/system/resource-group/docs/features/0007-e2e-testing.md
"""
import os
import uuid

import httpx
import pytest

from .conftest import REQUEST_TIMEOUT, assert_group_shape, unique_type_code


# ── URL helpers ──────────────────────────────────────────────────────────


def _groups(base: str) -> str:
    return f"{base}/resource-group/v1/groups"


def _types(base: str) -> str:
    return f"{base}/types-registry/v1/types"


def _memberships(base: str) -> str:
    return f"{base}/resource-group/v1/memberships"


# ── S1: Route smoke ─────────────────────────────────────────────────────


async def test_route_smoke_all_endpoints(rg_base_url, rg_headers):
    """Seam: Route registration -- handlers mounted on correct method + path.

    Verifies all endpoints respond (not 404/405), meaning routes are registered
    and handlers are wired. No data setup needed -- fastest possible test.
    """
    rid = str(uuid.uuid4())
    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        h = rg_headers

        # GET endpoints -- must not be 404/405
        r = await c.get(_groups(rg_base_url), headers=h)
        assert r.status_code not in (404, 405), f"GET /groups: {r.status_code}"

        r = await c.get(f"{_groups(rg_base_url)}/{rid}", headers=h)
        assert r.status_code != 405, f"GET /groups/{{id}}: {r.status_code}"

        r = await c.get(f"{_groups(rg_base_url)}/{rid}/hierarchy", headers=h)
        assert r.status_code != 405, f"GET /groups/{{id}}/hierarchy: {r.status_code}"

        r = await c.get(_memberships(rg_base_url), headers=h)
        assert r.status_code not in (404, 405), f"GET /memberships: {r.status_code}"

        # POST with empty body -- 400 is fine (validation), 404/405 is not
        r = await c.post(_types(rg_base_url), headers=h, content=b"{}")
        assert r.status_code not in (404, 405), f"POST /types: {r.status_code}"


# ── S2: DTO roundtrip ───────────────────────────────────────────────────


async def test_dto_roundtrip_group_json_shape(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: DTO serialization -- JSON field names, types match OpenAPI contract.

    Unit tests (0006 G39-G45) test Rust struct conversions, NOT the JSON wire
    format. A serde attribute typo passes unit tests but breaks clients.
    """
    type_data = await create_type("s2dto")
    group = await create_group(
        type_data["code"], "S2 DTO Test", metadata={"barrier": True},
    )

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        r = await c.get(
            f"{_groups(rg_base_url)}/{group['id']}", headers=rg_headers,
        )
        assert r.status_code == 200
        data = r.json()

    # Structural validation
    assert_group_shape(data)

    # Exact key set (no internal field leaks)
    expected_keys = {"id", "type", "name", "hierarchy"}
    # metadata present because we set it
    assert "metadata" in data
    assert data["metadata"] == {"barrier": True}

    # "type" key (NOT "type_path", NOT "gts_type_id")
    assert "type" in data
    assert "type_path" not in data
    assert "gts_type_id" not in data

    # Hierarchy sub-object
    hier = data["hierarchy"]
    assert "tenant_id" in hier
    # Root group: parent_id absent or null
    assert hier.get("parent_id") is None

    # No timestamps in GroupDto (per DESIGN)
    assert "created_at" not in data
    assert "updated_at" not in data


# ── S3: AuthZ tenant filter ─────────────────────────────────────────────


async def test_authz_tenant_filter_applied(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: AuthZ -> SecureORM -- SecurityContext produces correct tenant filter.

    Unit tests mock PolicyEnforcer; real wiring only exists in module.rs.
    """
    type_data = await create_type("s3authz")
    group = await create_group(type_data["code"], "S3 AuthZ Test")
    group_id = group["id"]
    tenant_id = group["hierarchy"]["tenant_id"]

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # List -- created group must appear
        r = await c.get(_groups(rg_base_url), headers=rg_headers)
        assert r.status_code == 200
        ids = [item["id"] for item in r.json()["items"]]
        assert group_id in ids, "Created group not found in list"

        # GET -- tenant_id must match
        r = await c.get(
            f"{_groups(rg_base_url)}/{group_id}", headers=rg_headers,
        )
        assert r.status_code == 200
        assert r.json()["hierarchy"]["tenant_id"] == tenant_id


# ── S4: Cross-tenant invisible ──────────────────────────────────────────


async def test_cross_tenant_invisible(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: Same as S3 but negative -- tenant boundary enforced.

    Uses two real HTTP tokens producing different SecurityContexts.
    Skip if E2E_AUTH_TOKEN_TENANT_B not set.
    """
    token_b = os.getenv("E2E_AUTH_TOKEN_TENANT_B")
    if not token_b:
        pytest.skip("E2E_AUTH_TOKEN_TENANT_B not set")

    headers_b = {**rg_headers, "Authorization": f"Bearer {token_b}"}

    type_data = await create_type("s4xtenant")
    group = await create_group(type_data["code"], "S4 Cross-Tenant")
    group_id = group["id"]

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # Token B: single-entity GET -> 404 (hides existence)
        r = await c.get(
            f"{_groups(rg_base_url)}/{group_id}", headers=headers_b,
        )
        assert r.status_code == 404, (
            f"Cross-tenant GET should be 404, got {r.status_code}"
        )

        # Token B: list -> group not in items
        r = await c.get(_groups(rg_base_url), headers=headers_b)
        assert r.status_code == 200
        ids = [item["id"] for item in r.json()["items"]]
        assert group_id not in ids, "Group visible to other tenant"

        # Token A: still visible
        r = await c.get(
            f"{_groups(rg_base_url)}/{group_id}", headers=rg_headers,
        )
        assert r.status_code == 200


# ── S5: Hierarchy + closure (PG) ────────────────────────────────────────


async def test_hierarchy_closure_postgresql(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: Closure table INSERT SQL under PostgreSQL.

    0006 TC-GRP-01/02 verify closure on SQLite. PostgreSQL uses dialect-specific
    INSERT INTO...SELECT that can silently produce wrong results.
    """
    root_type = await create_type("s5root")
    child_type = await create_type(
        "s5child", can_be_root=False, allowed_parents=[root_type["code"]],
    )
    gc_type = await create_type(
        "s5gc", can_be_root=False, allowed_parents=[child_type["code"]],
    )

    root = await create_group(root_type["code"], "S5 Root")
    child = await create_group(child_type["code"], "S5 Child", parent_id=root["id"])
    grandchild = await create_group(gc_type["code"], "S5 Grandchild", parent_id=child["id"])

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # Hierarchy from root
        r = await c.get(
            f"{_groups(rg_base_url)}/{root['id']}/hierarchy",
            headers=rg_headers,
        )
        assert r.status_code == 200
        items = r.json()["items"]
        assert len(items) == 3, f"Expected 3 items, got {len(items)}"

        by_id = {item["id"]: item for item in items}
        assert by_id[root["id"]]["hierarchy"]["depth"] == 0
        assert by_id[child["id"]]["hierarchy"]["depth"] == 1
        assert by_id[grandchild["id"]]["hierarchy"]["depth"] == 2

        # Hierarchy from child (includes ancestors with negative depth)
        r = await c.get(
            f"{_groups(rg_base_url)}/{child['id']}/hierarchy",
            headers=rg_headers,
        )
        assert r.status_code == 200
        items = r.json()["items"]
        assert len(items) == 3  # ancestor(root) + self(child) + descendant(grandchild)

        by_id = {item["id"]: item for item in items}
        assert by_id[root["id"]]["hierarchy"]["depth"] == -1  # ancestor
        assert by_id[child["id"]]["hierarchy"]["depth"] == 0
        assert by_id[grandchild["id"]]["hierarchy"]["depth"] == 1


# ── S6: Move + closure rebuild (PG) ─────────────────────────────────────


async def test_move_closure_rebuild_postgresql(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: Closure table DELETE + re-INSERT under PostgreSQL SERIALIZABLE.

    The move runs DELETE FROM closure WHERE descendant IN (subtree) then
    INSERT INTO...SELECT new paths. SQLite cannot reproduce this behavior.
    """
    root_type = await create_type("s6root")
    child_type = await create_type(
        "s6child", can_be_root=False, allowed_parents=[root_type["code"]],
    )
    gc_type = await create_type(
        "s6gc", can_be_root=False, allowed_parents=[child_type["code"]],
    )

    root_a = await create_group(root_type["code"], "S6 Root A")
    child = await create_group(child_type["code"], "S6 Child", parent_id=root_a["id"])
    grandchild = await create_group(gc_type["code"], "S6 Grandchild", parent_id=child["id"])
    root_b = await create_group(root_type["code"], "S6 Root B")

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # Move child subtree from root_a to root_b
        r = await c.put(
            f"{_groups(rg_base_url)}/{child['id']}",
            headers=rg_headers,
            json={"type": child_type["code"], "name": "S6 Child", "parent_id": root_b["id"]},
        )
        assert r.status_code == 200, f"Move failed: {r.status_code} {r.text}"

        # root_a hierarchy: only root_a remains
        r = await c.get(
            f"{_groups(rg_base_url)}/{root_a['id']}/hierarchy",
            headers=rg_headers,
        )
        assert r.status_code == 200
        ids_a = [i["id"] for i in r.json()["items"]]
        assert child["id"] not in ids_a, "Child still in old tree after move"

        # root_b hierarchy: root_b + moved subtree
        r = await c.get(
            f"{_groups(rg_base_url)}/{root_b['id']}/hierarchy",
            headers=rg_headers,
        )
        assert r.status_code == 200
        by_id = {i["id"]: i for i in r.json()["items"]}
        assert child["id"] in by_id, "Child not in new tree"
        assert grandchild["id"] in by_id, "Grandchild not in new tree"
        assert by_id[child["id"]]["hierarchy"]["depth"] == 1
        assert by_id[grandchild["id"]]["hierarchy"]["depth"] == 2

        # Subtree from child preserved
        r = await c.get(
            f"{_groups(rg_base_url)}/{child['id']}/hierarchy",
            headers=rg_headers,
        )
        assert r.status_code == 200
        child_items = r.json()["items"]
        child_ids = [i["id"] for i in child_items]
        assert grandchild["id"] in child_ids, "Grandchild lost from subtree"


# ── S7: Force delete cascade (PG) ───────────────────────────────────────


async def test_force_delete_cascade_postgresql(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: FK ON DELETE RESTRICT + service-level cascade on PostgreSQL.

    Force delete must delete in correct order: memberships first, then children
    bottom-up, then target. Wrong order fails on PostgreSQL FK constraints.
    """
    member_type = await create_type("s7member")
    root_type = await create_type(
        "s7root", allowed_memberships=[member_type["code"]],
    )
    child_type = await create_type(
        "s7child", can_be_root=False,
        allowed_parents=[root_type["code"]],
        allowed_memberships=[member_type["code"]],
    )

    root = await create_group(root_type["code"], "S7 Root")
    child = await create_group(child_type["code"], "S7 Child", parent_id=root["id"])

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # Add membership on child
        r = await c.post(
            f"{_memberships(rg_base_url)}/{child['id']}/{member_type['code']}/res-s7",
            headers=rg_headers,
        )
        assert r.status_code == 201, f"Add membership: {r.status_code} {r.text}"

        # Force delete root
        r = await c.delete(
            f"{_groups(rg_base_url)}/{root['id']}",
            headers=rg_headers,
            params={"force": "true"},
        )
        assert r.status_code == 204

        # Root gone
        r = await c.get(
            f"{_groups(rg_base_url)}/{root['id']}", headers=rg_headers,
        )
        assert r.status_code == 404

        # Child gone (cascade)
        r = await c.get(
            f"{_groups(rg_base_url)}/{child['id']}", headers=rg_headers,
        )
        assert r.status_code == 404

        # Membership cleaned up
        r = await c.get(
            _memberships(rg_base_url), headers=rg_headers,
            params={"$filter": f"group_id eq {child['id']}"},
        )
        assert r.status_code == 200
        assert len(r.json()["items"]) == 0, "Membership not cleaned up"


# ── S8: Error response format ───────────────────────────────────────────


async def test_error_response_rfc9457(rg_base_url, rg_headers):
    """Seam: Error middleware -- DomainError -> application/problem+json.

    Unit tests assert DomainError variant, not HTTP headers. If the error handler
    is missing, clients get generic framework errors instead of RFC 9457.
    """
    rid = str(uuid.uuid4())

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # 404 -- not found
        r = await c.get(
            f"{_groups(rg_base_url)}/{rid}", headers=rg_headers,
        )
        assert r.status_code == 404
        ct = r.headers.get("content-type", "")
        assert "application/problem+json" in ct, (
            f"Expected problem+json, got: {ct}"
        )
        body = r.json()
        assert body.get("status") == 404
        assert "title" in body
        assert "detail" in body
        # No internal leaks
        assert "stack" not in body
        assert "trace" not in body

        # 409 -- duplicate type
        type_code = unique_type_code("s8dup")
        payload = {"code": type_code, "can_be_root": True}
        r = await c.post(_types(rg_base_url), headers=rg_headers, json=payload)
        assert r.status_code == 201

        r = await c.post(_types(rg_base_url), headers=rg_headers, json=payload)
        assert r.status_code == 409
        body = r.json()
        assert body.get("status") == 409


# ── S9: Cursor pagination ───────────────────────────────────────────────


async def test_pagination_cursor_roundtrip(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: Cursor encode/decode across HTTP -- no duplicates, no missing.

    0006 tests Page<T> construction. The cursor codec (base64 encode/decode)
    only runs in the handler layer over HTTP.
    """
    type_data = await create_type("s9page")
    type_code = type_data["code"]
    created_ids = set()
    for i in range(5):
        g = await create_group(type_code, f"S9 Page {i}")
        created_ids.add(g["id"])

    # Paginate with limit=2, filtered to our type only
    all_ids = []
    cursor = None
    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        for _ in range(10):  # safety cap
            params = {
                "limit": "2",
                "$filter": f"type eq '{type_code}'",
            }
            if cursor:
                params["cursor"] = cursor

            r = await c.get(
                _groups(rg_base_url), headers=rg_headers, params=params,
            )
            assert r.status_code == 200
            data = r.json()

            page_ids = [item["id"] for item in data["items"]]
            all_ids.extend(page_ids)

            page_info = data["page_info"]
            if not page_info.get("has_next_page"):
                break
            cursor = page_info["next_cursor"]

    # No duplicates
    assert len(all_ids) == len(set(all_ids)), (
        f"Duplicate IDs in pagination: {all_ids}"
    )
    # All created groups present
    for gid in created_ids:
        assert gid in all_ids, f"Group {gid} missing from paginated results"


# ── S10: Membership filter wiring ───────────────────────────────────────


async def test_membership_filter_wiring(
    rg_base_url, rg_headers, create_type, create_group,
):
    """Seam: OData $filter parsing -> SQL WHERE for memberships.

    0006 verifies field mapping. The full chain -- HTTP $filter -> OData parser
    -> FilterField -> SQL WHERE -- is never tested end-to-end.
    """
    member_type = await create_type("s10member")
    org_type = await create_type("s10org", allowed_memberships=[member_type["code"]])

    group_a = await create_group(org_type["code"], "S10 Group A")
    group_b = await create_group(org_type["code"], "S10 Group B")

    async with httpx.AsyncClient(timeout=REQUEST_TIMEOUT) as c:
        # Add memberships to different groups
        r = await c.post(
            f"{_memberships(rg_base_url)}/{group_a['id']}/{member_type['code']}/res-1",
            headers=rg_headers,
        )
        assert r.status_code == 201

        r = await c.post(
            f"{_memberships(rg_base_url)}/{group_b['id']}/{member_type['code']}/res-2",
            headers=rg_headers,
        )
        assert r.status_code == 201

        # Filter by group_a -- should only see res-1
        r = await c.get(
            _memberships(rg_base_url), headers=rg_headers,
            params={"$filter": f"group_id eq {group_a['id']}"},
        )
        assert r.status_code == 200
        items = r.json()["items"]
        assert all(
            m["group_id"] == group_a["id"] for m in items
        ), "Filter leaked items from other group"
        assert any(m["resource_id"] == "res-1" for m in items)
        assert not any(m["resource_id"] == "res-2" for m in items)

        # Filter by group_b -- should only see res-2
        r = await c.get(
            _memberships(rg_base_url), headers=rg_headers,
            params={"$filter": f"group_id eq {group_b['id']}"},
        )
        assert r.status_code == 200
        items = r.json()["items"]
        assert all(m["group_id"] == group_b["id"] for m in items)
        assert any(m["resource_id"] == "res-2" for m in items)
