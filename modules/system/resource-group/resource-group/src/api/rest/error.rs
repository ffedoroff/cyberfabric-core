// @cpt-algo:cpt-cf-resource-group-algo-sdk-foundation-map-domain-error:p1
//! Map domain errors to RFC 9457 Problem Details for REST responses.

use modkit::api::problem::Problem;

use crate::domain::error::DomainError;

/// Implement `Into<Problem>` for `DomainError` so `?` works in handlers.
impl From<DomainError> for Problem {
    fn from(e: DomainError) -> Self {
        match &e {
            DomainError::TypeNotFound { code } => Problem::new(
                http::StatusCode::NOT_FOUND,
                "Type not found",
                format!("GTS type with code '{code}' was not found"),
            ),
            DomainError::TypeAlreadyExists { code } => Problem::new(
                http::StatusCode::CONFLICT,
                "Type already exists",
                format!("GTS type with code '{code}' already exists"),
            ),
            DomainError::Validation { message } => {
                Problem::new(http::StatusCode::BAD_REQUEST, "Validation error", message)
            }
            DomainError::AllowedParentsViolation { message } => Problem::new(
                http::StatusCode::CONFLICT,
                "Allowed parents violation",
                message,
            ),
            DomainError::ConflictActiveReferences { message } => Problem::new(
                http::StatusCode::CONFLICT,
                "Active references exist",
                message,
            ),
            DomainError::GroupNotFound { id } => Problem::new(
                http::StatusCode::NOT_FOUND,
                "Group not found",
                format!("Resource group with id '{id}' was not found"),
            ),
            DomainError::InvalidParentType { message } => Problem::new(
                http::StatusCode::BAD_REQUEST,
                "Invalid parent type",
                message,
            ),
            DomainError::CycleDetected { message } => {
                Problem::new(http::StatusCode::CONFLICT, "Cycle detected", message)
            }
            DomainError::LimitViolation { message } => {
                Problem::new(http::StatusCode::CONFLICT, "Limit violation", message)
            }
            DomainError::Conflict { message } => {
                Problem::new(http::StatusCode::CONFLICT, "Conflict", message)
            }
            DomainError::TenantIncompatibility { message } => Problem::new(
                http::StatusCode::CONFLICT,
                "Tenant incompatibility",
                message,
            ),
            DomainError::Database { .. } => {
                tracing::error!(error = ?e, "Database error occurred");
                Problem::new(
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error",
                    "An internal database error occurred",
                )
            }
            DomainError::InternalError => {
                tracing::error!(error = ?e, "Internal error occurred");
                Problem::new(
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error",
                    "An internal error occurred",
                )
            }
        }
    }
}
