//! Shared domain validation utilities.

use crate::domain::error::DomainError;

/// GTS type path prefix required for resource group types.
pub const RG_TYPE_PREFIX: &str = "gts.x.system.rg.type.v1~";

/// Validate a GTS type code: non-empty, correct prefix, length limit.
// @cpt-algo:cpt-cf-resource-group-algo-sdk-foundation-validate-gts-type-path:p1
// @cpt-algo:cpt-cf-resource-group-algo-type-mgmt-validate-type-input:p1
pub fn validate_type_code(code: &str) -> Result<(), DomainError> {
    if code.is_empty() {
        return Err(DomainError::validation("Type code must not be empty"));
    }
    if !code.starts_with(RG_TYPE_PREFIX) {
        return Err(DomainError::validation(format!(
            "Type code must start with prefix '{RG_TYPE_PREFIX}', got: '{code}'"
        )));
    }
    if code.len() > 1024 {
        return Err(DomainError::validation(
            "Type code must not exceed 1024 characters",
        ));
    }
    Ok(())
}
