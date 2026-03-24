# Feature: Unit & Integration Test Plan

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-unit-testing`

- [x] `p1` - `cpt-cf-resource-group-feature-unit-testing`

## Feature Context

### Overview

Unit and integration test plan for the resource-group module. Covers ~140 tests across domain services, value objects, error chains, DTOs, OData fields, seeding, and REST API layer using SQLite in-memory and mocked AuthZ.

### Purpose

Ensure deterministic domain logic correctness for features 0001-0005 and ADR-001. Unit tests are the primary defense line; E2E tests (Feature 0007) cover integration seams only.

**Requirements**: Features 0001-0005, ADR-001

### Actors

| Actor | Role in Feature |
|-------|-----------------|
| Developer | Runs tests via `cargo test`, adds new test cases |

### References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **Dependencies**: Features 0001-0005, ADR-001

## Actor Flows (CDSL)

No actor flows — this is a test specification feature.

## Processes / Business Logic (CDSL)

No processes — test logic is defined in test plan sections below.

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
  - [What Is a Unit Test in This Project](#what-is-a-unit-test-in-this-project)
  - [Three Questions Before Adding a Test](#three-questions-before-adding-a-test)
  - [What We Test Here](#what-we-test-here)
  - [What We Do NOT Test Here](#what-we-do-not-test-here)
  - [Relationship to Feature 0007 (E2E)](#relationship-to-feature-0007-e2e)
  - [Reliability Principles](#reliability-principles)
- [1. Overview](#1-overview)
- [2. Gap Analysis: Current Coverage](#2-gap-analysis-current-coverage)
  - [2.1 What IS Covered](#21-what-is-covered)
  - [2.2 What IS NOT Covered (Gaps)](#22-what-is-not-covered-gaps)
- [3. Test Plan by Area](#3-test-plan-by-area)
  - [3.1 Type Management (Feature 0002)](#31-type-management-feature-0002)
  - [3.2 Entity Hierarchy (Feature 0003)](#32-entity-hierarchy-feature-0003)
  - [3.3 Membership (Feature 0004)](#33-membership-feature-0004)
  - [3.4 SDK Value Objects & Models (Feature 0001)](#34-sdk-value-objects--models-feature-0001)
  - [3.5 DTO Conversions & Serialization](#35-dto-conversions--serialization)
  - [3.6 OData Filter Field Mapping](#36-odata-filter-field-mapping)
  - [3.7 Seeding (Features 0002-0004)](#37-seeding-features-0002-0004)
  - [3.8 Error Conversions](#38-error-conversions)
  - [3.9 Metadata Tests](#39-metadata-tests)
  - [3.10 Invalid / Non-GTS Input Tests](#310-invalid--non-gts-input-tests)
  - [3.11 ADR-001 GTS Type System — RG-Level Validation of metadata Values](#311-adr-001-gts-type-system--rg-level-validation-of-metadata-values)
  - [3.12 GTS-Specific Logic Tests](#312-gts-specific-logic-tests)
  - [3.13 REST API Layer (existing endpoints without tests)](#313-rest-api-layer-existing-endpoints-without-tests)
- [4. Priority Matrix](#4-priority-matrix)
  - [P1 - Critical (must have, business invariants) — 62 tests](#p1---critical-must-have-business-invariants--62-tests)
  - [P2 - Important (error paths, REST layer, edges) — 53 tests](#p2---important-error-paths-rest-layer-edges--53-tests)
  - [P3 - Nice to have (boundary, cosmetic) — 4 tests](#p3---nice-to-have-boundary-cosmetic--4-tests)
- [5. Assert Guidelines — What to Verify Beyond Ok/Err](#5-assert-guidelines--what-to-verify-beyond-okerr)
  - [5.1 Closure Table Assertions (`resource_group_closure`)](#51-closure-table-assertions-resourcegroupclosure)
  - [5.2 Junction Table Assertions (`gts_type_allowed_parent`, `gts_type_allowed_membership`)](#52-junction-table-assertions-gtstypeallowedparent-gtstypeallowedmembership)
  - [5.3 Membership Table Assertions (`resource_group_membership`)](#53-membership-table-assertions-resourcegroupmembership)
  - [5.4 Surrogate ID Non-Exposure (REST tests)](#54-surrogate-id-non-exposure-rest-tests)
  - [5.5 Entity State Assertions (`resource_group` table)](#55-entity-state-assertions-resourcegroup-table)
  - [5.6 Hierarchy Endpoint Response Shape](#56-hierarchy-endpoint-response-shape)
  - [5.7 Seeding DB Verification](#57-seeding-db-verification)
- [6. Test Infrastructure](#6-test-infrastructure)
  - [Core Principles](#core-principles)
  - [Anti-patterns (DO NOT)](#anti-patterns-do-not)
  - [Assertion & Parameterization Patterns](#assertion--parameterization-patterns)
  - [Shared Test Helpers (`tests/common/mod.rs`)](#shared-test-helpers-testscommonmodrs)
  - [Naming Convention](#naming-convention)
  - [Test File Organization](#test-file-organization)
- [7. Definitions of Done](#7-definitions-of-done)
  - [SDK Value Object & Model Tests](#sdk-value-object--model-tests)
  - [Unit Test Coverage for Type Management](#unit-test-coverage-for-type-management)
  - [Unit Test Coverage for Entity Hierarchy](#unit-test-coverage-for-entity-hierarchy)
  - [Unit Test Coverage for Membership](#unit-test-coverage-for-membership)
  - [OData Filter & DTO Tests](#odata-filter--dto-tests)
  - [Seeding Tests](#seeding-tests)
  - [Error Conversion Chain Tests](#error-conversion-chain-tests)
  - [REST API Test Coverage](#rest-api-test-coverage)
- [Acceptance Criteria](#acceptance-criteria)

<!-- /toc -->

---

## TL;DR

~140 tests. Fast (< 5s total). Zero sleeps. Every test atomic.

Unit tests guard **deterministic domain logic** — the same logic that runs identically regardless of whether it's called via HTTP or directly in Rust. If a test needs a real PostgreSQL or a real HTTP connection, it belongs in Feature 0007 (E2E), not here.

---

## Philosophy

### What Is a Unit Test in This Project

A unit test calls a Rust function directly — a service method, a value object constructor, an error conversion — and verifies the result. It uses SQLite `:memory:` for persistence (same migration scripts, ~1ms per DB), mocked AuthZ (`AllowAllAuthZ`), and no network I/O.

This is the **primary line of defense**. Every domain invariant, every validation rule, every error path is covered here. E2E tests (Feature 0007) exist only to verify integration seams that unit tests cannot see.

### Three Questions Before Adding a Test

Every test in this plan has been validated against three questions:

1. **"Does this test verify deterministic domain logic?"**
   If yes — it belongs here. Type validation, hierarchy invariants, metadata field constraints, closure table correctness, error mapping — all deterministic, all testable without HTTP.

2. **"Is this test atomic and fast?"**
   One `#[test]` = one scenario. No `sleep()`, no `timeout()`, no retry loops. Each test creates its own SQLite DB and service instances. Tests run in parallel (`cargo test -j N`). Target: entire suite < 5 seconds.

3. **"Does removing this test reduce confidence in domain correctness?"**
   If not — the test is redundant. Every test must guard a specific behavior that, if broken, would allow data corruption, invariant violation, or silent failure.

### What We Test Here

| Layer | What | How |
|-------|------|-----|
| **Domain services** | Type CRUD, group lifecycle, hierarchy, membership, seeding | `#[tokio::test]` with SQLite `:memory:` + mocked AuthZ |
| **Domain validation** | GTS type path format, name length, placement invariant, metadata against schema | `#[test]` pure logic, no DB |
| **Value objects** | `GtsTypePath` parsing, normalization, serde round-trip | `#[cfg(test)]` in-source |
| **Error chains** | `DomainError` → `ResourceGroupError` → `Problem` (RFC 9457), `EnforcerError` → `DomainError` | `#[test]` pure logic |
| **DTO conversions** | `From` impls, serde attributes (`rename`, `skip_serializing_if`, `default`, `camelCase`) | `#[cfg(test)]` in-source |
| **OData fields** | `FilterField` name/kind mapping, OData mapper field→column | `#[cfg(test)]` in-source |
| **Closure table** | Row correctness after create/move/delete — verified via direct DB queries | `#[tokio::test]` with DB assert helpers |
| **Metadata validation** | Group `metadata` validated against type's `metadata_schema` (ADR-001): field types, maxLength, `additionalProperties: false` | `#[tokio::test]` service-level |
| **SMALLINT non-exposure** | API responses contain GTS path strings, never SMALLINT surrogate IDs | REST-level `Router::oneshot` |

### What We Do NOT Test Here

- HTTP routing, middleware wiring, header serialization → Feature 0007 (E2E)
- PostgreSQL-specific behavior (FK RESTRICT, SERIALIZABLE isolation, `gts_type_path` DOMAIN) → Feature 0007
- Real AuthN/AuthZ pipeline with tokens → Feature 0007
- MTLS certificate verification → Feature 0007
- Cursor codec encode/decode over HTTP → Feature 0007
- Performance, load, concurrency under contention → out of scope

All of the above requires a running server with real PostgreSQL. Unit tests use SQLite and mock AuthZ — they cannot catch these bugs. Feature 0007 (10 E2E tests) covers these integration seams.

### Relationship to Feature 0007 (E2E)

Feature 0006 and 0007 form a **complementary pair** with zero overlap:

| Concern | 0006 (Unit) | 0007 (E2E) |
|---------|-------------|------------|
| Domain invariants | **Yes** (primary) | No |
| Metadata field validation | **Yes** (service-level) | No |
| Closure table correctness | **Yes** (SQLite) | Yes (PostgreSQL dialect) |
| Error response format | DomainError→Problem mapping | HTTP headers + Content-Type |
| Tenant isolation | AccessScope construction + scoped queries | Real tokens + real WHERE |
| JSON wire format | DTO serde attrs in-source | Full HTTP roundtrip |
| OData $filter | FilterField name/kind | Full parse→SQL→result chain |

If a bug is catchable by calling a Rust function directly, it lives in 0006. If it requires HTTP + PostgreSQL, it lives in 0007.

### Reliability Principles

**Atomic** — one `#[test]` = one behavior. No compound "test everything" functions.

**Fast** — no `sleep`, no `timeout`, no `tokio::time::*`, no polling. SQLite `:memory:` is ~1ms. Target: full suite < 5s.

**Independent** — no shared state. Each test creates its own DB. `cargo test -j N` runs in parallel.

**Synchronous where possible** — pure logic uses `#[test]`, not `#[tokio::test]`. Async only when DB is involved.

**Direct DB assertions** — do not rely solely on service-layer reads (they go through AccessScope). Use `sea_orm::Entity::find()` directly to verify closure table rows, junction tables, entity state.

**No new crate dependencies for testing** — follow project conventions. `assert!(matches!(err, Variant { .. }), "msg: {err:?}")` not `assert_matches!`. Manual `vec![]` + loop for table-driven tests, not `rstest`. Plain `async fn` helpers, not fixtures.

**No retry testing** — the SERIALIZABLE retry loop is an implementation detail. Tests do not simulate contention.

---

## 1. Overview

This feature covers the unit and integration test plan for the `resource-group` module. The plan is based on a thorough gap analysis between the existing test suite (5 test files in `tests/` + 1 in-source `#[cfg(test)]` block in `auth.rs`, ~2,450 lines total) and the acceptance criteria defined in features 0001-0005 and ADR-001 (GTS Type System).

The analysis incorporates:
- Acceptance criteria from features 0001-0005 and ADR-001
- Testing patterns from other project modules (`nodes-registry`, `types-registry`, `api-gateway`)
- ADR-001 metadata validation requirements (`additionalProperties: false`, field types, maxLength constraints)

**Scope**: Domain service tests with SQLite in-memory, in-source `#[cfg(test)]` for pure logic, metadata validation against `metadata_schema`.

**Out of scope**: E2E tests (Feature 0007), PostgreSQL-specific tests, MTLS, performance.

## 2. Gap Analysis: Current Coverage

### 2.1 What IS Covered

| File | Lines | Tests | Covers |
|------|-------|-------|--------|
| `domain_unit_test.rs` | 384 | 41 sync | `validate_type_code` (5 cases), `DomainError` construction (13 variants), `DomainError` -> `ResourceGroupError` mapping (7 cases), `DomainError` -> `Problem` mapping (12 cases), serialization failure detection (4 cases) |
| `api_rest_test.rs` | 481 | 9 async | Type CRUD REST (create 201, dup 409, invalid 400, list 200, get 200/404, delete 204), Group REST (create 201, list 200, get 404), RFC 9457 error format |
| `authz_integration_test.rs` | 450 | 9 async | PolicyEnforcer tenant scoping, deny-all, allow-all, resource_id passing, all CRUD actions, full chain list_groups/deny |
| `tenant_filtering_db_test.rs` | 715 | 7 async | Tenant isolation (list/get/hierarchy/update/delete cross-tenant), InGroup predicate, membership data storage |
| `tenant_scoping_test.rs` | 137 | 10 sync | AccessScope construction (for_tenant, for_tenants, allow_all, deny_all, tenant_only, for_resource) |
| `src/api/rest/auth.rs` (in-source) | 190 | 12 sync | MTLS auth mode routing: determine_auth_mode (JWT/MTLS), path_matches_pattern, allowed_clients, endpoint allowlist |

**Total**: ~2,450 lines, 88 tests (53 sync + 35 async)

### 2.2 What IS NOT Covered (Gaps)

**Critical gaps** (business logic not tested):

| # | Area | Gap | Acceptance Criteria Ref |
|---|------|-----|------------------------|
| G1 | Type Update | No update type tests at all (service or REST) | 0002-AC-5,6,7 |
| G2 | Type Safety | Hierarchy safety check on update (remove allowed_parent in use) | 0002-AC-5 |
| G3 | Type Safety | Hierarchy safety check (can_be_root=false with root groups) | 0002-AC-6 |
| G4 | Type Validation | Create type with non-existent allowed_parents | 0002-AC-3 |
| G5 | Type Validation | Create type with non-existent allowed_memberships | 0002-AC-3 |
| G6 | Type Validation | Placement invariant (can_be_root=false AND no parents) | 0002-AC-4 |
| G7 | Type Delete | Delete type with existing groups -> ConflictActiveReferences | 0002-AC-9 |
| G8 | Group Hierarchy | Create child group with parent (closure rows) | 0003-AC-1,2 |
| G9 | Group Hierarchy | Create group with incompatible parent type | 0003-AC-3 |
| G10 | Group Hierarchy | Create root when can_be_root=false | 0003-AC-5 |
| G11 | Group Move | Move group (subtree closure rebuild) | 0003-AC-6 |
| G12 | Group Move | Move under descendant -> CycleDetected | 0003-AC-7 |
| G13 | Group Move | Self-parent -> CycleDetected | 0003-AC-8 |
| G14 | Group Move | Move to incompatible parent type | 0003-AC-9 |
| G15 | Group Update | Update name/metadata/type | 0003-AC-10 |
| G16 | Group Update | Type change validates parent + children compatibility | 0003-AC-10 |
| G17 | Group Delete | Leaf delete (happy path) | 0003-AC-11 |
| G18 | Group Delete | Delete with children without force -> ConflictActiveReferences | 0003-AC-12 |
| G19 | Group Delete | Delete with memberships without force -> ConflictActiveReferences | 0003-AC-12 |
| G20 | Group Delete | Force delete subtree (cascade) | 0003-AC-13 |
| G21 | Hierarchy | Hierarchy endpoint depth traversal (ancestors + descendants) | 0003-AC-14 |
| G22 | Query Profile | max_depth enforcement on create | 0003-AC-16 |
| G23 | Query Profile | max_width enforcement on create | 0003-AC-17 |
| G24 | Query Profile | max_depth enforcement on move | 0003-AC-16 |
| G25 | Membership | Add membership REST (happy path 201) | 0004-AC-1 |
| G26 | Membership | Add to nonexistent group -> NotFound | 0004-AC-2 |
| G27 | Membership | Add duplicate -> Conflict | 0004-AC-3 |
| G28 | Membership | Unregistered resource_type -> Validation | 0004-AC-4 |
| G29 | Membership | resource_type not in allowed_memberships -> Validation | 0004-AC-5 |
| G30 | Membership | Tenant compatibility check -> TenantIncompatibility | 0004-AC-7 |
| G31 | Membership | Remove existing membership 204 | 0004-AC-8 |
| G32 | Membership | Remove nonexistent -> NotFound | 0004-AC-9 |
| G33 | Validation | Group name validation (empty, >255 chars) | 0003-flow |
| G34 | REST | Update type REST (PUT) | 0002-AC |
| G35 | REST | Membership REST endpoints (POST/DELETE) | 0004-AC |

**SDK & Infrastructure gaps** (patterns from other modules not applied here):

| # | Area | Gap | Source File |
|---|------|-----|-------------|
| G36 | SDK Value Object | `GtsTypePath::new()` - no in-source `#[cfg(test)]` for format validation, trim/lowercase, length, TryFrom/serde round-trip | `resource-group-sdk/src/models.rs` (110 lines of logic, 0 tests) |
| G37 | SDK Value Object | `GtsTypePath` serde round-trip: `#[serde(try_from = "String", into = "String")]` not tested | `models.rs:24-26` |
| G38 | SDK Value Object | `GtsTypePath::matches_format()` - regex-like validation with edge cases (double tilde, uppercase, special chars) | `models.rs:82-110` |
| G39 | DTO Conversion | 9 `From` impls (TypeDto, CreateTypeDto, UpdateTypeDto, GroupDto, GroupWithDepthDto, CreateGroupDto, UpdateGroupDto, MembershipDto) - no round-trip tests | `api/rest/dto.rs` (279 lines, 0 tests) |
| G40 | DTO Serde | `#[serde(rename = "type")]` for type_path, `#[serde(skip_serializing_if)]` for Option fields, `#[serde(default)]` for Vec - JSON shape not verified | `dto.rs` |
| G41 | DTO Serde | `#[serde(rename_all = "camelCase")]` on SDK models - camelCase wire format not verified | `sdk/models.rs` |
| G42 | OData Fields | `GroupFilterField` manual `FilterField` impl: field names (`hierarchy/parent_id`, `type`, `id`, `name`) and kinds (I64, String, Uuid) not unit-tested | `sdk/odata/groups.rs` |
| G43 | OData Fields | `HierarchyFilterField` field names (`hierarchy/depth`, `type`) and kinds not tested | `sdk/odata/hierarchy.rs` |
| G44 | OData Fields | `MembershipFilterField` field names and kinds not tested | `sdk/odata/memberships.rs` |
| G45 | OData Mapper | `TypeODataMapper`, `GroupODataMapper`, `MembershipODataMapper` field-to-column mapping correctness not tested | `infra/storage/odata_mapper.rs` (136 lines, 0 tests) |
| G46 | Seeding | `seed_types()` - idempotent create/update/skip with SeedResult tracking | `domain/seeding.rs` (189 lines, 0 tests) |
| G47 | Seeding | `seed_groups()` - ordered group seeding, idempotent skip, anonymous SecurityContext | `domain/seeding.rs` |
| G48 | Seeding | `seed_memberships()` - Conflict->skip, TenantIncompatibility->skip, SeedResult | `domain/seeding.rs` |
| G49 | Error Chain | `EnforcerError -> DomainError` mapping (Denied/EvaluationFailed/CompileFailed -> AccessDenied) | `domain/error.rs:190-201` |
| G50 | Error Chain | `sea_orm::DbErr -> DomainError` conversion | `domain/error.rs:178-182` |
| G51 | Error Chain | `modkit_db::DbError -> DomainError` conversion | `domain/error.rs:184-188` |
| G52 | QueryProfile | `QueryProfile::default()` returns `max_depth: Some(10), max_width: None` | `group_service.rs:49-56` |

---

## 3. Test Plan by Area

### 3.1 Type Management (Feature 0002)

**File**: `type_service_test.rs` (new)

Test setup: SQLite in-memory + TypeService + GroupService (for hierarchy safety tests).

#### TC-TYP-01: Create type with valid allowed_parents [P1]
- **Covers**: G4 (positive path), 0002-AC-1
- **Setup**: Create parent type first, then create child type referencing it
- **Assert**: Child type created, `allowed_parents` contains parent code

#### TC-TYP-02: Create type with non-existent allowed_parents [P1]
- **Covers**: G4
- **Setup**: Create type with `allowed_parents: ["gts.x.system.rg.type.v1~nonexistent.v1~"]`
- **Assert**: `DomainError::TypeNotFound` or `DomainError::Validation`

#### TC-TYP-03: Create type with non-existent allowed_memberships [P1]
- **Covers**: G5
- **Setup**: Create type with `allowed_memberships: ["gts.x.system.rg.type.v1~nonexistent.v1~"]`
- **Assert**: Error (type not found)

#### TC-TYP-04: Placement invariant violation (can_be_root=false, no parents) [P1]
- **Covers**: G6, 0002-AC-4
- **Setup**: `CreateTypeRequest { can_be_root: false, allowed_parents: [] }`
- **Assert**: `DomainError::Validation` with "root placement or" message

#### TC-TYP-05: Update type happy path [P1]
- **Covers**: G1, 0002-AC-7
- **Setup**: Create type, then update with new `allowed_parents`
- **Assert**: Updated type returned with new parents

#### TC-TYP-06: Update type - remove allowed_parent in use by groups [P1]
- **Covers**: G2, 0002-AC-5
- **Setup**: Create parent type P, child type C (allowed_parents=[P]), create group of type P, create child group of type C under P group. Then update type C removing P from allowed_parents.
- **Assert**: `DomainError::AllowedParentsViolation` with violating group names

#### TC-TYP-07: Update type - set can_be_root=false with existing root groups [P1]
- **Covers**: G3, 0002-AC-6
- **Setup**: Create type with can_be_root=true, create root group of that type. Then update type setting can_be_root=false.
- **Assert**: `DomainError::AllowedParentsViolation` with root group names

#### TC-TYP-08: Update type - not found [P2]
- **Covers**: G1
- **Assert**: `DomainError::TypeNotFound`

#### TC-TYP-09: Delete type with existing groups [P1]
- **Covers**: G7, 0002-AC-9
- **Setup**: Create type, create group of that type. Delete type.
- **Assert**: `DomainError::ConflictActiveReferences`

#### TC-TYP-10: Update type - placement invariant on new values [P2]
- **Covers**: G1
- **Setup**: Update type with can_be_root=false and allowed_parents=[]
- **Assert**: `DomainError::Validation`

#### TC-TYP-11: Create type with self-reference in allowed_parents [P2]
- Type A lists itself as allowed_parent, but A doesn't exist yet during resolve_ids
- **Assert**: Error (type not found for self-reference)

#### TC-TYP-12: Create type with invalid format in allowed_parents[i] [P2]
- allowed_parents: `["wrong.prefix"]` — each parent validated via validate_type_code
- **Assert**: `DomainError::Validation` (prefix error)

#### TC-TYP-13: Delete nonexistent type [P2]
- **Assert**: `DomainError::TypeNotFound`

#### TC-TYP-14: Create type with metadata_schema [P2]
- Create type with `metadata_schema: Some(json_schema)`, get type, verify schema stored
- **Assert**: Returned type has matching metadata_schema

#### TC-TYP-15: Update type replaces allowed_memberships [P2]
- Create type with memberships [A, B], update to [B, C]
- **Assert**: Updated type has only [B, C], A removed

#### TC-TYP-16: Update type - hierarchy check skips deleted parent type [P3]
- Remove parent type from allowed_parents, but the parent type itself was already deleted from system
- **Assert**: No error (resolve_id returns None → skip)

---

### 3.2 Entity Hierarchy (Feature 0003)

**File**: `group_service_test.rs` (new)

Test setup: SQLite in-memory + TypeService + GroupService with configurable QueryProfile.

#### TC-GRP-01: Create child group with parent - closure rows correct [P1]
- **Covers**: G8, 0003-AC-1,2
- **Setup**: Create parent type (can_be_root=true), child type (allowed_parents=[parent_type]). Create root group, then child group under it.
- **Assert**: Child group returned with `hierarchy.parent_id`, closure table has self-row (depth=0) and ancestor row (depth=1)

#### TC-GRP-02: Create 3-level hierarchy - closure table completeness [P1]
- **Covers**: G8, 0003-AC-2
- **Setup**: Grandparent -> Parent -> Child groups
- **Assert**: Child has closure rows to grandparent (depth=2), parent (depth=1), self (depth=0). Parent has rows to grandparent (depth=1), self (depth=0).

#### TC-GRP-03: Create group with incompatible parent type [P1]
- **Covers**: G9, 0003-AC-3
- **Setup**: Create type A (can_be_root=true), type B (can_be_root=false, allowed_parents=[]... wait, that violates placement invariant). Create type A (root), type B (allowed_parents=[A]), type C (allowed_parents=[B]). Create root group of type A. Try to create child of type C under group A.
- **Assert**: `DomainError::InvalidParentType`

#### TC-GRP-04: Create root group when can_be_root=false [P1]
- **Covers**: G10, 0003-AC-5
- **Setup**: Create type with can_be_root=false, allowed_parents=[some_parent_type]. Try to create root group (no parent).
- **Assert**: `DomainError::InvalidParentType` with "cannot be a root group"

#### TC-GRP-05: Move group - happy path with closure rebuild [P1]
- **Covers**: G11, 0003-AC-6
- **Setup**: Create tree: Root1 -> Child -> Grandchild. Create Root2. Move Child (with subtree) under Root2.
- **Assert**: Child.parent_id == Root2.id. Closure table rebuilt: Grandchild has path to Root2 (depth=2), Child (depth=1), self (depth=0). Old paths to Root1 removed.

#### TC-GRP-06: Move group under its descendant -> CycleDetected [P1]
- **Covers**: G12, 0003-AC-7
- **Setup**: Create Root -> Parent -> Child. Try to move Root under Child.
- **Assert**: `DomainError::CycleDetected`

#### TC-GRP-07: Self-parent -> CycleDetected [P1]
- **Covers**: G13, 0003-AC-8
- **Setup**: Create group. Update with parent_id = own id.
- **Assert**: `DomainError::CycleDetected`

#### TC-GRP-08: Move group to incompatible parent type [P1]
- **Covers**: G14, 0003-AC-9
- **Setup**: Type A (root), Type B (allowed_parents=[A]). Create group of type B under group of type A. Create group of type A (root). Try to move group B under another group B.
- **Assert**: `DomainError::InvalidParentType`

#### TC-GRP-09: Update group name and metadata [P2]
- **Covers**: G15
- **Setup**: Create group, update with new name and metadata
- **Assert**: Updated group returned with new name/metadata

#### TC-GRP-10: Update group type - validates parent compatibility [P1]
- **Covers**: G16, 0003-AC-10
- **Setup**: Type A (root), Type B (allowed_parents=[A]), Type C (root, no allowed_parents). Create group of type B under group of type A. Change group type to C (which doesn't allow parent A).
- **Assert**: `DomainError::InvalidParentType` ("does not allow current parent type")

#### TC-GRP-11: Update group type - validates children compatibility [P1]
- **Covers**: G16, 0003-AC-10
- **Setup**: Type P (root), Type C (allowed_parents=[P]), Type P2 (root). Create P group with C child. Change P group to type P2.
- **Assert**: `DomainError::InvalidParentType` ("child group... does not allow... as parent type")

#### TC-GRP-12: Delete leaf group (no children, no memberships) [P1]
- **Covers**: G17, 0003-AC-11
- **Setup**: Create group, delete without force
- **Assert**: Success, group no longer found, closure rows removed

#### TC-GRP-13: Delete group with children without force [P1]
- **Covers**: G18, 0003-AC-12
- **Setup**: Create parent -> child. Delete parent without force.
- **Assert**: `DomainError::ConflictActiveReferences` with "child group(s)"

#### TC-GRP-14: Delete group with memberships without force [P1]
- **Covers**: G19, 0003-AC-12
- **Setup**: Create group, add membership. Delete group without force.
- **Assert**: `DomainError::ConflictActiveReferences` with "memberships"

#### TC-GRP-15: Force delete subtree [P1]
- **Covers**: G20, 0003-AC-13
- **Setup**: Create Root -> Parent -> Child, with memberships on each. Force delete Root.
- **Assert**: All 3 groups gone, all memberships gone, all closure rows gone

#### TC-GRP-16: Hierarchy endpoint - ancestors and descendants [P1]
- **Covers**: G21, 0003-AC-14
- **Setup**: Create 3-level tree (A -> B -> C). Call list_group_hierarchy(B).
- **Assert**: Returns A (depth=-1), B (depth=0), C (depth=1)

#### TC-GRP-17: max_depth enforcement on create [P1]
- **Covers**: G22, 0003-AC-16
- **Setup**: QueryProfile { max_depth: Some(2), max_width: None }. Create Root (depth=0), Child (depth=1). Try to create Grandchild (depth=2).
- **Assert**: `DomainError::LimitViolation` with "Depth limit exceeded"

#### TC-GRP-18: max_width enforcement on create [P1]
- **Covers**: G23, 0003-AC-17
- **Setup**: QueryProfile { max_depth: None, max_width: Some(2) }. Create Root, add Child1, Child2 under Root. Try to add Child3.
- **Assert**: `DomainError::LimitViolation` with "Width limit exceeded"

#### TC-GRP-19: max_depth enforcement on move [P2]
- **Covers**: G24, 0003-AC-16
- **Setup**: QueryProfile { max_depth: Some(3) }. Deep tree. Move subtree to position that would exceed max_depth.
- **Assert**: `DomainError::LimitViolation`

#### TC-GRP-20: Group name validation - empty [P2]
- **Covers**: G33
- **Assert**: `DomainError::Validation` with "between 1 and 255"

#### TC-GRP-21: Group name validation - too long (>255) [P2]
- **Covers**: G33
- **Assert**: `DomainError::Validation` with "between 1 and 255"

#### TC-GRP-22: Create group with nonexistent type_path [P1]
- **Assert**: `DomainError::TypeNotFound`

#### TC-GRP-23: Create child group with parent from different tenant [P1]
- Parent.tenant_id != child tenant_id → `DomainError::Validation("must match parent tenant_id")`
- This is NOT InvalidParentType — it's a separate Validation branch (line 379-384)

#### TC-GRP-24: Create group with metadata (JSONB) [P2]
- Create with `metadata: Some(json!({"barrier": true}))`, verify stored and returned

#### TC-GRP-25: Multiple root groups of same type [P2]
- Create 2 root groups of same can_be_root=true type, both succeed

#### TC-GRP-26: Update group - simultaneous type change AND parent change [P1]
- Both `type_changed` and `parent_changed` are true → type validation + move logic both run
- Verify the combined operation succeeds or fails atomically

#### TC-GRP-27: Update root group type to non-root type (no parent) [P1]
- Root group (parent_id=None), change type to can_be_root=false type
- Hits `else if !rg_type.can_be_root` branch (line 508-512)
- **Assert**: `DomainError::InvalidParentType("cannot be a root group")`

#### TC-GRP-28: Update group with nonexistent new type_path [P2]
- **Assert**: `DomainError::TypeNotFound`

#### TC-GRP-29: Move child to root (detach from parent) - happy path [P1]
- Child under parent, move with new_parent_id=None, type allows can_be_root=true
- **Assert**: Success, parent_id=None, closure rebuilt (old ancestor rows removed, self-row only)

#### TC-GRP-30: Move child to root when can_be_root=false [P1]
- **Assert**: `DomainError::InvalidParentType("cannot be a root group")`

#### TC-GRP-31: Move nonexistent group [P2]
- **Assert**: `DomainError::GroupNotFound`

#### TC-GRP-32: Move to nonexistent parent [P2]
- **Assert**: `DomainError::GroupNotFound` for the new parent

#### TC-GRP-33: max_width enforcement on move [P2]
- Move group under parent that already has max_width children
- **Assert**: `DomainError::LimitViolation("Width limit exceeded")`

#### TC-GRP-34: Delete nonexistent group [P2]
- **Assert**: `DomainError::GroupNotFound`

#### TC-GRP-35: Force delete leaf node (no descendants) [P2]
- Group with no children, force=true — descendant_ids is empty, still works
- **Assert**: Success

#### TC-GRP-36: list_group_hierarchy nonexistent group [P2]
- **Assert**: `DomainError::GroupNotFound`

#### TC-GRP-37: Depth limit exact boundary (parent_depth+1 == max_depth) [P1]
- Comparison is `>=` not `>`: at exact limit, reject
- max_depth=3, parent at depth=2, try add child at depth=3 → `LimitViolation`

#### TC-GRP-38: Width limit exact boundary (sibling_count == max_width) [P1]
- max_width=2, parent has exactly 2 children, try add 3rd → `LimitViolation`

---

### 3.3 Membership (Feature 0004)

**File**: `membership_service_test.rs` (new)

Test setup: SQLite in-memory + TypeService + GroupService + MembershipService.

#### TC-MBR-01: Add membership happy path [P1]
- **Covers**: G25, 0004-AC-1
- **Setup**: Create type with allowed_memberships=[member_type], create group. Add membership.
- **Assert**: Membership returned with group_id, resource_type, resource_id

#### TC-MBR-02: Add membership to nonexistent group [P1]
- **Covers**: G26, 0004-AC-2
- **Assert**: `DomainError::GroupNotFound`

#### TC-MBR-03: Add duplicate membership [P1]
- **Covers**: G27, 0004-AC-3
- **Setup**: Add membership, then add same (group_id, resource_type, resource_id) again
- **Assert**: `DomainError::Conflict`

#### TC-MBR-04: Add membership with unregistered resource_type [P1]
- **Covers**: G28, 0004-AC-4
- **Assert**: `DomainError::Validation` with "Unknown resource type"

#### TC-MBR-05: Add membership with resource_type not in allowed_memberships [P1]
- **Covers**: G29, 0004-AC-5
- **Setup**: Create group of type that does NOT include resource_type in allowed_memberships
- **Assert**: `DomainError::Validation` with "not in allowed_memberships"

#### TC-MBR-06: Tenant compatibility violation [P1]
- **Covers**: G30, 0004-AC-7
- **Setup**: Create group in tenant A, add membership (type, resource-1). Create group in tenant B. Try to add same resource (type, resource-1) to group in tenant B.
- **Assert**: `DomainError::TenantIncompatibility`

#### TC-MBR-07: Remove existing membership [P1]
- **Covers**: G31, 0004-AC-8
- **Setup**: Add membership, then remove it
- **Assert**: Success (no error)

#### TC-MBR-08: Remove nonexistent membership [P1]
- **Covers**: G32, 0004-AC-9
- **Assert**: `DomainError::MembershipNotFound`

#### TC-MBR-09: Multiple resource types in same group [P2]
- **Covers**: 0004-AC-6
- **Setup**: Type with allowed_memberships=[typeA, typeB]. Create group. Add (group, typeA, R1) and (group, typeB, R2).
- **Assert**: Both succeed, list_memberships returns both

#### TC-MBR-10: Tenant compatibility - first membership always allowed [P2]
- **Covers**: 0004-algo-tenant-check-2
- **Setup**: First add for any resource should succeed regardless of tenant
- **Assert**: Success

#### TC-MBR-11: Add membership with empty resource_id [P2]
- No validation on resource_id in code — empty string will be inserted
- Verify it either fails at DB constraint or succeeds (document behavior)

#### TC-MBR-12: Remove membership with unregistered resource_type [P2]
- resolve_id returns None → `DomainError::Validation("Unknown resource type")`

#### TC-MBR-13: Add membership to group with empty allowed_memberships [P1]
- Group type has `allowed_memberships: []` — any resource_type should be rejected
- **Assert**: `DomainError::Validation("not in allowed_memberships")`

#### TC-MBR-14: Same resource linked in multiple groups of same tenant [P1]
- Resource (type, R1) added to Group A (tenant T), then to Group B (tenant T)
- `existing_tenants.contains(&tenant_id)` = true → pass
- **Assert**: Both succeed

#### TC-MBR-15: List memberships empty result [P3]
- No memberships exist, query returns empty Page
- **Assert**: `page.items.is_empty()`

---

### 3.4 SDK Value Objects & Models (Feature 0001)

**File**: `resource-group-sdk/src/models.rs` (in-source `#[cfg(test)]` block — following `auth.rs` pattern)

Other modules (`nodes-registry`, `types-registry`) place pure-logic tests directly in source files. `GtsTypePath` has 110 lines of validation logic with zero tests.

#### TC-SDK-01: GtsTypePath::new() valid path [P1]
- **Covers**: G36, 0001-AC-2
- **Input**: `"gts.x.system.rg.type.v1~"`
- **Assert**: `Ok(GtsTypePath)`, `as_str()` returns lowercase

#### TC-SDK-02: GtsTypePath::new() empty string [P1]
- **Covers**: G36
- **Assert**: `Err("must not be empty")`

#### TC-SDK-03: GtsTypePath::new() exceeds 255 chars [P1]
- **Covers**: G36
- **Assert**: `Err("exceeds maximum length")`

#### TC-SDK-04: GtsTypePath::new() invalid format - no gts prefix [P1]
- **Covers**: G38
- **Input**: `"invalid.path~"`
- **Assert**: `Err("Invalid GTS type path format")`

#### TC-SDK-05: GtsTypePath::new() invalid format - no trailing tilde [P1]
- **Covers**: G38
- **Input**: `"gts.x.system.rg.type.v1"`
- **Assert**: `Err`

#### TC-SDK-06: GtsTypePath::new() invalid format - uppercase chars [P1]
- **Covers**: G38
- **Input**: `"gts.x.system.rg.type.v1~"` with uppercase -> trimmed/lowercased

#### TC-SDK-07: GtsTypePath::new() trims whitespace and lowercases [P2]
- **Covers**: G36
- **Input**: `"  GTS.X.System.RG.Type.V1~  "`
- **Assert**: `Ok`, `as_str() == "gts.x.system.rg.type.v1~"`

#### TC-SDK-08: GtsTypePath::new() chained path (multi-segment) [P1]
- **Covers**: G38
- **Input**: `"gts.x.system.rg.type.v1~x.test.v1~"`
- **Assert**: `Ok`

#### TC-SDK-09: GtsTypePath::new() double tilde (empty segment) [P2]
- **Covers**: G38
- **Input**: `"gts.x.system.rg.type.v1~~"`
- **Assert**: `Err` (empty segment between tildes)

#### TC-SDK-10: GtsTypePath::new() special chars in segment [P2]
- **Covers**: G38
- **Input**: `"gts.x.system.rg.type.v1~hello-world~"` (hyphen not allowed)
- **Assert**: `Err`

#### TC-SDK-11: GtsTypePath serde round-trip (JSON) [P1]
- **Covers**: G37
- **Setup**: Serialize `GtsTypePath` to JSON string, deserialize back
- **Assert**: `serde_json::to_string(&path)` produces `"gts.x.system.rg.type.v1~"`, deserialize back equals original

#### TC-SDK-12: GtsTypePath serde invalid JSON string [P1]
- **Covers**: G37
- **Setup**: `serde_json::from_str::<GtsTypePath>("\"invalid\"")`
- **Assert**: `Err` (validation runs via TryFrom)

#### TC-SDK-13: GtsTypePath Display + Into<String> [P3]
- **Covers**: G36
- **Assert**: `.to_string()` and `String::from(path)` produce same result

#### TC-SDK-18: GtsTypePath "gts.~" (minimal rest, empty segment) [P2]
- rest = "~", segments = ["", ""], first segment empty → Err

#### TC-SDK-19: GtsTypePath numeric segments "gts.123~456~" [P2]
- Digits are allowed chars → Ok

#### TC-SDK-20: GtsTypePath underscores + dots "gts.a_b.c_d~" [P2]
- Valid chars → Ok

#### TC-SDK-21: GtsTypePath whitespace-only input "   " [P2]
- trim → empty → Err("must not be empty")

#### TC-SDK-22: GtsTypePath exactly 255 chars [P2]
- Boundary → Ok

#### TC-SDK-23: GtsTypePath exactly 256 chars [P2]
- Boundary → Err("exceeds maximum length")

#### TC-SDK-24: validate_type_code vs GtsTypePath normalization mismatch [P1]
- `validate_type_code("  GTS.X.SYSTEM.RG.TYPE.V1~  ")` → fails (no trim/lowercase)
- `GtsTypePath::new("  GTS.X.SYSTEM.RG.TYPE.V1~  ")` → succeeds (trims + lowercases)
- Document this inconsistency and verify behavior

#### TC-SDK-14: SDK model camelCase serialization [P1]
- **Covers**: G41
- **Setup**: Serialize `ResourceGroupType` to JSON
- **Assert**: Keys are camelCase (`canBeRoot`, `allowedParents`, `allowedMemberships`, `metadataSchema`)

#### TC-SDK-15: SDK model `type` field rename [P1]
- **Covers**: G41
- **Setup**: Serialize `ResourceGroup` to JSON
- **Assert**: Field is `"type"`, not `"type_path"`

#### TC-SDK-16: SDK model optional fields omitted when None [P2]
- **Covers**: G41
- **Setup**: Serialize `ResourceGroup { metadata: None, .. }` to JSON
- **Assert**: `"metadata"` key absent from JSON

#### TC-SDK-17: QueryProfile default values [P2]
- **Covers**: G52
- **Assert**: `QueryProfile::default().max_depth == Some(10)`, `.max_width == None`

---

### 3.5 DTO Conversions & Serialization

**File**: `api/rest/dto.rs` (in-source `#[cfg(test)]` block)

Other modules test DTO conversion correctness. `dto.rs` has 9 `From` impls and serde attributes with zero tests.

#### TC-DTO-01: ResourceGroupType -> TypeDto preserves all fields [P2]
- **Covers**: G39
- **Assert**: code, can_be_root, allowed_parents, allowed_memberships, metadata_schema all match

#### TC-DTO-02: CreateTypeDto -> CreateTypeRequest conversion [P2]
- **Covers**: G39
- **Assert**: All fields transferred

#### TC-DTO-03: ResourceGroup -> GroupDto preserves hierarchy fields [P2]
- **Covers**: G39
- **Assert**: hierarchy.parent_id, hierarchy.tenant_id match

#### TC-DTO-04: ResourceGroupWithDepth -> GroupWithDepthDto includes depth [P2]
- **Covers**: G39
- **Assert**: hierarchy.depth transferred

#### TC-DTO-05: CreateGroupDto JSON with `type` rename [P1]
- **Covers**: G40
- **Setup**: Deserialize `{"type": "gts...", "name": "X"}` into CreateGroupDto
- **Assert**: `dto.type_path` populated correctly from `"type"` JSON key

#### TC-DTO-06: CreateTypeDto default vectors [P2]
- **Covers**: G40
- **Setup**: Deserialize `{"code":"...", "can_be_root": true}` (no allowed_parents/memberships)
- **Assert**: `allowed_parents == []`, `allowed_memberships == []` (via `#[serde(default)]`)

#### TC-DTO-07: MembershipDto has no tenant_id field [P2]
- **Covers**: G40, 0004-AC-12
- **Setup**: Serialize MembershipDto to JSON
- **Assert**: No `tenant_id` key in output

---

### 3.6 OData Filter Field Mapping

**File**: `resource-group-sdk/src/odata/groups.rs` + `hierarchy.rs` + `memberships.rs` (in-source `#[cfg(test)]` blocks)

OData filter fields use manual `FilterField` trait implementations with string field names and `FieldKind` enum. Incorrect mapping silently breaks filtering.

#### TC-ODATA-01: GroupFilterField names [P1]
- **Covers**: G42
- **Assert**: `Type.name() == "type"`, `HierarchyParentId.name() == "hierarchy/parent_id"`, `Id.name() == "id"`, `Name.name() == "name"`

#### TC-ODATA-02: GroupFilterField kinds [P1]
- **Covers**: G42
- **Assert**: `Type -> I64`, `HierarchyParentId -> Uuid`, `Id -> Uuid`, `Name -> String`

#### TC-ODATA-03: GroupFilterField FIELDS constant completeness [P2]
- **Covers**: G42
- **Assert**: `FIELDS.len() == 4`, contains all variants

#### TC-ODATA-04: HierarchyFilterField names and kinds [P1]
- **Covers**: G43
- **Assert**: `HierarchyDepth.name() == "hierarchy/depth"`, `Type.name() == "type"`, both `I64`

#### TC-ODATA-05: MembershipFilterField names and kinds [P1]
- **Covers**: G44
- **Assert**: `GroupId -> ("group_id", Uuid)`, `ResourceType -> ("resource_type", I64)`, `ResourceId -> ("resource_id", String)`

#### TC-ODATA-06: TypeODataMapper field-to-column mapping [P2]
- **Covers**: G45
- **Assert**: `TypeFilterField::Code` maps to `TypeColumn::SchemaId`

#### TC-ODATA-07: GroupODataMapper field-to-column mapping [P2]
- **Covers**: G45
- **Assert**: `Type -> GtsTypeId`, `HierarchyParentId -> ParentId`, `Id -> Id`, `Name -> Name`

#### TC-ODATA-08: MembershipODataMapper field-to-column mapping [P2]
- **Covers**: G45
- **Assert**: `GroupId -> GroupId`, `ResourceType -> GtsTypeId`, `ResourceId -> ResourceId`

---

### 3.7 Seeding (Features 0002-0004)

**File**: `seeding_test.rs` (new, integration tests with SQLite)

`seeding.rs` (189 lines) has idempotent seed logic with zero tests. Seeding is a deployment-critical path — bugs here corrupt bootstrap data.

#### TC-SEED-01: seed_types creates missing type [P1]
- **Covers**: G46, 0002-AC-10
- **Setup**: Empty DB. Seed one type definition.
- **Assert**: `result.created == 1`, type exists in DB

#### TC-SEED-02: seed_types skips unchanged type [P1]
- **Covers**: G46, 0002-AC-10
- **Setup**: Seed type, then seed again with identical definition.
- **Assert**: `result.unchanged == 1`, `result.created == 0`

#### TC-SEED-03: seed_types updates changed type [P1]
- **Covers**: G46, 0002-AC-10
- **Setup**: Seed type with can_be_root=true, then seed again with can_be_root=false + allowed_parents.
- **Assert**: `result.updated == 1`, type in DB reflects new values

#### TC-SEED-04: seed_types idempotent (3 runs) [P2]
- **Covers**: G46
- **Setup**: Run seed 3 times with same definitions.
- **Assert**: Run 1: all created. Run 2,3: all unchanged.

#### TC-SEED-05: seed_groups creates hierarchy with closure [P1]
- **Covers**: G47, 0003-AC-24
- **Setup**: Seed parent group, then child group (ordered).
- **Assert**: `result.created == 2`, closure rows correct

#### TC-SEED-06: seed_groups skips existing group [P1]
- **Covers**: G47
- **Setup**: Seed group, seed again.
- **Assert**: `result.unchanged == 1`

#### TC-SEED-07: seed_memberships creates links [P1]
- **Covers**: G48, 0004-AC-14
- **Setup**: Create group + type, seed membership definitions.
- **Assert**: `result.created == N`

#### TC-SEED-08: seed_memberships skips duplicates (Conflict -> skip) [P1]
- **Covers**: G48
- **Setup**: Seed membership, seed again.
- **Assert**: `result.unchanged == 1`

#### TC-SEED-09: seed_memberships skips tenant-incompatible (TenantIncompatibility -> skip) [P2]
- **Covers**: G48
- **Setup**: Seed membership in tenant A, then seed same resource to tenant B group.
- **Assert**: `result.skipped == 1`

#### TC-SEED-10: seed_types with empty list [P3]
- **Assert**: `SeedResult { created: 0, updated: 0, unchanged: 0, skipped: 0 }`

#### TC-SEED-11: seed_groups wrong order (child before parent) [P2]
- Child references parent_id that doesn't exist yet → error propagates
- **Assert**: Error (GroupNotFound or similar)

#### TC-SEED-12: seed_memberships with nonexistent group [P2]
- group_id not in DB → error NOT caught by Conflict/TenantIncompatibility → early return Err
- **Assert**: Error propagated (not silently skipped)

---

### 3.8 Error Conversions

**File**: `domain_unit_test.rs` (extend existing)

Existing tests cover `DomainError -> ResourceGroupError` and `DomainError -> Problem`, but miss conversions FROM external crate errors.

#### TC-ERR-01: EnforcerError::Denied -> DomainError::AccessDenied [P1]
- **Covers**: G49
- **Assert**: Mapping produces AccessDenied variant

#### TC-ERR-02: EnforcerError::EvaluationFailed -> DomainError::AccessDenied [P2]
- **Covers**: G49
- **Assert**: Non-deny enforcer errors also map to AccessDenied

#### TC-ERR-03: EnforcerError::CompileFailed -> DomainError::AccessDenied [P2]
- **Covers**: G49

#### TC-ERR-04: sea_orm::DbErr -> DomainError::Database [P2]
- **Covers**: G50
- **Assert**: `DomainError::Database { message }` with original error text

#### TC-ERR-05: modkit_db::DbError -> DomainError::Database [P2]
- **Covers**: G51

---

### 3.9 Metadata Tests

**ZERO existing tests** use non-None metadata. Every test passes `metadata_schema: None` / `metadata: None`.

#### A. Type `metadata_schema` — Internal Storage Logic (`build_stored_schema` / `load_full_type`)

`type_repo.rs` transforms metadata_schema on write (inject `__can_be_root`) and on read (strip `__` keys, derive `can_be_root`). This logic has **0 tests**.

**File**: `type_service_test.rs` (service-level with DB)

#### TC-META-01: Type with metadata_schema Object — round-trip [P1]
- **Setup**: Create type with `metadata_schema: Some(json!({"type": "object", "properties": {"x": {"type": "string"}}}))`
- Get type → returned `metadata_schema` matches input exactly
- **DB assert**: stored JSONB contains `__can_be_root` key (internal) AND user keys
- **API assert**: response does NOT contain `__can_be_root`

#### TC-META-02: Type metadata_schema with non-Object (array) → wrap/unwrap [P1]
- `metadata_schema: Some(json!([1,2,3]))` — `build_stored_schema` wraps it: `{"__user_schema": [1,2,3], "__can_be_root": true}`
- `load_full_type` strips `__` keys → returns `[1,2,3]`? Actually: the `load_full_type` code checks `if let Value::Object(map) = ms` → for non-Object stored value, returns `Some(ms.clone())` — but stored value IS Object (`{"__user_schema": ..., "__can_be_root": ...}`), so it filters keys → returns `{}` → `None` if empty. **BUG?** The array is lost.
- **Assert**: Verify actual behavior — is non-Object metadata_schema recoverable after round-trip?

#### TC-META-03: Type metadata_schema with non-Object (string) → same wrap issue [P1]
- `metadata_schema: Some(json!("my-schema"))` → stored as `{"__user_schema": "my-schema", "__can_be_root": true}`
- Read: Object filtered → `{}` → None. **Original string lost?**
- **Assert**: Document actual behavior

#### TC-META-04: Type metadata_schema with non-Object (number) [P2]
- `metadata_schema: Some(json!(42))` → same wrap pattern
- Verify round-trip

#### TC-META-05: User sends `__can_be_root` in metadata_schema [P1]
- `metadata_schema: Some(json!({"__can_be_root": false, "myField": "value"}))`
- `build_stored_schema`: clones Object, then `map.insert("__can_be_root", Bool(can_be_root))` — **OVERWRITES** user's value
- `load_full_type`: strips `__can_be_root` → returned metadata has only `{"myField": "value"}`
- **Assert**: system's `can_be_root` wins, user's `__can_be_root` silently overwritten, `myField` preserved

#### TC-META-06: User sends `__other_internal` key in metadata_schema [P1]
- `metadata_schema: Some(json!({"__secret": "data", "visible": "ok"}))`
- Read: `__secret` stripped (starts with `__`)
- **Assert**: returned metadata is `{"visible": "ok"}` — data loss is silent

#### TC-META-07: Single underscore key `_myField` preserved [P2]
- `metadata_schema: Some(json!({"_myField": "value"}))`
- **Assert**: round-trip returns `{"_myField": "value"}` (NOT stripped — only `__` prefix)

#### TC-META-08: Type metadata_schema=None → stored with only __can_be_root [P2]
- Create type with `metadata_schema: None`
- **DB assert**: stored JSONB = `{"__can_be_root": true}` (or false)
- **API assert**: returned `metadata_schema` = None (stripped to empty → None)

#### TC-META-09: can_be_root derived from stored __can_be_root [P1]
- Create type can_be_root=true → get_type → `can_be_root == true`
- Create type can_be_root=false (with parents) → get_type → `can_be_root == false`
- **DB assert**: `__can_be_root` in stored JSONB matches

#### TC-META-10: can_be_root fallback when __can_be_root missing from stored JSON [P1]
- Manually insert gts_type row with `metadata_schema = '{}'` (no __can_be_root key)
- Type has allowed_parents → `can_be_root = false` (fallback: `allowed_parents.is_empty()`)
- Type has no allowed_parents → `can_be_root = true`

#### TC-META-11: metadata_schema not validated as JSON Schema [P2]
- Feature 0002 says "validate it is valid JSON Schema" (inst-val-input-7)
- Code does NOT validate — any JSON value accepted
- `metadata_schema: Some(json!({"not": "a valid json schema at all"}))` → succeeds
- **Assert**: Verify no validation (document gap vs requirement)

#### D. Attack Vectors on `metadata_schema`

These tests verify the system is resilient to adversarial metadata_schema payloads. The key attack surface is `build_stored_schema()` which clones user input into the storage JSONB.

#### TC-META-ATK-01: Overwrite `__can_be_root` via metadata_schema to escalate privileges [P1]
- Create type with `can_be_root: false, allowed_parents: [P], metadata_schema: {"__can_be_root": true}`
- **Attack**: user tries to force `can_be_root=true` via injected internal key
- **Assert**: `get_type().can_be_root == false` (system wins, user's `__can_be_root` overwritten by `build_stored_schema`)

#### TC-META-ATK-02: Inject `__can_be_root` with non-boolean value [P1]
- `metadata_schema: {"__can_be_root": "maybe", "x": 1}`
- `build_stored_schema` overwrites with `Bool(can_be_root)` → no issue
- But what if stored JSONB was manually corrupted to `{"__can_be_root": "not-a-bool"}`?
- `load_full_type` calls `.as_bool()` → `None` → fallback to `allowed_parents.is_empty()`
- **Assert**: Verify fallback works, no panic

#### TC-META-ATK-03: Inject multiple `__` prefixed keys to pollute internal storage [P1]
- `metadata_schema: {"__can_be_root": false, "__internal_flag": true, "__secret": "admin"}`
- **Assert**: After round-trip, user gets back only non-`__` keys. Internal keys don't accumulate across updates.

#### TC-META-ATK-04: Huge metadata_schema payload (DoS via JSONB size) [P1]
- `metadata_schema: Some(json!({"x": "A".repeat(1_000_000)}))` — 1MB payload
- **Assert**: Verify behavior — does DB accept it? Is there a size limit? If not, document as risk.

#### TC-META-ATK-05: Deeply nested metadata_schema (stack overflow / parse bomb) [P1]
- `metadata_schema` with 100+ levels of nesting: `{"a": {"a": {"a": ...}}}`
- **Assert**: No panic, no stack overflow in serde/sea-orm

#### TC-META-ATK-06: metadata_schema with special JSON values [P2]
- `metadata_schema: Some(json!({"nan": f64::NAN}))` — NaN is not valid JSON
- `metadata_schema: Some(json!(null))` — top-level null
- `metadata_schema: Some(json!(true))` — top-level boolean
- **Assert**: Each case either rejected or handled gracefully (no panic, no corrupt storage)

#### TC-META-ATK-07: metadata_schema with keys that conflict with SeaORM/SQL [P2]
- Keys like `"id"`, `"schema_id"`, `"gts_type_id"`, `"tenant_id"` in metadata_schema
- **Assert**: No column collision — JSONB is isolated from relational columns

#### TC-META-ATK-08: Group metadata with SQL/NoSQL injection payloads [P1]
- `metadata: Some(json!({"barrier": "'; DROP TABLE resource_group; --"}))`
- `metadata: Some(json!({"$where": "this.admin==true"}))`
- **Assert**: Stored/returned as-is (JSONB is opaque), no SQL injection. Verify via DB query.

#### TC-META-ATK-09: Group metadata with very large payload [P1]
- `metadata: Some(json!({"data": "X".repeat(1_000_000)}))`
- **Assert**: Same as ATK-04 — verify size limits or document risk

#### TC-META-ATK-10: Update type metadata_schema — verify old internal keys don't leak [P1]
- Create type with `metadata_schema: {"v1": "old"}`
- Update type with `metadata_schema: {"v2": "new"}`
- **Assert**: Stored JSONB fully replaced (no merge of old+new). `get_type` returns only `{"v2": "new"}`.
- **DB assert**: old `v1` key not present in stored JSONB

#### TC-META-ATK-11: Concurrent metadata updates don't merge [P2]
- Two updates to same type with different metadata_schema
- **Assert**: Last write wins, no partial merge

#### B. Group `metadata` — Barrier as Data

**File**: `group_service_test.rs` (service-level), `api_rest_test.rs` (REST-level)

#### TC-META-12: Group with metadata barrier stored and returned [P1]
- Create group with `metadata: Some(json!({"barrier": true}))`
- Get group → `metadata.barrier == true`
- **Covers**: PRD 3.4, Feature 0005-AC "barrier as data"
- **DB assert**: `resource_group.metadata` JSONB column contains `{"barrier": true}`

#### TC-META-13: Group with rich metadata — multiple fields [P1]
- `metadata: Some(json!({"barrier": true, "label": "Partner", "category": "premium"}))`
- **Assert**: all fields preserved in round-trip

#### TC-META-14: Group metadata update replaces entirely (not merge) [P1]
- Create group with `metadata: {"a": 1, "b": 2}`, update with `metadata: {"c": 3}`
- **Assert**: `metadata == {"c": 3}`, old keys gone
- **DB assert**: confirm in `resource_group` table

#### TC-META-15: Group metadata None → update with metadata → get returns metadata [P2]
- Create with None, update with `{"barrier": false}`, get → `{"barrier": false}`

#### TC-META-16: Group metadata set → update with None → metadata gone [P2]
- Create with `{"x": 1}`, update with `metadata: None`
- **Assert**: get returns metadata = None, JSON response has no `metadata` key

#### TC-META-17: Barrier group visible in hierarchy (RG does NOT filter) [P1]
- Create parent → child with `metadata: {"barrier": true}` → grandchild
- `list_group_hierarchy(parent)` → returns ALL 3 including barrier child
- **Covers**: PRD "RG does not filter based on barrier", Feature 0005-AC
- **Assert**: barrier group present in results, depth correct

#### TC-META-18: Group metadata in hierarchy endpoint response [P1]
- Create groups with various metadata
- `list_group_hierarchy` → each `GroupWithDepthDto` includes `metadata` field
- **Assert**: metadata preserved in hierarchy response (Feature 0005 requirement)

#### C. REST-level metadata serialization

**File**: `api_rest_test.rs`

#### TC-META-19: REST create type with metadataSchema (camelCase in JSON) [P1]
- POST body: `{"code": "...", "canBeRoot": true, "metadataSchema": {"type": "object"}}`
- **Assert**: 201, response body has `"metadataSchema"` key (camelCase)

#### TC-META-20: REST create group with metadata in body [P1]
- POST body: `{"type": "...", "name": "X", "metadata": {"barrier": true}}`
- **Assert**: 201, response body has `"metadata": {"barrier": true}`

#### TC-META-21: REST response omits metadata when null [P2]
- Create group without metadata
- **Assert**: JSON response does NOT contain `"metadata"` key (via `skip_serializing_if`)

#### TC-META-22: REST response omits metadataSchema when null [P2]
- Create type without metadata_schema
- **Assert**: JSON response does NOT contain `"metadataSchema"` key

---

### 3.10 Invalid / Non-GTS Input Tests

**ONE existing test** checks invalid type code: `create_type_invalid_code_returns_400` with `"code": "invalid"`. Nothing else.

**File**: `api_rest_test.rs` (REST-level for deserialization), `type_service_test.rs` / `group_service_test.rs` (service-level for domain validation)

#### Type code / type_path — wrong GTS format:

#### TC-NOGTS-01: Create type with valid GTS path but NOT RG prefix [P1]
- `code: "gts.x.core.user.v1~"` — valid GTS format, but missing `system.rg.type.v1~` prefix
- **Assert**: 400 Validation ("must start with prefix")

#### TC-NOGTS-02: Create type with empty code [P1]
- `code: ""`
- **Assert**: 400 ("must not be empty")

#### TC-NOGTS-03: Create type with completely garbage code [P2]
- `code: "'; DROP TABLE gts_type; --"` (SQL injection attempt)
- **Assert**: 400 Validation (wrong prefix) — no SQL injection

#### TC-NOGTS-04: Create group with non-RG type_path [P1]
- `type: "gts.x.core.user.v1~"` — valid GTS but not RG type
- **Assert**: 400 Validation ("must start with prefix")

#### TC-NOGTS-05: Create group with empty type_path [P1]
- `type: ""`
- **Assert**: 400 ("must not be empty")

#### TC-NOGTS-06: Membership with non-GTS resource_type [P1]
- POST `/memberships/{group_id}/not.a.gts.path/res-1`
- **Assert**: 400 Validation ("Unknown resource type") — resolve_id returns None

#### TC-NOGTS-07: Membership with empty resource_type [P2]
- POST `/memberships/{group_id}//res-1` or empty segment
- **Assert**: 404 (route mismatch) or 400

#### REST deserialization errors — wrong JSON types:

#### TC-DESER-01: Create type with `code: 123` (number not string) [P1]
- **Assert**: 400/422 — Axum JSON deserialization error before handler

#### TC-DESER-02: Create type with `can_be_root: "yes"` (string not bool) [P1]
- **Assert**: 400/422

#### TC-DESER-03: Create type with `can_be_root` missing [P1]
- Body: `{"code": "gts.x.system.rg.type.v1~test.v1~"}`
- **Assert**: 400/422 (required field missing — no `#[serde(default)]` on `can_be_root`)

#### TC-DESER-04: Create group with `type` field missing [P1]
- Body: `{"name": "X"}`
- **Assert**: 400/422

#### TC-DESER-05: Create group with `parent_id: "not-a-uuid"` [P2]
- **Assert**: 400/422 (UUID parse failure)

#### TC-DESER-06: Malformed JSON body [P1]
- Body: `{bad json}`
- **Assert**: 400

#### TC-DESER-07: Empty body when body expected [P1]
- POST /types with empty body
- **Assert**: 400/422

#### TC-DESER-08: Create group with `name: ""` (empty string) [P1]
- Deserialization succeeds, but `validate_name` catches it
- **Assert**: 400 Validation ("between 1 and 255 characters")

#### TC-DESER-09: Group path `group_id` not a UUID [P2]
- GET `/groups/not-a-uuid`
- **Assert**: 400 (Path parameter parse failure)

#### TC-DESER-10: Membership path `group_id` not a UUID [P2]
- POST `/memberships/not-a-uuid/type/res`
- **Assert**: 400

#### TC-DESER-11: Extra unknown fields in body [P3]
- `{"code": "...", "can_be_root": true, "unknown_field": 42}`
- **Assert**: Verify behavior — serde default is ignore (200) or reject?

---

### 3.11 ADR-001 GTS Type System — RG-Level Validation of metadata Values

**ADR-001 reference**: `rg_gts_type_system_tests.rs` (33 tests in types-registry) validates metadata field types, lengths, and unknown fields at GTS level. But **RG module does NOT call GTS validation on group create/update** — metadata is stored as-is in JSONB.

These tests document the **actual RG behavior** vs ADR expectations and verify that invalid metadata values pass through RG untouched (policy-agnostic storage).

**File**: `group_service_test.rs` + `api_rest_test.rs`

#### E. ADR-001 Hierarchy Reproduction in RG Module

Reproduce the full ADR example hierarchy (T1→D2→B3, T7→D8, T9) with correct types, parents, and metadata — entirely through RG service layer.

#### TC-ADR-01: Full ADR hierarchy with types, groups, and memberships [P1]
- **Setup**: Create all RG types (tenant, department, branch + user/course as membership types) via TypeService. Create groups T1, D2, B3, T7, D8, T9 via GroupService with correct parent-child and metadata. Add memberships (user in T1, user in D2, course in B3).
- **Assert per group**:
  - T1: root tenant, `parent_id=None`, `metadata: None`
  - D2: dept under T1, `metadata: {category: "finance", short_description: "Mega Department"}`
  - B3: branch under D2, `metadata: {location: "Building A, Floor 3"}`
  - T7: barrier tenant under T1, `metadata: {barrier: true}`
  - D8: dept under T7, `metadata: {category: "hr"}`
  - T9: root tenant, `metadata: {custom_domain: "t9.example.com"}`
- **Closure assert**: full hierarchy depths correct
- **Membership assert**: each membership group_id + resource_type correct

#### TC-ADR-02: Tenant type allows self-nesting (T7 under T1) [P1]
- Tenant type: `allowed_parents: [tenant_type_code]` — self-referential
- Create T1 (root), create T7 under T1 (both tenant type)
- **Assert**: Success, T7 parent_id = T1.id

#### TC-ADR-03: Department cannot be root [P1]
- Department type: `can_be_root: false, allowed_parents: [tenant_type_code]`
- Try to create department with parent_id=None
- **Assert**: `DomainError::InvalidParentType("cannot be a root group")`

#### TC-ADR-04: Branch only under department (not under tenant) [P1]
- Branch type: `allowed_parents: [department_type_code]`
- Try to create branch directly under tenant
- **Assert**: `DomainError::InvalidParentType`

#### TC-ADR-05: Branch allows users AND courses as members [P1]
- Branch type: `allowed_memberships: [user_type, course_type]`
- Add user membership to branch → success
- Add course membership to branch → success
- **Assert**: both memberships exist

#### TC-ADR-06: Tenant allows only users as members (not courses) [P1]
- Tenant type: `allowed_memberships: [user_type]` (no course)
- Add user to tenant → success
- Add course to tenant → **DomainError::Validation("not in allowed_memberships")**

#### TC-ADR-07: Same resource (user) in multiple groups with different group_ids [P1]
- Per ADR: "R8 appears twice: as user in D8 and T7"
- Add user R8 to D8, add user R8 to T7 (same tenant)
- **Assert**: both memberships succeed

#### TC-ADR-08: Same resource (R4) with different types in different groups [P1]
- Per ADR: "R4 as course in B3 and as user in T1"
- Add R4 with type=course to B3, add R4 with type=user to T1
- **Assert**: both succeed (different gts_type_id, same resource_id)

#### F. metadata Validation Against Type's metadata_schema

Per ADR-001, each chained RG type defines a `metadata_schema` with `additionalProperties: false`, field types, and length constraints. **RG MUST validate group metadata against the type's metadata_schema on create/update.**

GTS-level validation (33 tests in `rg_gts_type_system_tests.rs`) validates at schema registration time. These unit tests verify the **runtime** validation path: when a caller creates/updates a group, RG checks the `metadata` payload against the stored `metadata_schema` for the group's type.

> **Note**: As of current implementation, this validation is **missing** in code — `group_service.rs` stores metadata as-is without validation. These tests will initially fail and serve as acceptance criteria for implementing the validation.
>
> **Implementation**: Use `TypesRegistryClient` (types-registry-sdk, already used by `credstore` module) + `gts` crate (v0.8.4, already in workspace). The GTS type system validates instance data (including `metadata` sub-object) against the chained RG type schema registered in types-registry. RG module should resolve the group's GTS type via `TypesRegistryClient`, then validate the incoming metadata against the type's inline `metadata` schema (which includes `additionalProperties: false`, field types, `maxLength`). This follows the same pattern as `credstore` module which uses `TypesRegistryClient` from ClientHub for GTS-level validation. Do NOT use raw `jsonschema` crate directly — validation must go through the GTS layer to respect `x-gts-traits`, `allOf` composition, and the metadata sub-object schema.

##### Tenant metadata (`barrier: boolean`, `custom_domain: hostname`)

#### TC-ADR-09: Tenant — valid metadata.barrier=true accepted [P1]
- Create tenant group with `metadata: {"barrier": true}`
- **Assert**: 201 success

#### TC-ADR-10: Tenant — barrier wrong type (string) rejected [P1]
- Create tenant group with `metadata: {"barrier": "yes"}`
- **Assert**: 400 Validation error — `barrier` must be boolean

#### TC-ADR-11: Tenant — barrier wrong type (number) rejected [P1]
- `metadata: {"barrier": 42}`
- **Assert**: 400

#### TC-ADR-12: Tenant — unknown metadata field rejected [P1]
- `metadata: {"barrier": true, "foo": "bar"}`
- **Assert**: 400 — `additionalProperties: false` rejects unknown fields

#### TC-ADR-13: Tenant — valid custom_domain accepted [P1]
- `metadata: {"custom_domain": "t9.example.com"}`
- **Assert**: 201

#### TC-ADR-14: Tenant — custom_domain wrong type (number) rejected [P2]
- `metadata: {"custom_domain": 123}`
- **Assert**: 400

#### TC-ADR-15: Tenant — empty metadata accepted (all fields optional) [P1]
- `metadata: {}`
- **Assert**: 201 — no required fields in tenant metadata schema

#### TC-ADR-16: Tenant — metadata=null accepted [P2]
- `metadata: null` or field absent
- **Assert**: 201 — metadata is optional

##### Department metadata (`category: maxLength 100`, `short_description: maxLength 500`)

#### TC-ADR-17: Department — category within limit accepted [P1]
- `metadata: {"category": "finance"}` (7 chars, ≤ 100)
- **Assert**: 201

#### TC-ADR-18: Department — category at boundary (100 chars) accepted [P1]
- `metadata: {"category": "X".repeat(100)}`
- **Assert**: 201

#### TC-ADR-19: Department — category over limit (101 chars) rejected [P1]
- `metadata: {"category": "X".repeat(101)}`
- **Assert**: 400 — maxLength: 100 violated

#### TC-ADR-20: Department — short_description within limit (500 chars) accepted [P1]
- `metadata: {"short_description": "X".repeat(500)}`
- **Assert**: 201

#### TC-ADR-21: Department — short_description over limit (501 chars) rejected [P1]
- `metadata: {"short_description": "X".repeat(501)}`
- **Assert**: 400 — maxLength: 500 violated

#### TC-ADR-22: Department — unknown field rejected [P1]
- `metadata: {"category": "hr", "short_description2": "typo"}`
- **Assert**: 400 — `short_description2` not in schema, `additionalProperties: false`

#### TC-ADR-23: Department — wrong value type for category (bool not string) [P1]
- `metadata: {"category": false}`
- **Assert**: 400

##### Branch metadata (`location: string`, no maxLength)

#### TC-ADR-24: Branch — valid location accepted [P1]
- `metadata: {"location": "Building A, Floor 3"}`
- **Assert**: 201

#### TC-ADR-25: Branch — unknown field rejected [P1]
- `metadata: {"location": "ok", "unknown_field": true}`
- **Assert**: 400

#### TC-ADR-26: Branch — location wrong type (number) rejected [P2]
- `metadata: {"location": 42}`
- **Assert**: 400

##### Cross-type metadata isolation

#### TC-ADR-27: Tenant metadata fields on department → rejected [P1]
- Create department group with `metadata: {"barrier": true}`
- Department schema does NOT have `barrier` → `additionalProperties: false` rejects
- **Assert**: 400

#### TC-ADR-28: Department metadata fields on tenant → rejected [P1]
- Create tenant group with `metadata: {"category": "finance"}`
- Tenant schema does NOT have `category` → rejected
- **Assert**: 400

##### Update metadata validation

#### TC-ADR-29: Update group metadata — same validation rules apply [P1]
- Create department with valid metadata `{"category": "hr"}`
- Update with `metadata: {"category": "X".repeat(101)}` → 400 (over limit)
- Update with `metadata: {"category": "finance"}` → 200 (valid)

#### TC-ADR-30: Type without metadata_schema — any metadata accepted [P2]
- Create type with `metadata_schema: None`
- Create group with `metadata: {"anything": "goes", "x": 42}`
- **Assert**: 201 — no schema means no validation

#### TC-ADR-31: Update type metadata_schema — existing groups NOT retroactively validated [P2]
- Create type with permissive metadata_schema, create group with metadata
- Update type with stricter metadata_schema (adds maxLength)
- **Assert**: existing group still readable. New groups validated against new schema.

#### TC-ADR-15: metadata_schema round-trip with ADR tenant schema [P1]
- Create tenant RG type with metadata_schema from ADR (barrier: boolean, custom_domain: hostname)
- Get type → metadata_schema returned correctly (no `__can_be_root`, no `__user_schema`)
- **Assert**: metadata_schema matches input

#### TC-ADR-16: Chained type path format in RG [P1]
- ADR uses: `gts.x.core.rg.type.v1~y.core.tn.tenant.v1~` (multi-segment)
- Code validates prefix: `gts.x.system.rg.type.v1~` (different namespace!)
- **Assert**: Verify which prefix the code actually requires. If `system` not `core` → document discrepancy with ADR.

#### G. SMALLINT Non-Exposure (ADR Confirmation requirement)

ADR Confirmation: "Code review: verify all API responses use GTS type paths, never SMALLINT IDs"

#### TC-ADR-17: Type response contains no SMALLINT IDs [P1]
- Create type, GET → response JSON
- **Assert**: no `gts_type_id`, `type_id`, `parent_type_id` numeric fields. `code`, `allowed_parents`, `allowed_memberships` are all strings.

#### TC-ADR-18: Group response contains no SMALLINT IDs [P1]
- Create group, GET → response JSON
- **Assert**: `type` field is string GTS path. No `gts_type_id`.

#### TC-ADR-19: Membership response contains no SMALLINT IDs [P1]
- Add membership, response JSON
- **Assert**: `resource_type` is string GTS path. No `gts_type_id`.

#### TC-ADR-20: Hierarchy response contains no SMALLINT IDs [P1]
- list_group_hierarchy → response items
- **Assert**: each item `type` is string, no numeric type IDs

---

### 3.12 GTS-Specific Logic Tests

**ZERO** existing tests exercise GTS path resolution, roundtrip ID↔String, or metadata internal key handling in isolation.

#### GTS Path ↔ ID Resolution

**File**: `type_service_test.rs` or `group_service_test.rs` (service-level with DB)

#### TC-GTS-01: resolve_id returns SMALLINT for existing type [P1]
- Create type, verify `resolve_id(code)` returns `Some(id)` where id is `i16`

#### TC-GTS-02: resolve_id returns None for nonexistent path [P1]
- `resolve_id("gts.x.system.rg.type.v1~nonexistent.v1~")` → `None`

#### TC-GTS-03: resolve_ids batch — all found [P1]
- Create 3 types, `resolve_ids([code1, code2, code3])` → `Ok(vec![id1, id2, id3])`
- **Assert**: returned IDs match, order may differ

#### TC-GTS-04: resolve_ids batch — some missing [P1]
- Create type A, resolve_ids([A, "nonexistent"]) → `Err(Validation("Referenced types not found: nonexistent"))`
- **Assert**: error message lists ALL missing codes

#### TC-GTS-05: resolve_ids batch — multiple missing [P2]
- resolve_ids(["missing1", "missing2"]) → error message contains both

#### TC-GTS-06: resolve_ids empty list [P2]
- `resolve_ids([])` → `Ok(vec![])` (early return)

#### TC-GTS-07: Full roundtrip: create type → resolve_id → resolve_type_path_from_id [P1]
- Create type with code X, resolve to ID, resolve back to path
- **Assert**: returned path == X (exact string equality)

#### TC-GTS-08: load_allowed_parents resolves junction → IDs → paths [P1]
- Create parent type P, child type C(allowed_parents=[P])
- load_allowed_parents(C.id) → `vec!["gts.x...P..."]`
- **Assert**: returned path == P's code

#### TC-GTS-09: load_allowed_memberships resolves junction → IDs → paths [P1]
- Create member type M, group type G(allowed_memberships=[M])
- load_allowed_memberships(G.id) → `vec!["gts.x...M..."]`

#### can_be_root Derivation & Internal Key Handling

**File**: `type_service_test.rs` (service-level), or `type_repo.rs` in-source if repo functions are pub

#### TC-GTS-10: can_be_root derived from stored __can_be_root key [P1]
- Create type with can_be_root=true, get_type → `can_be_root == true`
- Create type with can_be_root=false (with parents), get_type → `can_be_root == false`
- **Verify via DB**: `__can_be_root` key in stored JSONB matches

#### TC-GTS-11: can_be_root fallback when __can_be_root key missing [P1]
- Manually insert row in gts_type with metadata_schema without `__can_be_root` key
- load_full_type → `can_be_root` should default to `allowed_parents.is_empty()`
- **Scenario**: type with parents → false; type without parents → true

#### TC-GTS-12: Internal keys stripped from metadata_schema response [P1]
- Create type with `metadata_schema: {"myField": "value"}`
- Stored JSONB will have `{"myField": "value", "__can_be_root": true}`
- get_type → returned metadata_schema is `{"myField": "value"}` (no `__can_be_root`)

#### TC-GTS-13: User key with __ prefix silently stripped [P1]
- Create type with `metadata_schema: {"__custom": "data", "normal": "ok"}`
- get_type → returned metadata_schema is `{"normal": "ok"}` only
- **Document**: double-underscore keys are reserved, silently dropped on read

#### TC-GTS-14: Single underscore key preserved [P2]
- `metadata_schema: {"_my_field": "value"}` → preserved in response

#### TC-GTS-15: metadata_schema=None → __can_be_root still stored, schema returned as None [P2]
- Create type with no metadata_schema
- DB has `{"__can_be_root": true}` → after stripping __ keys → empty object → `None`

#### URL Tilde Encoding for GTS Paths

**File**: `api_rest_test.rs` (REST-level)

#### TC-GTS-16: Membership POST with tilde in resource_type URL path [P1]
- POST `/memberships/{group_id}/gts.x.system.rg.type.v1%7Etest.v1%7E/res-1`
- **Assert**: 201 (Axum Path<String> decodes %7E → ~)

#### TC-GTS-17: Membership DELETE with tilde in resource_type URL path [P1]
- Same encoding pattern, verify 204

#### TC-GTS-18: PUT /types/{code} with tilde encoding [P2]
- PUT `/types-registry/v1/types/gts.x...%7Etest%7E` → 200

#### GTS Path Comparison Consistency

#### TC-GTS-19: allowed_parents.contains() exact string match after roundtrip [P1]
- Create parent type P, child type C(allowed_parents=[P])
- Create group of type P (root), create child group of type C under it
- **Assert**: success — proves the path stored for P matches P's code exactly during comparison

#### TC-GTS-20: validate_type_code vs GtsTypePath length limits differ [P2]
- Domain: `validate_type_code` allows up to 1024 chars
- SDK: `GtsTypePath::new()` allows up to 255 chars
- Create type with 300-char code via service → succeeds (domain limit 1024)
- Wrap same code in GtsTypePath::new() → fails (SDK limit 255)
- **Document**: inconsistency between domain and SDK validation

---

### 3.13 REST API Layer (existing endpoints without tests)

**File**: `api_rest_test.rs` (extend existing)

#### TC-REST-01: Update type PUT returns 200 [P2]
- **Covers**: G34, 0002-AC-7
- **Setup**: Create type via service, PUT with updated body
- **Assert**: 200 OK, body contains updated fields

#### TC-REST-02: Update type not found returns 404 [P2]
- **Covers**: G34

#### TC-REST-03: Add membership POST returns 201 [P2]
- **Covers**: G35, 0004-AC-1
- **Setup**: Create type+group via service, POST membership
- **Assert**: 201 Created

#### TC-REST-04: Remove membership DELETE returns 204 [P2]
- **Covers**: G35, 0004-AC-8

#### TC-REST-05: List memberships GET returns 200 [P2]
- **Covers**: G35, 0004-AC-10

#### TC-REST-06: Create group with parent via REST [P2]
- **Covers**: G8
- POST with parent_id in body, verify 201 + hierarchy fields

#### TC-REST-07: Delete group with force=true via REST [P2]
- **Covers**: G20
- DELETE /groups/{id}?force=true, verify 204

#### TC-REST-08: Hierarchy endpoint via REST [P2]
- **Covers**: G21
- GET /groups/{id}/hierarchy, verify 200 + depth fields

---

## 4. Priority Matrix

### P1 - Critical (must have, business invariants) — 62 tests

These test domain invariants that prevent data corruption or violate core business rules:

| ID | Test Case | Risk if Missing |
|----|-----------|-----------------|
| **TC-TYP-02** | Create type - nonexistent allowed_parents | Dangling type references |
| **TC-TYP-04** | Placement invariant violation | Orphan types that can't be placed |
| **TC-TYP-06** | Update type - remove parent in use | Breaks existing group hierarchy |
| **TC-TYP-07** | Update type - can_be_root=false with roots | Orphans existing root groups |
| **TC-TYP-09** | Delete type with groups | Cascading data loss |
| **TC-GRP-01** | Child group + closure rows | Hierarchy queries broken |
| **TC-GRP-02** | 3-level closure completeness | Ancestor/descendant queries wrong |
| **TC-GRP-03** | Incompatible parent type | Type system bypassed |
| **TC-GRP-04** | Root group when can_be_root=false | Type system bypassed |
| **TC-GRP-05** | Move with closure rebuild | Hierarchy corrupt after move |
| **TC-GRP-06** | Move under descendant -> cycle | Infinite loops in hierarchy |
| **TC-GRP-07** | Self-parent -> cycle | Infinite loops in hierarchy |
| **TC-GRP-08** | Move to incompatible type | Type system bypassed |
| **TC-GRP-10** | Type change vs parent compat | Type constraints violated |
| **TC-GRP-11** | Type change vs children compat | Children become orphans |
| **TC-GRP-12** | Leaf delete | Data cleanup |
| **TC-GRP-13** | Delete with children no force | Accidental data loss |
| **TC-GRP-14** | Delete with memberships no force | Accidental data loss |
| **TC-GRP-15** | Force delete subtree | Cascade completeness |
| **TC-GRP-16** | Hierarchy depth traversal | Core read feature |
| **TC-GRP-17** | max_depth on create | Profile guardrail |
| **TC-GRP-18** | max_width on create | Profile guardrail |
| **TC-MBR-01** | Add membership happy path | Core write feature |
| **TC-MBR-02** | Add to nonexistent group | Dangling membership |
| **TC-MBR-03** | Duplicate membership | Data integrity |
| **TC-MBR-05** | Not in allowed_memberships | Type system bypassed |
| **TC-MBR-06** | Tenant incompatibility | Cross-tenant data leak |
| **TC-MBR-13** | Empty allowed_memberships rejects all | Type system bypassed |
| **TC-MBR-14** | Same resource in multiple groups same tenant | Multi-group membership broken |
| **TC-GRP-22** | Create group nonexistent type | Group with invalid type |
| **TC-GRP-23** | Child group cross-tenant parent | Tenant isolation broken |
| **TC-GRP-26** | Simultaneous type + parent change | Atomicity of combined update |
| **TC-GRP-27** | Root group → non-root type change | Root groups orphaned |
| **TC-GRP-29** | Move child to root (detach) | Closure corruption on detach |
| **TC-GRP-30** | Move to root when can_be_root=false | Unauthorized root creation |
| **TC-GRP-37** | Depth exact boundary (>=) | Off-by-one in guardrail |
| **TC-GRP-38** | Width exact boundary (>=) | Off-by-one in guardrail |
| **TC-SDK-24** | validate_type_code vs GtsTypePath mismatch | Silent validation inconsistency |
| **TC-SDK-01..06,08** | GtsTypePath value object validation | Invalid types accepted into system |
| **TC-SDK-11,12** | GtsTypePath serde round-trip | API deserialization breaks silently |
| **TC-SDK-14,15** | SDK model camelCase + `type` rename | Wire format mismatch |
| **TC-DTO-05** | CreateGroupDto `type` JSON rename | Request deserialization fails |
| **TC-ODATA-01,02** | GroupFilterField names + kinds | OData $filter silently broken |
| **TC-ODATA-04,05** | Hierarchy + Membership FilterField | OData $filter silently broken |
| **TC-SEED-01..03** | seed_types create/update/skip | Bootstrap data corruption |
| **TC-SEED-05,06** | seed_groups create + skip | Hierarchy bootstrap broken |
| **TC-SEED-07,08** | seed_memberships create + skip | Membership bootstrap broken |
| **TC-ERR-01** | EnforcerError::Denied -> AccessDenied | Auth errors mishandled |

### P2 - Important (error paths, REST layer, edges) — 53 tests

| ID | Area |
|----|------|
| TC-TYP-03, TC-TYP-05, TC-TYP-08, TC-TYP-10..15 | Type management edges + metadata + memberships |
| TC-GRP-09, TC-GRP-19..21, TC-GRP-24..25, TC-GRP-28, TC-GRP-31..36 | Group update/validation/error paths |
| TC-MBR-07..12 | Membership edges + empty resource_id + unregistered type |
| TC-REST-01..08 | REST layer coverage |
| TC-SDK-07, TC-SDK-09, TC-SDK-10, TC-SDK-16..23 | SDK edge cases + boundary |
| TC-DTO-01..04, TC-DTO-06, TC-DTO-07 | DTO conversion |
| TC-ODATA-03, TC-ODATA-06..08 | OData mapper correctness |
| TC-SEED-04, TC-SEED-09, TC-SEED-11, TC-SEED-12 | Seeding edge cases |
| TC-ERR-02..05 | Error conversion chains |
| TC-GRP-33 | max_width on move |

### P3 - Nice to have (boundary, cosmetic) — 4 tests

| ID | Test Case |
|----|-----------|
| TC-SDK-13 | GtsTypePath Display + Into<String> |
| TC-TYP-16 | Hierarchy check skips deleted parent type |
| TC-MBR-15 | List memberships empty result |
| TC-SEED-10 | seed_types empty list |

---

## 5. Assert Guidelines — What to Verify Beyond Ok/Err

Most test cases above describe only the return value assertion. This section defines **mandatory DB-level assertions** that must accompany functional checks.

### 5.1 Closure Table Assertions (`resource_group_closure`)

Every test that creates, moves, or deletes groups **MUST** query the closure table directly and verify:

| Operation | Required DB Assertions |
|-----------|----------------------|
| **Create root group** | Self-row exists `(ancestor=id, descendant=id, depth=0)`. Total closure rows for this group = **1**. |
| **Create child group** | Self-row `(depth=0)` + ancestor rows for every ancestor with correct depth. Total = `depth_in_tree + 1`. |
| **Create 3-level tree** | Total closure rows = **6** (1+2+3). Each depth value verified. |
| **Move subtree** | Old ancestor paths **deleted** (COUNT=0 for old_root→moved_nodes). New ancestor paths created with correct depths. **Internal subtree paths preserved** (child→grandchild depth unchanged). Self-rows untouched. Nodes outside moved subtree **unaffected**. |
| **Move child to root** | All ancestor rows removed. Only self-row remains (COUNT=1). |
| **Delete leaf** | All closure rows WHERE `descendant_id=id OR ancestor_id=id` → **0 rows**. Parent's closure rows **untouched**. |
| **Force delete subtree** | All closure rows for all nodes in subtree → **0**. Nodes outside subtree **unaffected**. |

**Helper function**: `assert_closure_rows(conn, group_id, expected: &[(Uuid, i32)])` — verifies exact set of (ancestor_id, depth) pairs for a given descendant.

### 5.2 Junction Table Assertions (`gts_type_allowed_parent`, `gts_type_allowed_membership`)

| Operation | Required DB Assertions |
|-----------|----------------------|
| **Create type with parents** | Junction rows COUNT = `len(allowed_parents)`. Each `parent_type_id` correctly resolved from GTS path to SMALLINT. |
| **Update type (replace parents)** | Old junction rows **deleted**. New rows match new list. COUNT = `len(new_allowed_parents)`. |
| **Update type (replace memberships)** | `gts_type_allowed_membership` contains only new entries. |
| **Delete type (CASCADE)** | `gts_type_allowed_parent WHERE type_id` → 0. `gts_type_allowed_membership WHERE type_id` → 0. |

### 5.3 Membership Table Assertions (`resource_group_membership`)

| Operation | Required DB Assertions |
|-----------|----------------------|
| **Add membership** | Row exists with correct composite key `(group_id, gts_type_id, resource_id)`. `gts_type_id` is SMALLINT (resolved). |
| **Remove membership** | Row **gone**. Other memberships of same group **untouched**. |
| **Force delete group** | All membership rows for subtree groups → **0**. |
| **Same resource multiple groups** | Two rows with different `group_id`, same `(gts_type_id, resource_id)`. |

### 5.4 Surrogate ID Non-Exposure (REST tests)

Every REST test response **MUST** verify:
- No numeric `gts_type_id`, `type_id`, `parent_type_id` fields in JSON
- `type` / `resource_type` / `allowed_parents` / `allowed_memberships` are **string** GTS paths
- Membership responses have **no** `tenant_id` field

### 5.5 Entity State Assertions (`resource_group` table)

| Operation | Required DB Assertions |
|-----------|----------------------|
| **Create group** | `parent_id`, `gts_type_id`, `tenant_id`, `name`, `metadata` match request. |
| **Update name/metadata** | `name` and `metadata` changed. `parent_id`, `gts_type_id` **unchanged**. |
| **Move group** | `parent_id` updated. `gts_type_id`, `name`, `tenant_id` **unchanged**. |
| **Update type** | `gts_type_id` changed. `parent_id`, `name` **unchanged**. |

### 5.6 Hierarchy Endpoint Response Shape

`list_group_hierarchy(B)` for tree A → B → C **MUST** return:
- Self-node B: `hierarchy.depth == 0`
- Ancestor A: `hierarchy.depth < 0` (e.g., -1)
- Descendant C: `hierarchy.depth > 0` (e.g., 1)
- **All nodes present** (no missing nodes)
- Each node has `hierarchy.tenant_id`, `hierarchy.parent_id`

### 5.7 Seeding DB Verification

| Operation | Required DB Assertions |
|-----------|----------------------|
| **seed_types creates** | Type **physically exists** in `gts_type`. Junction rows present. |
| **seed_types unchanged** | `updated_at` **not modified** on re-run. |
| **seed_groups creates** | Groups exist in `resource_group`. Closure rows correct. `parent_id` FK valid. |
| **seed_memberships creates** | Membership rows exist with correct composite key. |

---

## 6. Test Infrastructure

### Core Principles

1. **Atomic**: each test verifies exactly one behavior. One `#[test]` or `#[tokio::test]` = one scenario. No "also check this while we're here".
2. **Fast**: no `sleep`, no `timeout`, no `tokio::time::*`, no polling, no retries in test code. Target: entire suite < 5s.
3. **Independent**: no shared state between tests. Each test creates its own `SQLite :memory:` DB and fresh service instances. Tests can run in any order and in parallel (`cargo test -j N`).
4. **Synchronous where possible**: pure logic tests (`GtsTypePath`, `FilterField`, DTO conversions, error mapping) are `#[test]`, not `#[tokio::test]`. Async only when DB or service layer is involved.
5. **No retry testing**: the serialization retry loop (`MAX_SERIALIZATION_RETRIES = 3`) is an implementation detail. Tests call service methods that internally use SERIALIZABLE transactions — but tests do NOT simulate contention, do NOT assert retry counts, do NOT use timeouts. The retry loop is transparent to the test.
6. **Direct DB queries for state verification**: use `sea_orm::EntityTrait::find()` directly to inspect table state. Do NOT rely solely on service-layer reads to verify writes (service reads go through AccessScope which may filter).

### Anti-patterns (DO NOT)

```rust
// BAD: timer in test
tokio::time::sleep(Duration::from_millis(100)).await;

// BAD: retry/poll loop
for _ in 0..10 { if check() { break; } sleep(50ms); }

// BAD: testing serialization retry by simulating contention
// (this requires real PostgreSQL concurrent connections — out of scope)

// BAD: compound test
#[tokio::test]
async fn test_everything() {
    // creates type, creates group, moves it, deletes it, checks seeding...
}

// BAD: verifying write via scoped read only
let group = group_svc.get_group(&ctx, id).await?; // goes through AccessScope!
// This does NOT prove the DB state — a scope bug could hide the group

// GOOD: direct DB assertion
use sea_orm::EntityTrait;
let model = resource_group::Entity::find_by_id(id).one(&conn).await?.unwrap();
assert_eq!(model.parent_id, Some(new_parent));

// GOOD: closure assertion helper
let rows = resource_group_closure::Entity::find()
    .filter(resource_group_closure::Column::DescendantId.eq(child_id))
    .all(&conn).await?;
assert_eq!(rows.len(), 2); // self + parent
assert!(rows.iter().any(|r| r.ancestor_id == child_id && r.depth == 0));
assert!(rows.iter().any(|r| r.ancestor_id == parent_id && r.depth == 1));
```

### Assertion & Parameterization Patterns

Follow the established project conventions. **None of the crates below are used in this repository** — do NOT introduce `rstest`, `proptest`, `assert_matches`, or `cool_asserts` as new dependencies.

**Error variant checks** — use `assert!(matches!(...))` with descriptive message (140 usages across 30 files in repo):
```rust
// Project standard pattern
let err = result.unwrap_err();
assert!(
    matches!(err, DomainError::InvalidParentType { .. }),
    "Expected InvalidParentType, got: {err:?}"
);
assert!(err.to_string().contains("does not allow parent type"));
```

**Table-driven tests** — use manual `vec![]` + loop (matches `nodes-registry/tests/error_tests.rs` pattern):
```rust
#[test]
fn domain_errors_map_to_correct_status_codes() {
    let cases: Vec<(DomainError, StatusCode)> = vec![
        (DomainError::type_not_found("x"), StatusCode::NOT_FOUND),
        (DomainError::validation("x"), StatusCode::BAD_REQUEST),
        (DomainError::cycle_detected("x"), StatusCode::CONFLICT),
        // ...
    ];
    for (err, expected_status) in cases {
        let problem: Problem = err.into();
        assert_eq!(problem.status, expected_status, "for error: {problem:?}");
    }
}
```

**GtsTypePath validation** — table-driven with loop (NOT rstest):
```rust
#[test]
fn gts_type_path_valid_cases() {
    let valid = vec![
        "gts.x.system.rg.type.v1~",
        "gts.x.system.rg.type.v1~x.test.v1~",
        "gts.123~456~",
        "gts.a_b.c_d~",
    ];
    for input in valid {
        assert!(GtsTypePath::new(input).is_ok(), "should be valid: {input}");
    }
}

#[test]
fn gts_type_path_invalid_cases() {
    let invalid = vec![
        ("", "empty"),
        ("invalid.path~", "no gts prefix"),
        ("gts.~", "empty segment"),
        ("gts.a-b~", "hyphen not allowed"),
        ("gts.x.system.rg.type.v1", "no trailing tilde"),
    ];
    for (input, reason) in invalid {
        assert!(GtsTypePath::new(input).is_err(), "should reject ({reason}): {input}");
    }
}
```

**Setup helpers** — plain `async fn` in `tests/common/mod.rs` (NOT fixtures):

### Shared Test Helpers (`tests/common/mod.rs`)

Extract duplicated setup code from existing tests:

```rust
// tests/common/mod.rs

/// SQLite in-memory DB with migrations. ~1ms per call.
pub async fn test_db() -> Arc<DBProvider<DbError>> { ... }

/// SecurityContext for given tenant.
pub fn make_ctx(tenant_id: Uuid) -> SecurityContext { ... }

/// AllowAll PolicyEnforcer (returns tenant-scoped AccessScope).
pub fn make_enforcer() -> PolicyEnforcer { ... }

/// Create root type with unique code suffix. One DB round-trip.
pub async fn create_root_type(svc: &TypeService, suffix: &str) -> ResourceGroupType { ... }

/// Create child type referencing parent codes. One DB round-trip.
pub async fn create_child_type(
    svc: &TypeService, suffix: &str, parents: &[&str], memberships: &[&str]
) -> ResourceGroupType { ... }

/// Create root group. Returns ResourceGroup.
pub async fn create_root_group(
    svc: &GroupService, ctx: &SecurityContext, type_code: &str, name: &str, tenant_id: Uuid
) -> ResourceGroup { ... }

/// Create child group under parent.
pub async fn create_child_group(
    svc: &GroupService, ctx: &SecurityContext, type_code: &str, parent_id: Uuid, name: &str, tenant_id: Uuid
) -> ResourceGroup { ... }

/// Assert exact closure rows for a descendant. Panics with diff on mismatch.
pub async fn assert_closure_rows(
    conn: &impl DBRunner, descendant_id: Uuid, expected: &[(Uuid, i32)]  // (ancestor_id, depth)
) { ... }

/// Assert total closure row count for a set of groups (no extra rows left).
pub async fn assert_closure_count(conn: &impl DBRunner, group_ids: &[Uuid], expected_total: usize) { ... }

/// Assert junction table rows for a type.
pub async fn assert_allowed_parents(conn: &impl DBRunner, type_id: i16, expected_parent_ids: &[i16]) { ... }

/// Assert no SMALLINT IDs in JSON value (recursive check).
pub fn assert_no_surrogate_ids(json: &serde_json::Value) { ... }
```

### Naming Convention

Tests follow existing pattern: `{area}_{scenario}` in snake_case, e.g.:
- `type_update_remove_parent_in_use_returns_violation`
- `group_move_under_descendant_returns_cycle_detected`
- `membership_add_duplicate_returns_conflict`
- `closure_correct_after_3_level_create`
- `gts_type_path_rejects_empty_segment`

### Test File Organization

```
# In-source unit tests (pure logic, no DB, #[test] only — instant)
resource-group-sdk/src/models.rs              # (ADD #[cfg(test)]) TC-SDK-01..24
resource-group-sdk/src/odata/groups.rs        # (ADD #[cfg(test)]) TC-ODATA-01..03
resource-group-sdk/src/odata/hierarchy.rs     # (ADD #[cfg(test)]) TC-ODATA-04
resource-group-sdk/src/odata/memberships.rs   # (ADD #[cfg(test)]) TC-ODATA-05
resource-group/src/api/rest/auth.rs           # (EXISTING) 12 tests
resource-group/src/api/rest/dto.rs            # (ADD #[cfg(test)]) TC-DTO-01..07
resource-group/src/infra/storage/odata_mapper.rs  # (ADD #[cfg(test)]) TC-ODATA-06..08

# Integration tests (SQLite :memory: DB, #[tokio::test])
tests/
  common/mod.rs                  # (NEW) shared helpers + assertion helpers
  domain_unit_test.rs            # (EXTEND) TC-ERR-01..05
  api_rest_test.rs               # (EXTEND) TC-REST-01..08
  authz_integration_test.rs      # (existing)
  tenant_filtering_db_test.rs    # (existing)
  tenant_scoping_test.rs         # (existing)
  type_service_test.rs           # (NEW) TC-TYP-01..16
  group_service_test.rs          # (NEW) TC-GRP-01..38
  membership_service_test.rs     # (NEW) TC-MBR-01..15
  seeding_test.rs                # (NEW) TC-SEED-01..12
```

---

## 7. Definitions of Done

### SDK Value Object & Model Tests

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-testing-sdk-models`

In-source `#[cfg(test)]` tests in `resource-group-sdk/src/models.rs`:
- `GtsTypePath::new()` validation (empty, too long, invalid format, whitespace/case normalization, multi-segment paths)
- `GtsTypePath` serde round-trip (JSON serialize/deserialize, invalid input rejection via TryFrom)
- SDK model serialization shape (camelCase, `type` rename, optional field omission)
- `QueryProfile::default()` values

### Unit Test Coverage for Type Management

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-testing-type-mgmt`

All acceptance criteria from feature 0002 are covered by automated tests:
- Create with valid/invalid `allowed_parents` and `allowed_memberships`
- Placement invariant enforcement
- Update with hierarchy safety checks (removed parent in use, can_be_root toggle)
- Delete with active group references blocked

### Unit Test Coverage for Entity Hierarchy

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-testing-entity-hierarchy`

All acceptance criteria from feature 0003 are covered by automated tests:
- Child group creation with closure table verification
- Move operations with cycle detection and closure rebuild
- Type compatibility on create/move/update
- Query profile enforcement (max_depth, max_width)
- Delete with reference checks and force cascade
- Hierarchy depth endpoint with correct relative depths

### Unit Test Coverage for Membership

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-testing-membership`

All acceptance criteria from feature 0004 are covered by automated tests:
- Add/remove lifecycle with composite key semantics
- allowed_memberships validation
- Tenant compatibility enforcement
- Duplicate detection

### OData Filter & DTO Tests

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-testing-odata-dto`

In-source `#[cfg(test)]` tests for filter field definitions and DTO conversions:
- `GroupFilterField`, `HierarchyFilterField`, `MembershipFilterField` name() and kind() correctness
- OData mapper field-to-column mapping (Type/Group/Membership mappers)
- DTO `From` conversions (domain model -> DTO and DTO -> request)
- DTO serde attributes (`type` rename, `default` vectors, optional field omission)

### Seeding Tests

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-testing-seeding`

Integration tests for deployment bootstrapping:
- `seed_types`: create/update/skip idempotency with SeedResult tracking
- `seed_groups`: ordered hierarchy creation with closure table verification
- `seed_memberships`: create + Conflict/TenantIncompatibility skip handling

### Error Conversion Chain Tests

- [x] `p2` - **ID**: `cpt-cf-resource-group-dod-testing-error-conversions`

Extend `domain_unit_test.rs` with FROM-direction error conversions:
- `EnforcerError` (Denied, EvaluationFailed, CompileFailed) -> `DomainError::AccessDenied`
- `sea_orm::DbErr` -> `DomainError::Database`
- `modkit_db::DbError` -> `DomainError::Database`

### REST API Test Coverage

- [x] `p2` - **ID**: `cpt-cf-resource-group-dod-testing-rest-api`

REST-level tests for endpoints not covered by existing `api_rest_test.rs`:
- PUT /types/{code} (update type)
- POST/DELETE /memberships/{group_id}/{type}/{resource_id}
- GET /groups/{id}/hierarchy
- DELETE /groups/{id}?force=true

## Acceptance Criteria

- [x] All ~140 unit tests pass (`cargo test -p cf-resource-group -p cf-resource-group-sdk`) — 291 tests, 0 failed
- [x] Full suite completes in < 5 seconds
- [x] Zero `sleep`, `timeout`, or `tokio::time` usage in tests
- [x] Every domain invariant from features 0001-0005 is covered by at least one test
- [x] `make fmt && make lint && make test` passes with zero errors
