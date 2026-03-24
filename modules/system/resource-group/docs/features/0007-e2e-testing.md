# Feature: E2E Test Plan for Resource Group Module

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-e2e-testing`

- [x] `p1` - `cpt-cf-resource-group-feature-e2e-testing`

## Feature Context

### Overview

End-to-end test plan for the resource-group module. Covers 10 integration seam tests using real PostgreSQL, HTTP, and AuthN/AuthZ pipeline.

### Purpose

Verify integration seams that unit tests (Feature 0006) cannot cover: HTTP routing, PostgreSQL-specific behavior, real AuthN/AuthZ, MTLS, and cursor codec over HTTP.

**Requirements**: Features 0001-0006

### Actors

| Actor | Role in Feature |
|-------|-----------------|
| Developer | Runs E2E tests, maintains test infrastructure |

### References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **Dependencies**: Features 0001-0006

## Actor Flows (CDSL)

No actor flows — this is a test specification feature.

## Processes / Business Logic (CDSL)

No processes — test logic is defined in test plan sections below.

---

## TL;DR

10 tests. One file. < 15 seconds. Zero flakes.

Each test exists because it guards a specific **integration seam** — a point where two independently correct components can break when connected. If the seam is already covered by a unit test (Feature 0006), there is no E2E test for it.

---

## Philosophy

### What Is an E2E Test in This Project

An E2E test is an HTTP request to a **running** `hyperspot-server` with a **real PostgreSQL** database, traversing the full chain: TCP → HTTP router → AuthN middleware → AuthZ PolicyEnforcer → Service → Repository → PostgreSQL → Response serialization → HTTP response. Pytest sends requests from the outside, exactly like a real client.

This is not "another way to verify business logic". Business logic is already verified in Feature 0006 (88 unit/integration tests on SQLite in-memory). E2E tests verify the **seams between components**, not the components themselves.

### Three Questions Before Adding a Test

Every E2E test in this plan has been validated against three questions:

1. **"Can this bug only manifest during real HTTP interaction?"**
   If yes — it's an E2E test. If the bug is catchable by calling a Rust function directly — it's a unit test and does not belong here.

2. **"Can this bug only manifest on PostgreSQL but not on SQLite?"**
   FK constraints, SERIALIZABLE transactions, `gts_type_path` DOMAIN, JSONB — all behave differently on SQLite. If the bug depends on the DB dialect — it's an E2E test.

3. **"If we remove this test, does integration confidence decrease?"**
   If not — the test is unnecessary. A test that duplicates unit coverage adds no confidence — it adds execution time and flake surface.

### What We Do NOT Test via E2E

- GTS type path validation (0006: 12 unit tests on `GtsTypePath`)
- Cycle detection (0006: TC-GRP-06/07 with real closure table on SQLite)
- Placement invariant `can_be_root OR allowed_parents` (0006: TC-TYP-04)
- Query profile limits `max_depth/max_width` (0006: TC-GRP-17/18/19)
- All 11 `DomainError` variants (0006: 41 tests in `domain_unit_test.rs`)
- DTO struct↔struct conversions (0006: G39-G45)
- Seeding idempotency (0006: TC-SEED-01–06)
- AccessScope construction (0006: `tenant_scoping_test.rs`, 10 tests)

All of the above is **deterministic logic** that works identically whether called via HTTP or called directly. Running it through HTTP does not increase coverage — it increases execution time and brittleness.

### What We Test (7 Integration Seams)

| Seam | What breaks between components | Why unit tests are blind |
|------|-------------------------------|-------------------------|
| **Router ↔ Handler** | Route not registered, wrong method/path | Unit tests call the service directly, router never participates |
| **Handler ↔ JSON wire** | `#[serde(rename)]` typo, missing field, camelCase mismatch | Unit tests operate on Rust structs, not JSON bytes |
| **Module init ↔ AuthZ** | `PolicyEnforcer` not created, `AccessScope` not passed to repo | Unit tests mock PolicyEnforcer; real wiring only exists in `module.rs` |
| **Service ↔ PostgreSQL** | FK enforcement, SERIALIZABLE isolation, `gts_type_path` DOMAIN | 0006 runs on SQLite — different FK behavior, no domain types |
| **Closure SQL ↔ PostgreSQL** | `INSERT INTO ... SELECT` rows, `DELETE + re-INSERT` on move | SQLite does not reproduce concurrency or dialect-specific SQL |
| **Error handler ↔ HTTP** | `Content-Type: application/problem+json` not set, stack trace leaked | Unit tests assert `DomainError` variant, not HTTP headers |
| **Cursor codec ↔ HTTP** | Base64 encode/decode roundtrip, URL-encoding, offset drift | Unit tests test `Page<T>`; the codec only runs in the handler layer |

### Reliability Principles

> *"A smaller set of reliable E2E tests is better than a large set of flaky tests that everyone ignores."*
> — [Bunnyshell, E2E Testing for Microservices (2026)](https://www.bunnyshell.com/blog/end-to-end-testing-for-microservices-a-2025-guide/)

> *"Think about the properties you'd like from your test suite using the SMURF mnemonic: Speed, Maintainability, Utilization, Reliability, Fidelity."*
> — [Google Testing Blog, SMURF: Beyond the Test Pyramid (2024)](https://testing.googleblog.com/2024/10/smurf-beyond-test-pyramid.html)

**Speed** — 10 tests, < 15 seconds. One file. No fan-out across 8 files.

**Maintainability** — each test is tied to a specific seam. When the seam changes (e.g., migrating from Actix to Axum), one test breaks, not twenty.

**Reliability** — `pytest-timeout=10s` per test. `httpx timeout=5s` per request. Zero `time.sleep()`. Self-contained data: each test creates its own types and groups via factory fixtures, never depends on another test's data.

**Fidelity** — real PostgreSQL, real AuthZ pipeline, real HTTP. This is the only thing that justifies E2E on top of 88 unit tests.

**Utilization** — every test is unique. None duplicates 0006 coverage. A test that can be removed without losing integration confidence should not exist.

---

<!-- toc -->

- [Feature Context](#feature-context)
  - [Overview](#overview)
  - [Purpose](#purpose)
  - [Actors](#actors)
  - [References](#references)
- [Actor Flows (CDSL)](#actor-flows-cdsl)
- [Processes / Business Logic (CDSL)](#processes--business-logic-cdsl)
- [TL;DR](#tldr)
- [Philosophy](#philosophy)
  - [What Is an E2E Test in This Project](#what-is-an-e2e-test-in-this-project)
  - [Three Questions Before Adding a Test](#three-questions-before-adding-a-test)
  - [What We Do NOT Test via E2E](#what-we-do-not-test-via-e2e)
  - [What We Test (7 Integration Seams)](#what-we-test-7-integration-seams)
  - [Reliability Principles](#reliability-principles)
- [1. Integration Seams Map](#1-integration-seams-map)
- [2. Test Infrastructure](#2-test-infrastructure)
  - [File Layout](#file-layout)
  - [Dependencies](#dependencies)
  - [pytest Configuration](#pytest-configuration)
  - [Reliability Rules](#reliability-rules)
  - [Shared Helpers (add to conftest.py)](#shared-helpers-add-to-conftestpy)
- [3. Core Test Suite (10 tests)](#3-core-test-suite-10-tests)
  - [S1: `test_route_smoke_all_endpoints`](#s1-testroutesmokeallendpoints)
  - [S2: `test_dto_roundtrip_group_json_shape`](#s2-testdtoroundtripgroupjsonshape)
  - [S3: `test_authz_tenant_filter_applied`](#s3-testauthztenantfilterapplied)
  - [S4: `test_cross_tenant_invisible`](#s4-testcrosstenantinvisible)
  - [S5: `test_hierarchy_closure_postgresql`](#s5-testhierarchyclosurepostgresql)
  - [S6: `test_move_closure_rebuild_postgresql`](#s6-testmoveclosurerebuildpostgresql)
  - [S7: `test_force_delete_cascade_postgresql`](#s7-testforcedeletecascadepostgresql)
  - [S8: `test_error_response_rfc9457`](#s8-testerrorresponserfc9457)
  - [S9: `test_pagination_cursor_roundtrip`](#s9-testpaginationcursorroundtrip)
  - [S10: `test_membership_filter_wiring`](#s10-testmembershipfilterwiring)
- [4. Optional Suite](#4-optional-suite)
- [5. Anti-Patterns (do NOT test here)](#5-anti-patterns-do-not-test-here)
- [Definitions of Done](#definitions-of-done)
  - [E2E Test Suite Implementation](#e2e-test-suite-implementation)
- [6. Acceptance Criteria](#6-acceptance-criteria)

<!-- /toc -->

## 1. Integration Seams Map

Each test below targets exactly one seam. The "Seam" column explains why this test exists despite 0006 coverage.

| # | Test | Seam | 0006 overlap | What E2E adds |
|---|------|------|--------------|---------------|
| S1 | Route smoke | Route registration | None (0006 never touches HTTP) | All 11 endpoints respond non-405 |
| S2 | DTO roundtrip | Serialization | G39-G45 test DTO→struct; not JSON wire | Full JSON shape matches OpenAPI |
| S3 | AuthZ tenant filter | AuthZ → SecureORM | authz_integration_test mocks PDP | Real PolicyEnforcer + real DB + real WHERE |
| S4 | Cross-tenant isolation | AuthZ → SecureORM | tenant_filtering_db_test uses SQLite | Real PostgreSQL + real tokens + HTTP 404 |
| S5 | Hierarchy + closure (PG) | PostgreSQL closure SQL | TC-GRP-01/02 on SQLite | Real PG INSERT INTO...SELECT, correct depths |
| S6 | Move + closure rebuild (PG) | PostgreSQL closure SQL | TC-GRP-05 on SQLite | Real PG DELETE + re-INSERT, SERIALIZABLE |
| S7 | Force delete cascade (PG) | PostgreSQL FK cascade | TC-GRP-15 on SQLite | Real ON DELETE RESTRICT + service cascade |
| S8 | Error response format | Error middleware | domain_unit_test checks DomainError→Problem | HTTP status + Content-Type header + no leaks |
| S9 | Cursor pagination | Pagination codec | None (0006 tests page logic, not codec) | Encode→HTTP→decode roundtrip, no duplicates |
| S10 | Membership filter | OData filter wiring | G42-G45 test field mapping | Real $filter parse → SQL WHERE → correct subset |

---

## 2. Test Infrastructure

### File Layout

```
testing/e2e/modules/resource_group/
├── conftest.py                          ← Extended: helpers, timeout config
├── test_authz_tenant_scoping.py         ← Existing (9 tests) — keep as-is
├── test_mtls_auth.py                    ← Existing (4 tests) — keep as-is
├── test_integration_seams.py            ← NEW: 10 tests
```

### Dependencies

```
httpx>=0.27
pytest>=8.0
pytest-asyncio>=0.24
pytest-timeout>=2.3        # prevents hanging async coroutines — #1 anti-flake measure
```

### pytest Configuration

```ini
# testing/e2e/pytest.ini (extend existing)
[pytest]
asyncio_mode = auto        # every async def test_* runs automatically, no marker needed
timeout = 10               # per-test hard timeout (seconds) via pytest-timeout
```

- `asyncio_mode = auto` — eliminates `@pytest.mark.asyncio` boilerplate on every test
- `timeout = 10` — if a test hangs >10s, it's broken, not slow. Fail fast instead of blocking CI.

### Reliability Rules

| Rule | Rationale |
|------|-----------|
| **Per-request timeout: 5s** | Every `httpx.AsyncClient(timeout=5.0)` call. Prevents one slow response from triggering the 10s test timeout with a confusing error. |
| **No `time.sleep()` anywhere** | Sleep-based waits are the #1 flakiness source. If you wait for state, poll with a short retry or restructure the test. |
| **No shared mutable state** | Each test creates its own types/groups. Never depends on another test's data. |
| **Session-scoped reachability check** | Existing `_check_rg_reachable` — skip entire module if server down, don't waste CI on 10 connection errors. |
| **Function-scoped factory fixtures** | `create_type`, `create_group` — unique names via timestamp counter, no cleanup between tests needed. |

> Sources: [Bunnyshell E2E Best Practices 2025](https://www.bunnyshell.com/blog/best-practices-for-end-to-end-testing-in-2025/), [pytest-timeout](https://pypi.org/project/pytest-timeout/), [async test patterns for pytest](https://tonybaloney.github.io/posts/async-test-patterns-for-pytest-and-unittest.html)

### Shared Helpers (add to conftest.py)

```python
REQUEST_TIMEOUT = 5.0  # every httpx call uses this

def assert_group_shape(data: dict):
    """Verify JSON wire format matches OpenAPI contract."""
    uuid.UUID(data["id"])
    uuid.UUID(data["tenant_id"])
    assert isinstance(data["name"], str)
    assert "created_at" in data
    datetime.fromisoformat(data["created_at"])
    if data.get("parent_id") is not None:
        uuid.UUID(data["parent_id"])
    if data.get("metadata") is not None:
        assert isinstance(data["metadata"], dict)
```

---

## 3. Core Test Suite (10 tests)

### S1: `test_route_smoke_all_endpoints`

**Seam**: Route registration — handlers mounted on correct method + path.

**Why not in 0006**: Unit tests call `TypeService::create()` / `GroupService::list()` directly. If a handler is not registered in `module.rs`, or mounted on wrong path, unit tests pass but the API is broken.

```
HEAD /cf/resource-group/v1/groups               → not 404/405
HEAD /cf/resource-group/v1/groups/{uuid}        → not 405 (404 ok — group doesn't exist)
HEAD /cf/resource-group/v1/groups/{uuid}/hierarchy  → not 405
HEAD /cf/resource-group/v1/memberships          → not 405
POST /cf/types-registry/v1/types (empty body)   → not 404/405 (400 ok — validation)

Verify: each returns a status code, meaning the route exists and the handler runs.
No data setup needed. Fastest possible test.
```

---

### S2: `test_dto_roundtrip_group_json_shape`

**Seam**: DTO serialization — JSON field names, types, presence match OpenAPI contract.

**Why not in 0006**: 0006 tests `From<Group> for GroupDto` (Rust struct conversion). It does NOT test the JSON wire format: `#[serde(rename = "type")]`, `#[serde(skip_serializing_if = "Option::is_none")]`, camelCase conventions, timestamp format. A serde attribute typo passes unit tests but breaks clients.

```
POST /types → create type
POST /groups → create group with metadata: {"barrier": true}
GET  /groups/{id} → 200

Assert JSON keys:
  "id"         — string, UUID format
  "name"       — string
  "type"       — string (NOT "type_path", NOT "gts_type_id")
  "tenant_id"  — string, UUID format
  "parent_id"  — null (root group)
  "depth"      — integer, == 0
  "metadata"   — {"barrier": true} (JSONB roundtrip)
  "created_at" — string, ISO 8601
  "updated_at" — null or absent (fresh create)

Assert NO unexpected keys leaking (like "gts_type_id" internal SMALLINT).
```

---

### S3: `test_authz_tenant_filter_applied`

**Seam**: AuthZ → SecureORM full chain — SecurityContext → PolicyEnforcer → AccessScope → `WHERE tenant_id IN (...)`.

**Why not in 0006**: `authz_integration_test.rs` mocks the PDP and checks that `access_scope()` returns correct scope. `tenant_filtering_db_test.rs` manually constructs AccessScope and passes it to the repo. Neither test verifies the **real wiring** in `module.rs` where PolicyEnforcer is created from ClientHub and injected into GroupService.

```
POST /groups {name: "AuthZ Test"}       → 201, note tenant_id from response
GET  /groups                            → 200
  assert created group appears in list   (tenant filter allows own groups)
GET  /groups/{id}                       → 200
  assert tenant_id matches              (single-entity fetch also scoped)
```

This is a positive-only test. It verifies the full pipeline produces correct results for the happy path. Cross-tenant negative testing is in S4.

---

### S4: `test_cross_tenant_invisible`

**Seam**: Same as S3, but negative — verifies tenant boundary is enforced, not just that own data is visible.

**Why not in 0006**: `tenant_filtering_db_test.rs` creates two AccessScopes manually. E2E uses two **real HTTP tokens** producing different SecurityContexts, exercising the full authn → authz → scope → SQL chain.

> **Skip if** `E2E_AUTH_TOKEN_TENANT_B` not set.

```
[Token A] POST /groups              → 201, group_id
[Token B] GET  /groups/{group_id}   → 404 (not 403 — hides existence)
[Token B] GET  /groups              → 200, group_id NOT in items
[Token A] GET  /groups/{group_id}   → 200 (still visible to owner)
```

---

### S5: `test_hierarchy_closure_postgresql`

**Seam**: Closure table INSERT SQL under PostgreSQL.

**Why not in 0006**: 0006 TC-GRP-01/02 verify closure rows on SQLite. PostgreSQL uses `INSERT INTO resource_group_closure SELECT ...` with joins against existing closure rows — this SQL can silently produce wrong results if column order or join condition is off, and SQLite won't catch it because its type system is looser.

```
POST parent_type (can_be_root), child_type (allowed_parents: [parent])
POST root → child → grandchild

GET /groups/{root.id}/hierarchy     → 200
  assert len(items) == 3
  assert root  depth == 0
  assert child depth == 1
  assert grandchild depth == 2

GET /groups/{child.id}/hierarchy    → 200
  assert len(items) == 2
  assert child depth == 0           (relative to query root)
  assert grandchild depth == 1
```

---

### S6: `test_move_closure_rebuild_postgresql`

**Seam**: Closure table DELETE + re-INSERT under PostgreSQL SERIALIZABLE transaction.

**Why not in 0006**: The move operation runs: `DELETE FROM resource_group_closure WHERE descendant_id IN (subtree)` then `INSERT INTO ... SELECT` new paths. On SQLite this runs without real SERIALIZABLE isolation. On PostgreSQL, a concurrent read could see inconsistent closure state if the transaction isn't properly isolated.

```
POST root_A → child → grandchild
POST root_B

PUT /groups/{child.id} {parent_id: root_B.id}    → 200

GET /groups/{root_A.id}/hierarchy    → items == [root_A]
  assert child NOT in items                        (detached from old tree)

GET /groups/{root_B.id}/hierarchy    → items == [root_B, child, grandchild]
  assert child depth == 1                          (recalculated)
  assert grandchild depth == 2                     (recalculated)

GET /groups/{child.id}/hierarchy     → items == [child, grandchild]
  assert grandchild in items                       (subtree preserved)
```

---

### S7: `test_force_delete_cascade_postgresql`

**Seam**: FK ON DELETE RESTRICT + service-level cascade on PostgreSQL.

**Why not in 0006**: `resource_group.parent_id` has `ON DELETE RESTRICT` and `resource_group_membership.group_id` has `ON DELETE RESTRICT`. Force delete must delete in correct order: memberships first, then children bottom-up, then target. On SQLite, FK enforcement is off by default (`PRAGMA foreign_keys = ON` needed). A wrong deletion order passes SQLite but fails PostgreSQL.

```
POST root → child (add membership on child)

DELETE /groups/{root.id}?force=true              → 204

GET /groups/{root.id}                            → 404
GET /groups/{child.id}                           → 404 (cascade)
GET /memberships?$filter=group_id eq '{child.id}'  → items: [] (cleaned up)
```

---

### S8: `test_error_response_rfc9457`

**Seam**: Error middleware — DomainError → HTTP status + `application/problem+json` Content-Type + no internal leaks.

**Why not in 0006**: `domain_unit_test.rs` asserts `DomainError → Problem` mapping (status code, title). But it doesn't test the HTTP middleware that serializes Problem to JSON with the correct `Content-Type` header. If the error handler is missing or the middleware is misconfigured, you get `application/json` with a generic Actix/Axum error instead of RFC-9457.

```
GET /groups/{random-uuid}                        → 404
  assert "content-type" header contains "application/problem+json"
  assert body has "status": 404, "title", "detail"
  assert "stack" not in body and "trace" not in body

POST /types {invalid: "body"}                    → 400
  assert body has "status": 400

POST /types {duplicate of existing}              → 409
  assert body has "status": 409
```

Three status codes, one test. Error middleware is uniform — if it works for 404, it works for all.

---

### S9: `test_pagination_cursor_roundtrip`

**Seam**: Cursor encode/decode across HTTP — base64 token survives URL encoding, pagination offset doesn't drift.

**Why not in 0006**: 0006 tests `Page<T>` construction and `PageInfo` fields. It does NOT test the cursor **codec**: the handler encodes a cursor into the response, the client sends it back as `$skiptoken`, and the handler decodes it to resume. A bug in encode/decode (e.g., wrong base64 variant, missing URL-safe encoding) only manifests over HTTP.

```
Create 5 groups of same type

all_ids = []
cursor = None
while True:
    GET /groups?$top=2&$skiptoken={cursor}     → 200
    all_ids.extend(page item IDs)
    if not page_info.has_next_page:
        break
    cursor = page_info.next_cursor

assert len(all_ids) == len(set(all_ids))         (no duplicates)
assert all 5 created IDs present                 (no missing)
```

---

### S10: `test_membership_filter_wiring`

**Seam**: OData `$filter` parsing → SQL WHERE clause for memberships.

**Why not in 0006**: 0006 TC-G42/G44 verify that `MembershipFilterField` maps `group_id` to the correct column name and kind. But the full chain — HTTP `$filter=group_id eq '{id}'` → OData parser → FilterField lookup → SQL `WHERE group_id = ?` — is never tested end-to-end. A mismatch between the parser's expected field name and the FilterField impl breaks filtering silently (returns all rows instead of filtered).

```
POST type (with allowed_memberships)
POST group_a, group_b

PUT /groups/{a.id}/memberships/{type}/res-1     → 201
PUT /groups/{b.id}/memberships/{type}/res-2     → 201

GET /memberships?$filter=group_id eq '{a.id}'   → 200
  assert all items have group_id == a.id
  assert res-2 NOT in items                      (filter actually applied)

GET /memberships?$filter=group_id eq '{b.id}'   → 200
  assert all items have group_id == b.id
```

---

## 4. Optional Suite

| Suite | Skip condition | Tests |
|-------|---------------|-------|
| **Cross-tenant** (S4) | `E2E_AUTH_TOKEN_TENANT_B` not set | 1 test |
| **MTLS** | `E2E_MTLS_CERT_DIR` not set | Existing `test_mtls_auth.py` (4 tests) |

All other 9 core tests run with a single token, no special infra.

---

## 5. Anti-Patterns (do NOT test here)

| Don't test | Why | Where it's covered |
|---|---|---|
| Type validation (invalid GTS path, placement invariant) | Pure domain logic, deterministic | 0006 TC-TYP-02/04, TC-SDK-01–12 |
| Cycle detection logic | Pure domain logic on closure table | 0006 TC-GRP-06/07 |
| allowed_parents / allowed_memberships enforcement | Junction table lookup, deterministic | 0006 TC-TYP-01–03, TC-MBR-05 |
| Query profile (max_depth, max_width) | Pure domain logic | 0006 TC-GRP-17/18/19 |
| Group name validation (empty, >255) | CHECK constraint + domain validation | 0006 TC-GRP-20/21 |
| Membership tenant compatibility | Domain logic with mock tenants | 0006 TC-MBR-06/10/14 |
| Seeding idempotency | Domain logic, no HTTP | 0006 TC-SEED-01–06 |
| Individual error variants (11 DomainError types) | Error construction + mapping | 0006 domain_unit_test (41 tests) |
| AccessScope construction | Pure logic | 0006 tenant_scoping_test (10 tests) |
| InGroup / InGroupSubtree predicates | Compiler + SecureORM | 0006 tenant_filtering_db_test + cond.rs |

---

## Definitions of Done

### E2E Test Suite Implementation

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-e2e-test-suite`

The system **MUST** have 10 E2E tests covering integration seams as defined in section 3.

## 6. Acceptance Criteria

**Suite-level:**
- [x] 10 core tests in single file `test_integration_seams.py`
- [x] Total suite runtime < 15 seconds (excluding optional)
- [x] Zero flakes on 10 consecutive runs (`pytest --count=10` with `pytest-repeat`)
- [x] `pytest-timeout` configured: per-test hard limit 10s, per-request 5s
- [x] `asyncio_mode = auto` — no `@pytest.mark.asyncio` boilerplate
- [x] No `time.sleep()` in any test

**Per-test quality:**
- [x] Each test targets exactly one integration seam (documented in test docstring)
- [x] S1 (route smoke) requires no data setup — fastest possible
- [x] S2 (DTO roundtrip) verifies exact JSON key names, not just "response is 200"
- [x] S5/S6 (closure) verify `depth` values, not just "hierarchy returns items"
- [x] S7 (force delete) verifies children, memberships cleaned — not just 204
- [x] S8 (error format) checks `Content-Type: application/problem+json` header
- [x] S9 (pagination) asserts no duplicates AND no missing items across pages

**Isolation:**
- [x] Each test creates its own data — no cross-test dependencies
- [x] S4 (cross-tenant) skips gracefully when second token unavailable
- [x] No test duplicates 0006 domain logic — if removing the test doesn't reduce integration confidence, the test shouldn't exist

> Design guided by [Google SMURF (2024)](https://testing.googleblog.com/2024/10/smurf-beyond-test-pyramid.html): each test justified by high **Fidelity** (real PG + real AuthZ) that compensates for lower **Speed** vs unit tests. Tests that add Fidelity without unique integration coverage are cut.
