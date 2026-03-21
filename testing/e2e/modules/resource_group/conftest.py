"""Pytest configuration and fixtures for resource-group E2E tests."""
import os
import uuid
import time

import httpx
import pytest


# ── Environment-driven fixtures ──────────────────────────────────────────

@pytest.fixture
def rg_base_url():
    """Resource-group service base URL."""
    return os.getenv("E2E_BASE_URL", "http://localhost:8087")


@pytest.fixture
def rg_headers():
    """Standard headers with auth token for resource-group requests."""
    headers = {"Content-Type": "application/json"}
    token = os.getenv("E2E_AUTH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


# ── Reachability check ───────────────────────────────────────────────────

@pytest.fixture(scope="session", autouse=True)
def _check_rg_reachable():
    """Skip all resource-group tests if the service is not reachable."""
    url = os.getenv("E2E_BASE_URL", "http://localhost:8087")
    try:
        resp = httpx.get(
            f"{url}/cf/resource-group/v1/groups",
            timeout=5.0,
            headers={"Authorization": "Bearer test"},
        )
        # Any response (even 401/403) means the service is up.
    except httpx.ConnectError:
        pytest.skip(
            f"Resource-group service not running at {url}",
            allow_module_level=True,
        )
    except Exception:
        pass


# ── Test data helpers ────────────────────────────────────────────────────

_counter = int(time.time() * 1000) % 1000000


def unique_type_code(name: str) -> str:
    """Generate a unique RG type code to avoid collisions between test runs."""
    global _counter
    _counter += 1
    return f"gts.x.system.rg.type.v1~x.e2etest.{name}{_counter}.v1~"


@pytest.fixture
def create_type(rg_base_url, rg_headers):
    """Factory fixture: create a GTS type and return its code."""
    created_codes = []

    async def _create(name: str, can_be_root: bool = True, allowed_parents=None, allowed_memberships=None):
        code = unique_type_code(name)
        payload = {
            "code": code,
            "can_be_root": can_be_root,
            "allowed_parents": allowed_parents or [],
            "allowed_memberships": allowed_memberships or [],
        }
        async with httpx.AsyncClient(timeout=10.0) as client:
            resp = await client.post(
                f"{rg_base_url}/cf/resource-group/v1/types",
                headers=rg_headers,
                json=payload,
            )
            assert resp.status_code == 201, (
                f"Failed to create type '{code}': {resp.status_code} {resp.text}"
            )
            created_codes.append(code)
            return resp.json()

    return _create


@pytest.fixture
def create_group(rg_base_url, rg_headers):
    """Factory fixture: create a resource group and return its data."""
    created_ids = []

    async def _create(type_code: str, name: str, parent_id: str = None):
        payload = {
            "type": type_code,
            "name": name,
        }
        if parent_id:
            payload["parent_id"] = parent_id

        async with httpx.AsyncClient(timeout=10.0) as client:
            resp = await client.post(
                f"{rg_base_url}/cf/resource-group/v1/groups",
                headers=rg_headers,
                json=payload,
            )
            assert resp.status_code == 201, (
                f"Failed to create group '{name}': {resp.status_code} {resp.text}"
            )
            data = resp.json()
            created_ids.append(data["id"])
            return data

    return _create
