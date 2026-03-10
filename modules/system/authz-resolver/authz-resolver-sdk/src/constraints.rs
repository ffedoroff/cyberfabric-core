//! Constraint types for authorization decisions.
//!
//! Constraints represent row-level filtering conditions returned by the PDP.
//! They are compiled into `AccessScope` by the PEP compiler.
//!
//! ## Supported predicates
//!
//! - `Eq` — equality (`property = value`)
//! - `In` — set membership (`property IN (values)`)
//! - `InTenantSubtree` — tenant hierarchy via closure table
//! - `InGroup` — flat group membership
//! - `InGroupSubtree` — group hierarchy via closure + membership tables
//!
//! See `docs/arch/authorization/DESIGN.md` for the full predicate taxonomy.

use crate::pep::IntoPropertyValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A constraint on a specific resource property.
///
/// Multiple constraints within a response are `ORed`:
/// a resource matches if it satisfies ANY constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// The predicates within this constraint. All predicates are `ANDed`:
    /// a resource matches this constraint only if ALL predicates are satisfied.
    pub predicates: Vec<Predicate>,
}

/// A predicate comparing a resource property to a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Predicate {
    /// Equality: `resource_property = value`
    Eq(EqPredicate),
    /// Set membership: `resource_property IN (values)`
    In(InPredicate),
    /// Tenant subtree: `property IN (SELECT descendant_id FROM tenant_closure WHERE ...)`
    InTenantSubtree(InTenantSubtreePredicate),
    /// Group membership: `property IN (SELECT resource_id FROM resource_group_membership WHERE ...)`
    InGroup(InGroupPredicate),
    /// Group subtree: `property IN (SELECT ... FROM resource_group_membership JOIN resource_group_closure ...)`
    InGroupSubtree(InGroupSubtreePredicate),
}

/// Equality predicate: `property = value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqPredicate {
    /// Resource property name (e.g., `pep_properties::OWNER_TENANT_ID`, `pep_properties::RESOURCE_ID`).
    pub property: String,
    /// The value to match (UUID string, plain string, number, bool, etc.).
    pub value: Value,
}

impl EqPredicate {
    /// Create an equality predicate with any convertible value.
    #[must_use]
    pub fn new(property: impl Into<String>, value: impl IntoPropertyValue) -> Self {
        Self {
            property: property.into(),
            value: value.into_filter_value(),
        }
    }
}

/// Set membership predicate: `property IN (values)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InPredicate {
    /// Resource property name (e.g., `pep_properties::OWNER_TENANT_ID`, `pep_properties::RESOURCE_ID`).
    pub property: String,
    /// The set of values to match against.
    pub values: Vec<Value>,
}

impl InPredicate {
    /// Create an `IN` predicate from an iterator of convertible values.
    #[must_use]
    pub fn new<V: IntoPropertyValue>(
        property: impl Into<String>,
        values: impl IntoIterator<Item = V>,
    ) -> Self {
        Self {
            property: property.into(),
            values: values
                .into_iter()
                .map(IntoPropertyValue::into_filter_value)
                .collect(),
        }
    }
}

/// Barrier mode for tenant subtree predicates.
///
/// Mirrors `BarrierMode` from `models.rs` but lives in the constraint layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PredicateBarrierMode {
    /// Respect all barriers — `AND barrier = 0`.
    ///
    /// Serializes as `"respect"`. Also accepts legacy alias `"all"` on deserialization
    /// (used in architecture docs before SDK canonicalization).
    #[serde(alias = "all")]
    Respect,
    /// Ignore barriers — no barrier filter.
    ///
    /// Serializes as `"ignore"`. Also accepts legacy alias `"none"` on deserialization.
    #[serde(alias = "none")]
    Ignore,
}

impl Default for PredicateBarrierMode {
    fn default() -> Self {
        Self::Respect
    }
}

/// Tenant subtree predicate: `property IN (SELECT descendant_id FROM tenant_closure WHERE ...)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InTenantSubtreePredicate {
    /// Resource property name (e.g., `pep_properties::OWNER_TENANT_ID`).
    pub property: String,
    /// Root tenant ID for the subtree query.
    pub root_tenant_id: Uuid,
    /// Barrier enforcement mode (default: `Respect`).
    #[serde(default)]
    pub barrier_mode: PredicateBarrierMode,
    /// Optional tenant status filter (e.g., `["active", "suspended"]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_status: Option<Vec<String>>,
}

impl InTenantSubtreePredicate {
    /// Create a new tenant subtree predicate.
    #[must_use]
    pub fn new(property: impl Into<String>, root_tenant_id: Uuid) -> Self {
        Self {
            property: property.into(),
            root_tenant_id,
            barrier_mode: PredicateBarrierMode::default(),
            tenant_status: None,
        }
    }

    /// Set the barrier mode.
    #[must_use]
    pub fn barrier_mode(mut self, mode: PredicateBarrierMode) -> Self {
        self.barrier_mode = mode;
        self
    }

    /// Set the tenant status filter.
    #[must_use]
    pub fn tenant_status(mut self, statuses: Vec<String>) -> Self {
        self.tenant_status = Some(statuses);
        self
    }
}

/// Group membership predicate: `property IN (SELECT resource_id FROM resource_group_membership WHERE group_id IN (?))`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGroupPredicate {
    /// Resource property name (e.g., `pep_properties::RESOURCE_ID`).
    pub property: String,
    /// Group IDs to check membership against.
    pub group_ids: Vec<Uuid>,
}

impl InGroupPredicate {
    /// Create a new group membership predicate.
    #[must_use]
    pub fn new(property: impl Into<String>, group_ids: Vec<Uuid>) -> Self {
        Self {
            property: property.into(),
            group_ids,
        }
    }
}

/// Group subtree predicate: `property IN (SELECT resource_id FROM resource_group_membership WHERE group_id IN (SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = ?))`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGroupSubtreePredicate {
    /// Resource property name (e.g., `pep_properties::RESOURCE_ID`).
    pub property: String,
    /// Root group ID for the subtree query.
    pub root_group_id: Uuid,
}

impl InGroupSubtreePredicate {
    /// Create a new group subtree predicate.
    #[must_use]
    pub fn new(property: impl Into<String>, root_group_id: Uuid) -> Self {
        Self {
            property: property.into(),
            root_group_id,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use modkit_security::pep_properties;
    use serde_json::json;

    #[test]
    fn constraint_serialization_roundtrip() {
        let constraint = Constraint {
            predicates: vec![
                Predicate::In(InPredicate {
                    property: pep_properties::OWNER_TENANT_ID.to_owned(),
                    values: vec![
                        json!("11111111-1111-1111-1111-111111111111"),
                        json!("22222222-2222-2222-2222-222222222222"),
                    ],
                }),
                Predicate::Eq(EqPredicate {
                    property: pep_properties::RESOURCE_ID.to_owned(),
                    value: json!("33333333-3333-3333-3333-333333333333"),
                }),
            ],
        };

        let json_str = serde_json::to_string(&constraint).unwrap();
        let deserialized: Constraint = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.predicates.len(), 2);
    }

    #[test]
    fn predicate_tag_serialization() {
        let eq = Predicate::Eq(EqPredicate {
            property: pep_properties::RESOURCE_ID.to_owned(),
            value: json!("00000000-0000-0000-0000-000000000000"),
        });

        let json_str = serde_json::to_string(&eq).unwrap();
        assert!(json_str.contains(r#""op":"eq""#));

        let in_pred = Predicate::In(InPredicate {
            property: pep_properties::OWNER_TENANT_ID.to_owned(),
            values: vec![json!("00000000-0000-0000-0000-000000000000")],
        });

        let json_str = serde_json::to_string(&in_pred).unwrap();
        assert!(json_str.contains(r#""op":"in""#));
    }

    // --- InTenantSubtree serialization ---

    #[test]
    fn in_tenant_subtree_serialization_roundtrip() {
        let tid = uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let pred = Predicate::InTenantSubtree(InTenantSubtreePredicate {
            property: pep_properties::OWNER_TENANT_ID.to_owned(),
            root_tenant_id: tid,
            barrier_mode: PredicateBarrierMode::Respect,
            tenant_status: Some(vec!["active".to_owned(), "suspended".to_owned()]),
        });

        let json_str = serde_json::to_string(&pred).unwrap();
        assert!(json_str.contains(r#""op":"in_tenant_subtree""#));
        assert!(json_str.contains(r#""root_tenant_id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa""#));
        assert!(json_str.contains(r#""barrier_mode":"respect""#));

        let deserialized: Predicate = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(p.root_tenant_id, tid);
                assert_eq!(p.barrier_mode, PredicateBarrierMode::Respect);
                assert_eq!(
                    p.tenant_status,
                    Some(vec!["active".to_owned(), "suspended".to_owned()])
                );
            }
            other => panic!("Expected InTenantSubtree, got: {other:?}"),
        }
    }

    #[test]
    fn in_tenant_subtree_without_optional_fields() {
        let tid = uuid::Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
        let pred = InTenantSubtreePredicate::new(pep_properties::OWNER_TENANT_ID, tid);

        let json_str = serde_json::to_string(&Predicate::InTenantSubtree(pred)).unwrap();
        // barrier_mode defaults to "respect", tenant_status is skipped
        assert!(!json_str.contains("tenant_status"));

        let deserialized: Predicate = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(p.barrier_mode, PredicateBarrierMode::Respect);
                assert!(p.tenant_status.is_none());
            }
            other => panic!("Expected InTenantSubtree, got: {other:?}"),
        }
    }

    #[test]
    fn in_tenant_subtree_ignore_barrier() {
        let tid = uuid::Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
        let pred = InTenantSubtreePredicate::new(pep_properties::OWNER_TENANT_ID, tid)
            .barrier_mode(PredicateBarrierMode::Ignore);

        let json_str = serde_json::to_string(&Predicate::InTenantSubtree(pred)).unwrap();
        assert!(json_str.contains(r#""barrier_mode":"ignore""#));
    }

    #[test]
    fn barrier_mode_legacy_alias_all_deserializes_to_respect() {
        let json = r#"{"op":"in_tenant_subtree","property":"owner_tenant_id","root_tenant_id":"cccccccc-cccc-cccc-cccc-cccccccccccc","barrier_mode":"all"}"#;
        let pred: Predicate = serde_json::from_str(json).unwrap();
        match pred {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(p.barrier_mode, PredicateBarrierMode::Respect);
            }
            other => panic!("Expected InTenantSubtree, got: {other:?}"),
        }
    }

    #[test]
    fn barrier_mode_legacy_alias_none_deserializes_to_ignore() {
        let json = r#"{"op":"in_tenant_subtree","property":"owner_tenant_id","root_tenant_id":"cccccccc-cccc-cccc-cccc-cccccccccccc","barrier_mode":"none"}"#;
        let pred: Predicate = serde_json::from_str(json).unwrap();
        match pred {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(p.barrier_mode, PredicateBarrierMode::Ignore);
            }
            other => panic!("Expected InTenantSubtree, got: {other:?}"),
        }
    }

    // --- InGroup serialization ---

    #[test]
    fn in_group_serialization_roundtrip() {
        let g1 = uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let g2 = uuid::Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
        let pred = Predicate::InGroup(InGroupPredicate {
            property: pep_properties::RESOURCE_ID.to_owned(),
            group_ids: vec![g1, g2],
        });

        let json_str = serde_json::to_string(&pred).unwrap();
        assert!(json_str.contains(r#""op":"in_group""#));

        let deserialized: Predicate = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            Predicate::InGroup(p) => {
                assert_eq!(p.group_ids, vec![g1, g2]);
                assert_eq!(p.property, pep_properties::RESOURCE_ID);
            }
            other => panic!("Expected InGroup, got: {other:?}"),
        }
    }

    // --- InGroupSubtree serialization ---

    #[test]
    fn in_group_subtree_serialization_roundtrip() {
        let gid = uuid::Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
        let pred = Predicate::InGroupSubtree(InGroupSubtreePredicate {
            property: pep_properties::RESOURCE_ID.to_owned(),
            root_group_id: gid,
        });

        let json_str = serde_json::to_string(&pred).unwrap();
        assert!(json_str.contains(r#""op":"in_group_subtree""#));

        let deserialized: Predicate = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            Predicate::InGroupSubtree(p) => {
                assert_eq!(p.root_group_id, gid);
                assert_eq!(p.property, pep_properties::RESOURCE_ID);
            }
            other => panic!("Expected InGroupSubtree, got: {other:?}"),
        }
    }

    // --- Mixed constraint ---

    #[test]
    fn mixed_old_and_new_predicates_in_constraint() {
        let tid = uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let gid = uuid::Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
        let constraint = Constraint {
            predicates: vec![
                Predicate::InTenantSubtree(InTenantSubtreePredicate::new(
                    pep_properties::OWNER_TENANT_ID,
                    tid,
                )),
                Predicate::InGroupSubtree(InGroupSubtreePredicate::new(
                    pep_properties::RESOURCE_ID,
                    gid,
                )),
                Predicate::Eq(EqPredicate {
                    property: "status".to_owned(),
                    value: json!("active"),
                }),
            ],
        };

        let json_str = serde_json::to_string(&constraint).unwrap();
        let deserialized: Constraint = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.predicates.len(), 3);
    }
}
