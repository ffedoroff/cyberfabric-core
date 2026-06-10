//! Barrier-traversal mode — shared leaf type.
//!
//! Extracted into its own leaf module so that both the low-level
//! [`crate::constraints`] vocabulary and the high-level [`crate::models`]
//! envelope can depend on it without forming an import cycle (ADP).
//!
//! Re-exported as [`crate::models::BarrierMode`] for backward compatibility.

use serde::{Deserialize, Serialize};

/// Controls how barriers (self-managed tenants) are handled during `AuthZ` evaluation.
///
/// Consistent with `tenant_resolver_sdk::BarrierMode`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BarrierMode {
    /// Respect all barriers - stop at barrier boundaries (default).
    #[default]
    Respect,
    /// Ignore barriers - traverse through self-managed tenants.
    Ignore,
}
