// @cpt-dod:cpt-cf-resource-group-dod-sdk-foundation-sdk-errors:p1
//! Public error types for the resource-group module.
//!
//! These errors are safe to expose to other modules and consumers.

use thiserror::Error;

/// Errors that can be returned by the `ResourceGroupClient`.
#[derive(Error, Debug, Clone)]
pub enum ResourceGroupError {
    /// Resource with the specified code was not found.
    #[error("Type not found: {code}")]
    NotFound { code: String },

    /// A type with the specified code already exists.
    #[error("Type already exists: {code}")]
    TypeAlreadyExists { code: String },

    /// Validation error with the provided data.
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Removing allowed parents or disabling root placement would break
    /// existing group hierarchy relationships.
    #[error("Allowed parents violation: {message}")]
    AllowedParentsViolation { message: String },

    /// Cannot delete a type because groups of this type still exist.
    #[error("Active references exist: {message}")]
    ConflictActiveReferences { message: String },

    /// Tenant scope incompatibility.
    #[error("Tenant incompatibility: {message}")]
    TenantIncompatibility { message: String },

    /// An internal error occurred.
    #[error("Internal error")]
    Internal,
}

impl ResourceGroupError {
    /// Create a `NotFound` error.
    pub fn not_found(code: impl Into<String>) -> Self {
        Self::NotFound { code: code.into() }
    }

    /// Create a `TypeAlreadyExists` error.
    pub fn type_already_exists(code: impl Into<String>) -> Self {
        Self::TypeAlreadyExists { code: code.into() }
    }

    /// Create a `Validation` error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create an `AllowedParentsViolation` error.
    pub fn allowed_parents_violation(message: impl Into<String>) -> Self {
        Self::AllowedParentsViolation {
            message: message.into(),
        }
    }

    /// Create a `ConflictActiveReferences` error.
    pub fn conflict_active_references(message: impl Into<String>) -> Self {
        Self::ConflictActiveReferences {
            message: message.into(),
        }
    }

    /// Create a `TenantIncompatibility` error.
    pub fn tenant_incompatibility(message: impl Into<String>) -> Self {
        Self::TenantIncompatibility {
            message: message.into(),
        }
    }

    /// Create an `Internal` error.
    #[must_use]
    pub fn internal() -> Self {
        Self::Internal
    }
}
