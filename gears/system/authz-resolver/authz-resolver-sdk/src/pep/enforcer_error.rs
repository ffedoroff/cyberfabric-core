//! [`EnforcerError`] — error from the PEP enforcement flow.

use crate::error::AuthZResolverError;
use crate::models::DenyReason;
use crate::pep::compiler::ConstraintCompileError;

/// Error from the PEP enforcement flow.
#[derive(Debug, thiserror::Error)]
pub enum EnforcerError {
    /// The PDP explicitly denied access.
    #[error("access denied by PDP")]
    Denied {
        /// Optional deny reason from the PDP.
        deny_reason: Option<DenyReason>,
    },

    /// The `AuthZ` evaluation RPC failed.
    #[error("authorization evaluation failed: {0}")]
    EvaluationFailed(#[from] AuthZResolverError),

    /// Constraint compilation failed (missing or unsupported constraints).
    #[error("constraint compilation failed: {0}")]
    CompileFailed(#[from] ConstraintCompileError),
}
