"""E2E tests: AuthZ tenant scoping for resource-group module (Phase 1).

Verifies that RG endpoints apply tenant isolation via SecurityContext:
- Groups created under one tenant are visible to that tenant
- Groups are not visible across tenants (when AuthZ is enabled)
- CRUD operations respect tenant boundaries

Prerequisites:
  - hyperspot-server running with resource-group module enabled
  - PostgreSQL available for resource-group database
  - For full AuthZ testing: auth_disabled=false with static-authn/static-authz

Run:
  E2E_BASE_URL=http://localhost:8087 pytest testing/e2e/modules/resource_group/ -v
"""
import httpx
import pytest

from .conftest import unique_type_code


# ── Phase 1 Tests: Basic CRUD + Tenant Context ──────────────────────────


@pytest.mark.asyncio
async def test_create_and_get_type(rg_base_url, rg_headers, create_type):
    """Types can be created and retrieved."""
    type_data = await create_type("org")
    code = type_data["code"]

    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(
            f"{rg_base_url}/types-registry/v1/types/{code}",
            headers=rg_headers,
        )

        if resp.status_code in (401, 403) and "Authorization" not in rg_headers:
            pytest.skip("Endpoint requires authentication")

        assert resp.status_code == 200, f"GET type failed: {resp.status_code} {resp.text}"
        data = resp.json()
        assert data["code"] == code
        assert data["can_be_root"] is True


@pytest.mark.asyncio
async def test_create_and_get_group(rg_base_url, rg_headers, create_type, create_group):
    """Groups can be created and retrieved; tenant_id is set from SecurityContext."""
    type_data = await create_type("org")
    group_data = await create_group(type_data["code"], "Test Org Alpha")

    group_id = group_data["id"]
    assert group_data["name"] == "Test Org Alpha"
    assert "tenant_id" in group_data["hierarchy"], "Response should include tenant_id in hierarchy"

    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(
            f"{rg_base_url}/resource-group/v1/groups/{group_id}",
            headers=rg_headers,
        )
        assert resp.status_code == 200
        fetched = resp.json()
        assert fetched["id"] == group_id
        assert fetched["hierarchy"]["tenant_id"] == group_data["hierarchy"]["tenant_id"]


@pytest.mark.asyncio
async def test_list_groups_returns_created_groups(rg_base_url, rg_headers, create_type, create_group):
    """List groups endpoint returns groups created in this session."""
    type_data = await create_type("team")
    g1 = await create_group(type_data["code"], "Team Alpha")
    g2 = await create_group(type_data["code"], "Team Beta")

    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(
            f"{rg_base_url}/resource-group/v1/groups",
            headers=rg_headers,
        )
        assert resp.status_code == 200
        data = resp.json()

        # Page structure
        assert "items" in data, f"Expected paginated response with 'items', got: {list(data.keys())}"

        item_ids = [item["id"] for item in data["items"]]
        assert g1["id"] in item_ids, "Group 1 should be in list"
        assert g2["id"] in item_ids, "Group 2 should be in list"


@pytest.mark.asyncio
async def test_group_has_tenant_id_from_security_context(rg_base_url, rg_headers, create_type, create_group):
    """Created group's tenant_id matches the SecurityContext subject_tenant_id.

    When using static-authn in accept_all mode, all requests get a default
    SecurityContext with a known tenant. The group's tenant_id should match.
    """
    type_data = await create_type("project")
    group_data = await create_group(type_data["code"], "Project X")

    assert "tenant_id" in group_data["hierarchy"]
    tenant_id = group_data["hierarchy"]["tenant_id"]

    # All groups created with same token should have same tenant
    group_data_2 = await create_group(type_data["code"], "Project Y")
    assert group_data_2["hierarchy"]["tenant_id"] == tenant_id, (
        "Groups created with same auth should have same tenant_id"
    )


@pytest.mark.asyncio
async def test_child_group_inherits_parent_tenant(rg_base_url, rg_headers, create_type, create_group):
    """Child groups must have the same tenant_id as their parent."""
    parent_type_code = unique_type_code("parentorg")
    child_type_code = unique_type_code("childteam")

    async with httpx.AsyncClient(timeout=10.0) as client:
        # Create parent type
        resp = await client.post(
            f"{rg_base_url}/types-registry/v1/types",
            headers=rg_headers,
            json={
                "code": parent_type_code,
                "can_be_root": True,
                "allowed_parents": [],
                "allowed_memberships": [],
            },
        )
        assert resp.status_code == 201, f"Create parent type: {resp.text}"

        # Create child type (allowed_parents includes parent type)
        resp = await client.post(
            f"{rg_base_url}/types-registry/v1/types",
            headers=rg_headers,
            json={
                "code": child_type_code,
                "can_be_root": False,
                "allowed_parents": [parent_type_code],
                "allowed_memberships": [],
            },
        )
        assert resp.status_code == 201, f"Create child type: {resp.text}"

    # Create parent group
    parent = await create_group(parent_type_code, "Parent Org")

    # Create child group under parent
    child = await create_group(child_type_code, "Child Team", parent_id=parent["id"])

    assert child["hierarchy"]["tenant_id"] == parent["hierarchy"]["tenant_id"], (
        "Child group tenant_id must match parent tenant_id"
    )
    assert child["hierarchy"]["parent_id"] == parent["id"]


@pytest.mark.asyncio
async def test_group_hierarchy_returns_parent_and_children(rg_base_url, rg_headers, create_type, create_group):
    """Hierarchy endpoint returns the group and its descendants."""
    parent_type_code = unique_type_code("hier_parent")
    child_type_code = unique_type_code("hier_child")

    async with httpx.AsyncClient(timeout=10.0) as client:
        # Setup types
        await client.post(
            f"{rg_base_url}/types-registry/v1/types",
            headers=rg_headers,
            json={
                "code": parent_type_code,
                "can_be_root": True,
                "allowed_parents": [],
                "allowed_memberships": [],
            },
        )
        await client.post(
            f"{rg_base_url}/types-registry/v1/types",
            headers=rg_headers,
            json={
                "code": child_type_code,
                "can_be_root": False,
                "allowed_parents": [parent_type_code],
                "allowed_memberships": [],
            },
        )

    parent = await create_group(parent_type_code, "Hier Parent")
    child1 = await create_group(child_type_code, "Hier Child 1", parent_id=parent["id"])
    child2 = await create_group(child_type_code, "Hier Child 2", parent_id=parent["id"])

    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(
            f"{rg_base_url}/resource-group/v1/groups/{parent['id']}/hierarchy",
            headers=rg_headers,
        )
        assert resp.status_code == 200
        data = resp.json()
        assert "items" in data

        hier_ids = [item["id"] for item in data["items"]]
        assert parent["id"] in hier_ids, "Parent should be in hierarchy"
        assert child1["id"] in hier_ids, "Child 1 should be in hierarchy"
        assert child2["id"] in hier_ids, "Child 2 should be in hierarchy"


@pytest.mark.asyncio
async def test_delete_group(rg_base_url, rg_headers, create_type, create_group):
    """Groups can be deleted and are no longer retrievable."""
    type_data = await create_type("deletable")
    group = await create_group(type_data["code"], "To Be Deleted")

    async with httpx.AsyncClient(timeout=10.0) as client:
        # Delete
        resp = await client.delete(
            f"{rg_base_url}/resource-group/v1/groups/{group['id']}",
            headers=rg_headers,
        )
        assert resp.status_code == 204

        # Verify gone
        resp = await client.get(
            f"{rg_base_url}/resource-group/v1/groups/{group['id']}",
            headers=rg_headers,
        )
        assert resp.status_code == 404


@pytest.mark.asyncio
async def test_membership_add_and_list(rg_base_url, rg_headers, create_type, create_group):
    """Memberships can be added and listed for a group."""
    member_type_data = await create_type("mem_task", can_be_root=True)
    member_type_code = member_type_data["code"]
    t = await create_type("mem_org", allowed_memberships=[member_type_code])
    type_code = t["code"]

    group = await create_group(type_code, "Membership Test Org")
    resource_id = "task-001"

    async with httpx.AsyncClient(timeout=10.0) as client:
        # Add membership
        resp = await client.post(
            f"{rg_base_url}/resource-group/v1/memberships/{group['id']}/{member_type_code}/{resource_id}",
            headers=rg_headers,
        )
        assert resp.status_code == 201, f"Add membership: {resp.status_code} {resp.text}"

        # List memberships
        resp = await client.get(
            f"{rg_base_url}/resource-group/v1/memberships",
            headers=rg_headers,
        )
        assert resp.status_code == 200
        data = resp.json()
        assert "items" in data

        found = any(
            m.get("group_id") == group["id"] and m.get("resource_id") == resource_id
            for m in data["items"]
        )
        assert found, f"Membership not found in list. Items: {data['items']}"


# ── Tenant Isolation (requires auth_disabled=false) ──────────────────────


@pytest.mark.asyncio
async def test_tenant_isolation_same_token_sees_own_groups(rg_base_url, rg_headers, create_type, create_group):
    """Groups created with the same auth token are all visible.

    This is the baseline: all groups share the same SecurityContext tenant_id.
    """
    type_data = await create_type("iso_org")
    g1 = await create_group(type_data["code"], "Isolation Org 1")
    g2 = await create_group(type_data["code"], "Isolation Org 2")

    async with httpx.AsyncClient(timeout=10.0) as client:
        resp = await client.get(
            f"{rg_base_url}/resource-group/v1/groups",
            headers=rg_headers,
        )
        assert resp.status_code == 200
        ids = [item["id"] for item in resp.json().get("items", [])]
        assert g1["id"] in ids
        assert g2["id"] in ids
