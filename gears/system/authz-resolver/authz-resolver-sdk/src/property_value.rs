//! [`IntoPropertyValue`] — shared leaf trait.
//!
//! This trait lives at the PEP level conceptually (it converts typed values
//! into JSON for the authorization protocol), but it is defined here in a
//! crate-level leaf so that both [`crate::constraints`] (predicate
//! construction) and [`crate::pep`] (enforcer/compiler) can depend on it
//! without forming an import cycle through the `pep` re-export hub (ADP).
//!
//! Re-exported as [`crate::pep::IntoPropertyValue`] for backward compatibility.
//!
//! For scope-level typed values (post-compilation), see
//! [`toolkit_security::ScopeValue`].

use serde_json::Value;
use uuid::Uuid;

/// Trait for types that can be converted into `serde_json::Value` for PDP
/// evaluation requests and predicate construction.
pub trait IntoPropertyValue {
    /// Convert into a `serde_json::Value` for use in authorization predicates.
    fn into_filter_value(self) -> Value;
}

impl IntoPropertyValue for Uuid {
    #[inline]
    fn into_filter_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoPropertyValue for &Uuid {
    #[inline]
    fn into_filter_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoPropertyValue for String {
    #[inline]
    fn into_filter_value(self) -> Value {
        Value::String(self)
    }
}

impl IntoPropertyValue for &str {
    #[inline]
    fn into_filter_value(self) -> Value {
        Value::String(self.to_owned())
    }
}

impl IntoPropertyValue for i64 {
    #[inline]
    fn into_filter_value(self) -> Value {
        Value::Number(self.into())
    }
}

impl IntoPropertyValue for bool {
    #[inline]
    fn into_filter_value(self) -> Value {
        Value::Bool(self)
    }
}

impl IntoPropertyValue for Value {
    #[inline]
    fn into_filter_value(self) -> Value {
        self
    }
}
