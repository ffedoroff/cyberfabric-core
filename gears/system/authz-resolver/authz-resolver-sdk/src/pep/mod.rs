//! PEP (Policy Enforcement Point) helpers.
//!
//! - [`PolicyEnforcer`] - PEP object (build → evaluate → compile)
//! - [`ResourceType`] - Static descriptor for a resource type + its supported properties
//! - [`compile_to_access_scope`] - Low-level: compile evaluation response into `AccessScope`
//! - [`IntoPropertyValue`] - Convert typed values into `serde_json::Value` for PDP requests

pub mod compiler;
pub mod enforcer;

pub use compiler::{ConstraintCompileError, compile_to_access_scope};
pub use enforcer::{AccessRequest, EnforcerError, PolicyEnforcer, ResourceType};

/// Convert typed values into `serde_json::Value` for PDP evaluation requests
/// and predicate construction.
///
/// Defined in the [`crate::property_value`] leaf so it can be shared by
/// `constraints` and `pep` without an import cycle; re-exported here so
/// `pep::IntoPropertyValue` stays a stable path.
pub use crate::property_value::IntoPropertyValue;
