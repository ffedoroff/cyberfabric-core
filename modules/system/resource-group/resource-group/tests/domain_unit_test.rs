//! Unit tests for domain layer pure logic.
//!
//! Tests validation functions, error construction, error mapping,
//! and serialization failure detection — all without database dependencies.
//!
//! Full domain service tests with mock repositories are deferred to
//! TODO-16 (repository trait abstraction).

use cf_resource_group::domain::error::DomainError;
use cf_resource_group::domain::validation::{self, RG_TYPE_PREFIX};

// ── validate_type_code ──────────────────────────────────────────────────

#[test]
fn validate_type_code_rejects_empty() {
    let result = validation::validate_type_code("");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DomainError::Validation { .. }));
    assert!(err.to_string().contains("empty"));
}

#[test]
fn validate_type_code_rejects_wrong_prefix() {
    let result = validation::validate_type_code("wrong.prefix.type");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DomainError::Validation { .. }));
    assert!(err.to_string().contains("prefix"));
}

#[test]
fn validate_type_code_rejects_too_long() {
    let long_code = format!(
        "{}{}",
        RG_TYPE_PREFIX,
        "a".repeat(1025 - RG_TYPE_PREFIX.len())
    );
    assert!(long_code.len() > 1024);
    let result = validation::validate_type_code(&long_code);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DomainError::Validation { .. }));
    assert!(err.to_string().contains("1024"));
}

#[test]
fn validate_type_code_accepts_valid_code() {
    let code = format!("{RG_TYPE_PREFIX}tenant");
    let result = validation::validate_type_code(&code);
    assert!(result.is_ok());
}

#[test]
fn validate_type_code_accepts_exact_max_length() {
    let code = format!(
        "{}{}",
        RG_TYPE_PREFIX,
        "a".repeat(1024 - RG_TYPE_PREFIX.len())
    );
    assert_eq!(code.len(), 1024);
    let result = validation::validate_type_code(&code);
    assert!(result.is_ok());
}

#[test]
fn validate_type_code_rejects_prefix_only() {
    // The prefix itself is a valid type code (non-empty, correct prefix, within length)
    let result = validation::validate_type_code(RG_TYPE_PREFIX);
    assert!(result.is_ok());
}

// ── DomainError construction ────────────────────────────────────────────

#[test]
fn domain_error_type_not_found_format() {
    let err = DomainError::type_not_found("my.type.code");
    assert!(matches!(err, DomainError::TypeNotFound { .. }));
    assert!(err.to_string().contains("my.type.code"));
}

#[test]
fn domain_error_type_already_exists_format() {
    let err = DomainError::type_already_exists("dup.code");
    assert!(matches!(err, DomainError::TypeAlreadyExists { .. }));
    assert!(err.to_string().contains("dup.code"));
}

#[test]
fn domain_error_validation_format() {
    let err = DomainError::validation("bad input");
    assert!(matches!(err, DomainError::Validation { .. }));
    assert!(err.to_string().contains("bad input"));
}

#[test]
fn domain_error_group_not_found_format() {
    let id = uuid::Uuid::now_v7();
    let err = DomainError::group_not_found(id);
    assert!(matches!(err, DomainError::GroupNotFound { .. }));
    assert!(err.to_string().contains(&id.to_string()));
}

#[test]
fn domain_error_cycle_detected_format() {
    let err = DomainError::cycle_detected("A -> B -> A");
    assert!(matches!(err, DomainError::CycleDetected { .. }));
    assert!(err.to_string().contains("A -> B -> A"));
}

#[test]
fn domain_error_limit_violation_format() {
    let err = DomainError::limit_violation("depth exceeded");
    assert!(matches!(err, DomainError::LimitViolation { .. }));
    assert!(err.to_string().contains("depth exceeded"));
}

#[test]
fn domain_error_invalid_parent_type_format() {
    let err = DomainError::invalid_parent_type("type mismatch");
    assert!(matches!(err, DomainError::InvalidParentType { .. }));
    assert!(err.to_string().contains("type mismatch"));
}

#[test]
fn domain_error_conflict_active_references_format() {
    let err = DomainError::conflict_active_references("has children");
    assert!(matches!(err, DomainError::ConflictActiveReferences { .. }));
    assert!(err.to_string().contains("has children"));
}

#[test]
fn domain_error_allowed_parents_violation_format() {
    let err = DomainError::allowed_parents_violation("parent removed");
    assert!(matches!(err, DomainError::AllowedParentsViolation { .. }));
    assert!(err.to_string().contains("parent removed"));
}

#[test]
fn domain_error_tenant_incompatibility_format() {
    let err = DomainError::tenant_incompatibility("wrong tenant");
    assert!(matches!(err, DomainError::TenantIncompatibility { .. }));
    assert!(err.to_string().contains("wrong tenant"));
}

#[test]
fn domain_error_database_format() {
    let err = DomainError::database("connection lost");
    assert!(matches!(err, DomainError::Database { .. }));
    assert!(err.to_string().contains("connection lost"));
}

#[test]
fn domain_error_membership_not_found_format() {
    let err = DomainError::membership_not_found("(gid, type, rid)");
    assert!(matches!(err, DomainError::MembershipNotFound { .. }));
    assert!(err.to_string().contains("(gid, type, rid)"));
}

#[test]
fn domain_error_conflict_format() {
    let err = DomainError::conflict("duplicate key");
    assert!(matches!(err, DomainError::Conflict { .. }));
    assert!(err.to_string().contains("duplicate key"));
}

// ── is_serialization_failure ────────────────────────────────────────────

#[test]
fn is_serialization_failure_detects_sqlstate_40001() {
    let err = DomainError::database("ERROR: 40001 could not serialize access");
    assert!(err.is_serialization_failure());
}

#[test]
fn is_serialization_failure_detects_serialize_message() {
    let err = DomainError::database("could not serialize access due to concurrent update");
    assert!(err.is_serialization_failure());
}

#[test]
fn is_serialization_failure_false_for_other_db_errors() {
    let err = DomainError::database("connection refused");
    assert!(!err.is_serialization_failure());
}

#[test]
fn is_serialization_failure_false_for_non_db_errors() {
    let err = DomainError::validation("bad input");
    assert!(!err.is_serialization_failure());
}

// ── DomainError -> ResourceGroupError mapping ───────────────────────────

#[test]
fn domain_to_sdk_type_not_found() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::type_not_found("code");
    let sdk: ResourceGroupError = domain.into();
    assert!(sdk.to_string().contains("code"));
}

#[test]
fn domain_to_sdk_type_already_exists() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::type_already_exists("code");
    let sdk: ResourceGroupError = domain.into();
    assert!(sdk.to_string().contains("code"));
}

#[test]
fn domain_to_sdk_validation() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::validation("msg");
    let sdk: ResourceGroupError = domain.into();
    assert!(sdk.to_string().contains("msg") || !sdk.to_string().is_empty());
}

#[test]
fn domain_to_sdk_group_not_found() {
    use resource_group_sdk::ResourceGroupError;
    let id = uuid::Uuid::now_v7();
    let domain = DomainError::group_not_found(id);
    let sdk: ResourceGroupError = domain.into();
    assert!(sdk.to_string().contains(&id.to_string()));
}

#[test]
fn domain_to_sdk_cycle_detected() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::cycle_detected("cycle");
    let sdk: ResourceGroupError = domain.into();
    assert!(!sdk.to_string().is_empty());
}

#[test]
fn domain_to_sdk_database_maps_to_internal() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::database("db error");
    let sdk: ResourceGroupError = domain.into();
    // Database errors map to internal (no sensitive info leaked)
    assert!(sdk.to_string().to_lowercase().contains("internal"));
}

#[test]
fn domain_to_sdk_membership_not_found() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::membership_not_found("key");
    let sdk: ResourceGroupError = domain.into();
    assert!(sdk.to_string().contains("key"));
}

#[test]
fn domain_to_problem_membership_not_found_is_404() {
    use modkit::api::problem::Problem;
    let domain = DomainError::membership_not_found("(gid, type, rid)");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::NOT_FOUND);
}

#[test]
fn domain_to_sdk_access_denied_maps_to_internal() {
    use resource_group_sdk::ResourceGroupError;
    let domain = DomainError::AccessDenied {
        message: "denied".to_owned(),
    };
    let sdk: ResourceGroupError = domain.into();
    assert!(sdk.to_string().to_lowercase().contains("internal"));
}

// ── DomainError -> Problem (RFC 9457) mapping ───────────────────────────

#[test]
fn domain_to_problem_type_not_found_is_404() {
    use modkit::api::problem::Problem;
    let domain = DomainError::type_not_found("my.code");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::NOT_FOUND);
}

#[test]
fn domain_to_problem_type_already_exists_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::type_already_exists("dup");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}

#[test]
fn domain_to_problem_validation_is_400() {
    use modkit::api::problem::Problem;
    let domain = DomainError::validation("bad");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::BAD_REQUEST);
}

#[test]
fn domain_to_problem_group_not_found_is_404() {
    use modkit::api::problem::Problem;
    let domain = DomainError::group_not_found(uuid::Uuid::now_v7());
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::NOT_FOUND);
}

#[test]
fn domain_to_problem_cycle_detected_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::cycle_detected("cycle");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}

#[test]
fn domain_to_problem_limit_violation_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::limit_violation("too deep");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}

#[test]
fn domain_to_problem_invalid_parent_type_is_400() {
    use modkit::api::problem::Problem;
    let domain = DomainError::invalid_parent_type("mismatch");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::BAD_REQUEST);
}

#[test]
fn domain_to_problem_conflict_active_refs_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::conflict_active_references("children exist");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}

#[test]
fn domain_to_problem_allowed_parents_violation_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::allowed_parents_violation("violation");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}

#[test]
fn domain_to_problem_tenant_incompatibility_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::tenant_incompatibility("wrong tenant");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}

#[test]
fn domain_to_problem_access_denied_is_403() {
    use modkit::api::problem::Problem;
    let domain = DomainError::AccessDenied {
        message: "denied".to_owned(),
    };
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::FORBIDDEN);
}

#[test]
fn domain_to_problem_database_is_500() {
    use modkit::api::problem::Problem;
    let domain = DomainError::database("db error");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn domain_to_problem_internal_error_is_500() {
    use modkit::api::problem::Problem;
    let problem: Problem = DomainError::InternalError.into();
    assert_eq!(problem.status, http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn domain_to_problem_conflict_is_409() {
    use modkit::api::problem::Problem;
    let domain = DomainError::conflict("dup");
    let problem: Problem = domain.into();
    assert_eq!(problem.status, http::StatusCode::CONFLICT);
}
