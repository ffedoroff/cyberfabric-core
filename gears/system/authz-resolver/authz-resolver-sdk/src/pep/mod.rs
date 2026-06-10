//! PEP (Policy Enforcement Point) helpers.
//!
//! - [`Enforce`] - consumer-facing PEP trait (depend on this, not the concrete type)
//! - [`PolicyEnforcer`] - PEP object (build → evaluate → compile)
//! - [`ResourceType`] - Static descriptor for a resource type + its supported properties
//! - [`AccessRequest`] - Per-request evaluation overrides
//! - [`EnforcerError`] - Error from the PEP enforcement flow
//! - [`compile_to_access_scope`] - Low-level: compile evaluation response into `AccessScope`
//! - [`IntoPropertyValue`] - Convert typed values into `serde_json::Value` for PDP requests

pub mod access_request;
pub mod compiler;
pub mod enforce;
pub mod enforcer;
pub mod enforcer_error;
pub mod resource_type;
pub(crate) mod request_builder;

pub use access_request::AccessRequest;
pub use compiler::{ConstraintCompileError, compile_to_access_scope};
pub use enforce::Enforce;
pub use enforcer::PolicyEnforcer;
pub use enforcer_error::EnforcerError;
pub use resource_type::ResourceType;

/// Convert typed values into `serde_json::Value` for PDP evaluation requests
/// and predicate construction.
///
/// Defined in the [`crate::property_value`] leaf so it can be shared by
/// `constraints` and `pep` without an import cycle; re-exported here so
/// `pep::IntoPropertyValue` stays a stable path.
pub use crate::property_value::IntoPropertyValue;
