// @cpt-req:cpt-cf-resource-group-dod-error-mapper:p1
// @cpt-algo:cpt-cf-resource-group-algo-error-mapping:p1

use http::StatusCode;
use modkit::api::problem::Problem;

use crate::domain::error::DomainError;

impl From<DomainError> for Problem {
    fn from(err: DomainError) -> Self {
        let trace_id = tracing::Span::current()
            .id()
            .map(|id| id.into_u64().to_string());
        let trace = trace_id.unwrap_or_default();

        match &err {
            DomainError::Validation { message } => Problem::new(
                StatusCode::BAD_REQUEST,
                "Validation Error",
                message.clone(),
            )
            .with_trace_id(trace),

            DomainError::TypeNotFound { code } => Problem::new(
                StatusCode::NOT_FOUND,
                "Type Not Found",
                format!("Type with code '{code}' was not found"),
            )
            .with_trace_id(trace),

            DomainError::GroupNotFound { id } => Problem::new(
                StatusCode::NOT_FOUND,
                "Group Not Found",
                format!("Group with id '{id}' was not found"),
            )
            .with_trace_id(trace),

            DomainError::MembershipNotFound {
                group_id,
                resource_type,
                resource_id,
            } => Problem::new(
                StatusCode::NOT_FOUND,
                "Membership Not Found",
                format!(
                    "Membership {group_id}/{resource_type}/{resource_id} was not found"
                ),
            )
            .with_trace_id(trace),

            DomainError::TypeAlreadyExists { code } => Problem::new(
                StatusCode::CONFLICT,
                "Type Already Exists",
                format!("Type with code '{code}' already exists"),
            )
            .with_trace_id(trace),

            DomainError::InvalidParentType {
                child_type,
                parent_type,
            } => Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid Parent Type",
                format!(
                    "Type '{child_type}' cannot have parent type '{parent_type}'"
                ),
            )
            .with_trace_id(trace),

            DomainError::CycleDetected {
                ancestor_id,
                descendant_id,
            } => Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Cycle Detected",
                format!(
                    "Adding this relationship would create a cycle between {ancestor_id} and {descendant_id}"
                ),
            )
            .with_trace_id(trace),

            DomainError::ActiveReferences { count } => Problem::new(
                StatusCode::CONFLICT,
                "Active References",
                format!("Cannot delete: {count} active references exist"),
            )
            .with_trace_id(trace),

            DomainError::LimitViolation {
                limit_name,
                current,
                max,
            } => Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Limit Violation",
                format!("{limit_name} = {current} exceeds maximum {max}"),
            )
            .with_trace_id(trace),

            DomainError::TenantIncompatibility { message } => Problem::new(
                StatusCode::FORBIDDEN,
                "Tenant Incompatibility",
                message.clone(),
            )
            .with_trace_id(trace),

            DomainError::ServiceUnavailable { message } => Problem::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "Service Unavailable",
                message.clone(),
            )
            .with_trace_id(trace),

            DomainError::Database { .. } => {
                tracing::error!(error = ?err, "Internal database error");
                Problem::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Error",
                    "An internal error occurred",
                )
                .with_trace_id(trace)
            }

            DomainError::Forbidden => Problem::new(
                StatusCode::FORBIDDEN,
                "Forbidden",
                "Access denied",
            )
            .with_trace_id(trace),
        }
    }
}
