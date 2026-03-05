use resource_group_sdk::ResourceGroupError;
use uuid::Uuid;

/// Internal domain error used within the module.
/// Mapped to `ResourceGroupError` (SDK) and `Problem` (REST) at boundaries.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DomainError {
    #[error("Validation: {message}")]
    Validation { message: String },

    #[error("Type not found: {code}")]
    TypeNotFound { code: String },

    #[error("Group not found: {id}")]
    GroupNotFound { id: Uuid },

    #[error("Membership not found: group={group_id}, type={resource_type}, id={resource_id}")]
    MembershipNotFound {
        group_id: Uuid,
        resource_type: String,
        resource_id: String,
    },

    #[error("Type already exists: {code}")]
    TypeAlreadyExists { code: String },

    #[error("Invalid parent type: child={child_type}, parent={parent_type}")]
    InvalidParentType {
        child_type: String,
        parent_type: String,
    },

    #[error("Cycle detected: {ancestor_id} -> {descendant_id}")]
    CycleDetected {
        ancestor_id: Uuid,
        descendant_id: Uuid,
    },

    #[error("Active references: {count} references prevent deletion")]
    ActiveReferences { count: i64 },

    #[error("Limit violation: {limit_name}={current} exceeds max={max}")]
    LimitViolation {
        limit_name: String,
        current: i64,
        max: i64,
    },

    #[error("Tenant incompatibility: {message}")]
    TenantIncompatibility { message: String },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Forbidden")]
    Forbidden,
}

impl DomainError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
        }
    }

    pub fn database_err(err: impl std::fmt::Display) -> Self {
        Self::Database {
            message: err.to_string(),
        }
    }
}

impl From<DomainError> for ResourceGroupError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::Validation { message } => ResourceGroupError::Validation { message },
            DomainError::TypeNotFound { code } => ResourceGroupError::not_found("Type", code),
            DomainError::GroupNotFound { id } => ResourceGroupError::group_not_found(id),
            DomainError::MembershipNotFound {
                group_id,
                resource_type,
                resource_id,
            } => ResourceGroupError::not_found(
                "Membership",
                format!("{group_id}/{resource_type}/{resource_id}"),
            ),
            DomainError::TypeAlreadyExists { code } => {
                ResourceGroupError::TypeAlreadyExists { code }
            }
            DomainError::InvalidParentType {
                child_type,
                parent_type,
            } => ResourceGroupError::InvalidParentType {
                child_type,
                parent_type,
            },
            DomainError::CycleDetected {
                ancestor_id,
                descendant_id,
            } => ResourceGroupError::CycleDetected {
                ancestor_id,
                descendant_id,
            },
            DomainError::ActiveReferences { count } => {
                ResourceGroupError::ConflictActiveReferences {
                    reference_count: count,
                }
            }
            DomainError::LimitViolation {
                limit_name,
                current,
                max,
            } => ResourceGroupError::LimitViolation {
                limit_name,
                current_value: current,
                max_value: max,
            },
            DomainError::TenantIncompatibility { message } => {
                ResourceGroupError::TenantIncompatibility { message }
            }
            DomainError::Database { .. } | DomainError::Forbidden => ResourceGroupError::Internal,
        }
    }
}
