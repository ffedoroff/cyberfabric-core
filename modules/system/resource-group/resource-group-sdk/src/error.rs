use uuid::Uuid;

/// Deterministic public error taxonomy for Resource Group operations.
/// Maps to DESIGN section 3.9 error mapping table.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResourceGroupError {
    /// Invalid input: format, length, or missing required field.
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Missing type or entity (lookup returned no rows).
    #[error("{entity_kind} not found: {identifier}")]
    NotFound {
        entity_kind: String,
        identifier: String,
    },

    /// Duplicate type code (unique constraint violation on `code_ci`).
    #[error("Type already exists: {code}")]
    TypeAlreadyExists { code: String },

    /// Invalid parent type (parent-child compatibility violation).
    #[error(
        "Invalid parent type: child type '{child_type}' cannot have parent type '{parent_type}'"
    )]
    InvalidParentType {
        child_type: String,
        parent_type: String,
    },

    /// Cycle detected in hierarchy (closure table cycle check).
    #[error("Cycle detected between {ancestor_id} and {descendant_id}")]
    CycleDetected {
        ancestor_id: Uuid,
        descendant_id: Uuid,
    },

    /// Active references prevent deletion (children or memberships exist).
    #[error("Cannot delete: {reference_count} active references exist")]
    ConflictActiveReferences { reference_count: i64 },

    /// Depth or width limit violation (query profile exceeded).
    #[error("Limit violation: {limit_name} = {current_value} exceeds maximum {max_value}")]
    LimitViolation {
        limit_name: String,
        current_value: i64,
        max_value: i64,
    },

    /// Tenant-incompatible write (parent/child/membership tenant mismatch).
    #[error("Tenant incompatibility: {message}")]
    TenantIncompatibility { message: String },

    /// Infrastructure timeout or service unavailability.
    #[error("Service unavailable")]
    ServiceUnavailable,

    /// Unexpected or unclassified failure.
    #[error("Internal error")]
    Internal,
}

impl ResourceGroupError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn not_found(entity_kind: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            entity_kind: entity_kind.into(),
            identifier: identifier.into(),
        }
    }

    pub fn type_not_found(code: impl Into<String>) -> Self {
        Self::not_found("Type", code)
    }

    #[must_use]
    pub fn group_not_found(id: Uuid) -> Self {
        Self::not_found("Group", id.to_string())
    }

    pub fn type_already_exists(code: impl Into<String>) -> Self {
        Self::TypeAlreadyExists { code: code.into() }
    }

    pub fn tenant_incompatibility(message: impl Into<String>) -> Self {
        Self::TenantIncompatibility {
            message: message.into(),
        }
    }
}
