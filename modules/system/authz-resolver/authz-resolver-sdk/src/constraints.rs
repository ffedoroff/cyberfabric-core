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
    /// Tenant subtree: `property IN (SELECT descendant_id FROM resource_group_closure JOIN resource_group WHERE group_type = 'tenant')`
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

/// Tenant subtree predicate: `property IN (SELECT descendant_id FROM resource_group_closure JOIN resource_group ... WHERE group_type = 'tenant')`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InTenantSubtreePredicate {
    /// Resource property name (e.g., `pep_properties::OWNER_TENANT_ID`).
    pub property: String,
    /// Root tenant ID for the subtree query.
    pub root_tenant_id: Uuid,
}

impl InTenantSubtreePredicate {
    /// Create a new tenant subtree predicate.
    #[must_use]
    pub fn new(property: impl Into<String>, root_tenant_id: Uuid) -> Self {
        Self {
            property: property.into(),
            root_tenant_id,
        }
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
        });

        let json_str = serde_json::to_string(&pred).unwrap();
        assert!(json_str.contains(r#""op":"in_tenant_subtree""#));
        assert!(json_str.contains(r#""root_tenant_id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa""#));

        let deserialized: Predicate = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(p.root_tenant_id, tid);
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
