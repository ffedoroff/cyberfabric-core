// @cpt-req:cpt-cf-resource-group-dod-error-mapper:p1
// @cpt-algo:cpt-cf-resource-group-algo-error-mapping:p1

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

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Forbidden")]
    Forbidden,
}

impl From<modkit_db::DbError> for DomainError {
    fn from(err: modkit_db::DbError) -> Self {
        DomainError::Database {
            message: err.to_string(),
        }
    }
}

impl From<authz_resolver_sdk::EnforcerError> for DomainError {
    fn from(e: authz_resolver_sdk::EnforcerError) -> Self {
        match e {
            authz_resolver_sdk::EnforcerError::Denied { ref deny_reason } => {
                tracing::warn!(deny_reason = ?deny_reason, "AuthZ denied access");
                Self::Forbidden
            }
            authz_resolver_sdk::EnforcerError::CompileFailed(ref err) => {
                tracing::error!(error = %err, "AuthZ constraint compile failed");
                Self::Database {
                    message: format!("authorization constraint compilation failed: {err}"),
                }
            }
            authz_resolver_sdk::EnforcerError::EvaluationFailed(ref err) => {
                tracing::error!(error = %err, "AuthZ evaluation failed");
                Self::ServiceUnavailable {
                    message: format!("authorization service unavailable: {err}"),
                }
            }
        }
    }
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

// @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-1
impl From<DomainError> for ResourceGroupError {
    fn from(err: DomainError) -> Self {
        match err {
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-2
            DomainError::Validation { message } => ResourceGroupError::Validation { message },
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-2
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-3
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
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-3
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-4
            DomainError::TypeAlreadyExists { code } => {
                ResourceGroupError::TypeAlreadyExists { code }
            }
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-4
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-5
            DomainError::InvalidParentType {
                child_type,
                parent_type,
            } => ResourceGroupError::InvalidParentType {
                child_type,
                parent_type,
            },
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-5
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-6
            DomainError::CycleDetected {
                ancestor_id,
                descendant_id,
            } => ResourceGroupError::CycleDetected {
                ancestor_id,
                descendant_id,
            },
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-6
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-7
            DomainError::ActiveReferences { count } => {
                ResourceGroupError::ConflictActiveReferences {
                    reference_count: count,
                }
            }
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-7
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-8
            DomainError::LimitViolation {
                limit_name,
                current,
                max,
            } => ResourceGroupError::LimitViolation {
                limit_name,
                current_value: current,
                max_value: max,
            },
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-8
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-9
            DomainError::TenantIncompatibility { message } => {
                ResourceGroupError::TenantIncompatibility { message }
            }
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-9
            // @cpt-begin:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-11
            DomainError::Forbidden => ResourceGroupError::Forbidden,
            DomainError::ServiceUnavailable { .. } => ResourceGroupError::ServiceUnavailable,
            DomainError::Database { .. } => ResourceGroupError::Internal,
            // @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-11
        }
    }
}
// @cpt-end:cpt-cf-resource-group-algo-error-mapping:p1:inst-errmap-1
