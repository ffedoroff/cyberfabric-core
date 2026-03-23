//! Integration tests: `AccessScope` tenant scoping for resource-group queries.
//!
//! Verifies that `AccessScope` constructed from `AuthZ` decisions correctly
//! represents tenant isolation -- the building block for `SecureORM` filtering.
//! These tests validate the scope shape without a database; the full
//! `SecureORM` -> SQL path is covered by E2E tests.

use uuid::Uuid;

use modkit_security::{AccessScope, pep_properties};

// ── AccessScope construction from tenant context ────────────────────────

/// `AccessScope::for_tenant()` produces a scope that contains exactly
/// the given `tenant_id` under `owner_tenant_id`.
#[test]
fn for_tenant_contains_tenant_id() {
    let tid = Uuid::now_v7();
    let scope = AccessScope::for_tenant(tid);

    assert!(!scope.is_unconstrained());
    assert!(scope.contains_uuid(pep_properties::OWNER_TENANT_ID, tid));
}

/// `AccessScope::for_tenant()` does NOT contain a different tenant.
#[test]
fn for_tenant_excludes_other_tenants() {
    let tid = Uuid::now_v7();
    let other = Uuid::now_v7();
    let scope = AccessScope::for_tenant(tid);

    assert!(!scope.contains_uuid(pep_properties::OWNER_TENANT_ID, other));
}

/// `AccessScope::for_tenants()` with multiple IDs contains all of them.
#[test]
fn for_tenants_contains_all_given_ids() {
    let t1 = Uuid::now_v7();
    let t2 = Uuid::now_v7();
    let t3 = Uuid::now_v7();
    let scope = AccessScope::for_tenants(vec![t1, t2, t3]);

    assert!(scope.contains_uuid(pep_properties::OWNER_TENANT_ID, t1));
    assert!(scope.contains_uuid(pep_properties::OWNER_TENANT_ID, t2));
    assert!(scope.contains_uuid(pep_properties::OWNER_TENANT_ID, t3));
}

/// `all_uuid_values_for()` extracts all tenant IDs from a scope.
#[test]
fn all_uuid_values_extracts_tenant_ids() {
    let t1 = Uuid::now_v7();
    let t2 = Uuid::now_v7();
    let scope = AccessScope::for_tenants(vec![t1, t2]);

    let ids = scope.all_uuid_values_for(pep_properties::OWNER_TENANT_ID);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&t1));
    assert!(ids.contains(&t2));
}

/// `allow_all()` scope is unconstrained — no tenant filtering.
#[test]
fn allow_all_is_unconstrained() {
    let scope = AccessScope::allow_all();
    assert!(scope.is_unconstrained());
}

/// `deny_all()` scope is not unconstrained and contains no values.
#[test]
fn deny_all_has_no_values() {
    let scope = AccessScope::deny_all();
    assert!(!scope.is_unconstrained());
    assert!(
        scope
            .all_uuid_values_for(pep_properties::OWNER_TENANT_ID)
            .is_empty()
    );
}

// ── tenant_only() helper ────────────────────────────────────────────────

/// `tenant_only()` on a tenant scope keeps the tenant filter.
#[test]
fn tenant_only_preserves_tenant_filter() {
    let tid = Uuid::now_v7();
    let scope = AccessScope::for_tenant(tid).tenant_only();

    assert!(scope.contains_uuid(pep_properties::OWNER_TENANT_ID, tid));
}

/// `tenant_only()` on an `allow_all` scope becomes `deny_all` (fail-closed).
/// This is by design: unconstrained scopes have no tenant filters to retain.
#[test]
fn tenant_only_on_allow_all_becomes_deny_all() {
    let scope = AccessScope::allow_all().tenant_only();
    assert!(scope.is_deny_all());
}

// ── Scope combination scenarios ─────────────────────────────────────────

/// Two scopes for different tenants are distinct (no cross-contamination).
#[test]
fn separate_tenant_scopes_are_isolated() {
    let tid_a = Uuid::now_v7();
    let tid_b = Uuid::now_v7();

    let scope_a = AccessScope::for_tenant(tid_a);
    let scope_b = AccessScope::for_tenant(tid_b);

    // A sees only A
    assert!(scope_a.contains_uuid(pep_properties::OWNER_TENANT_ID, tid_a));
    assert!(!scope_a.contains_uuid(pep_properties::OWNER_TENANT_ID, tid_b));

    // B sees only B
    assert!(scope_b.contains_uuid(pep_properties::OWNER_TENANT_ID, tid_b));
    assert!(!scope_b.contains_uuid(pep_properties::OWNER_TENANT_ID, tid_a));
}

/// `for_resource` creates a scope on the `id` property, not `owner_tenant_id`.
#[test]
fn for_resource_scopes_by_id_not_tenant() {
    let resource_id = Uuid::now_v7();
    let scope = AccessScope::for_resource(resource_id);

    assert!(scope.contains_uuid(pep_properties::RESOURCE_ID, resource_id));
    assert!(!scope.contains_uuid(pep_properties::OWNER_TENANT_ID, resource_id));
}
