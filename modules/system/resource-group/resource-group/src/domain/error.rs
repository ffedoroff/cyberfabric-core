//! Domain error types for the resource-group module.

use resource_group_sdk::ResourceGroupError;
use thiserror::Error;

/// Domain-specific errors for the resource-group module.
#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Type not found: {code}")]
    TypeNotFound { code: String },

    #[error("Type already exists: {code}")]
    TypeAlreadyExists { code: String },

    #[error("Validation failed: {message}")]
    Validation { message: String },

    #[error("Allowed parents violation: {message}")]
    AllowedParentsViolation { message: String },

    #[error("Active references exist: {message}")]
    ConflictActiveReferences { message: String },

    #[error("Group not found: {id}")]
    GroupNotFound { id: uuid::Uuid },

    #[error("Invalid parent type: {message}")]
    InvalidParentType { message: String },

    #[error("Cycle detected: {message}")]
    CycleDetected { message: String },

    #[error("Limit violation: {message}")]
    LimitViolation { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Tenant incompatibility: {message}")]
    TenantIncompatibility { message: String },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Internal error")]
    InternalError,
}

impl DomainError {
    pub fn type_not_found(code: impl Into<String>) -> Self {
        Self::TypeNotFound { code: code.into() }
    }

    pub fn type_already_exists(code: impl Into<String>) -> Self {
        Self::TypeAlreadyExists { code: code.into() }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn allowed_parents_violation(message: impl Into<String>) -> Self {
        Self::AllowedParentsViolation {
            message: message.into(),
        }
    }

    pub fn conflict_active_references(message: impl Into<String>) -> Self {
        Self::ConflictActiveReferences {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn group_not_found(id: uuid::Uuid) -> Self {
        Self::GroupNotFound { id }
    }

    pub fn invalid_parent_type(message: impl Into<String>) -> Self {
        Self::InvalidParentType {
            message: message.into(),
        }
    }

    pub fn cycle_detected(message: impl Into<String>) -> Self {
        Self::CycleDetected {
            message: message.into(),
        }
    }

    pub fn limit_violation(message: impl Into<String>) -> Self {
        Self::LimitViolation {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn tenant_incompatibility(message: impl Into<String>) -> Self {
        Self::TenantIncompatibility {
            message: message.into(),
        }
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
        }
    }
}

/// Convert domain errors to SDK errors for public API consumption.
impl From<DomainError> for ResourceGroupError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::TypeNotFound { code } => ResourceGroupError::not_found(code),
            DomainError::TypeAlreadyExists { code } => {
                ResourceGroupError::type_already_exists(code)
            }
            DomainError::Validation { message }
            | DomainError::InvalidParentType { message }
            | DomainError::CycleDetected { message }
            | DomainError::LimitViolation { message } => ResourceGroupError::validation(message),
            DomainError::AllowedParentsViolation { message } => {
                ResourceGroupError::allowed_parents_violation(message)
            }
            DomainError::ConflictActiveReferences { message } => {
                ResourceGroupError::conflict_active_references(message)
            }
            DomainError::GroupNotFound { id } => ResourceGroupError::not_found(id.to_string()),
            DomainError::Conflict { message } => {
                ResourceGroupError::conflict_active_references(message)
            }
            DomainError::TenantIncompatibility { message } => {
                ResourceGroupError::tenant_incompatibility(message)
            }
            DomainError::Database { .. } | DomainError::InternalError => {
                ResourceGroupError::internal()
            }
        }
    }
}

impl From<sea_orm::DbErr> for DomainError {
    fn from(e: sea_orm::DbErr) -> Self {
        DomainError::database(e.to_string())
    }
}

impl From<modkit_db::DbError> for DomainError {
    fn from(e: modkit_db::DbError) -> Self {
        DomainError::database(e.to_string())
    }
}
