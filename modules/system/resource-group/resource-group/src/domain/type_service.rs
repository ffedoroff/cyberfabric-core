use std::sync::Arc;

use modkit_db::secure::DBRunner;
use time::OffsetDateTime;

use crate::domain::error::DomainError;
use crate::infra::db::entity::resource_group_type;
use crate::infra::db::repo::group_repo::GroupRepository;
use crate::infra::db::repo::type_repo::TypeRepository;
use resource_group_sdk::{
    CreateTypeRequest, ListQuery, Page, PageInfo, ResourceGroupType, UpdateTypeRequest,
};

/// Maximum length for type code (aligned with practical TEXT column limit).
const MAX_TYPE_CODE_LENGTH: usize = 255;

/// Default page size for list operations.
const DEFAULT_TOP: i32 = 50;
/// Maximum page size for list operations.
const MAX_TOP: i32 = 300;

// @cpt-flow:cpt-cf-resource-group-flow-type-create:p1
// @cpt-flow:cpt-cf-resource-group-flow-type-get:p2
// @cpt-flow:cpt-cf-resource-group-flow-type-list:p2
// @cpt-flow:cpt-cf-resource-group-flow-type-update:p1
// @cpt-flow:cpt-cf-resource-group-flow-type-delete:p1
// @cpt-flow:cpt-cf-resource-group-flow-type-seed:p1
// @cpt-algo:cpt-cf-resource-group-algo-type-code-validation:p1
// @cpt-algo:cpt-cf-resource-group-algo-type-delete-guard:p1
// @cpt-req:cpt-cf-resource-group-dod-type-create:p1
// @cpt-req:cpt-cf-resource-group-dod-type-read:p1
// @cpt-req:cpt-cf-resource-group-dod-type-update:p1
// @cpt-req:cpt-cf-resource-group-dod-type-delete:p1
// @cpt-req:cpt-cf-resource-group-dod-type-uniqueness:p1
// @cpt-req:cpt-cf-resource-group-dod-type-seed:p1

pub struct TypeService<TR: TypeRepository, GR: GroupRepository> {
    type_repo: TR,
    group_repo: GR,
    db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
}

#[allow(clippy::missing_errors_doc)]
impl<TR: TypeRepository, GR: GroupRepository> TypeService<TR, GR> {
    pub fn new(
        type_repo: TR,
        group_repo: GR,
        db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
    ) -> Self {
        Self {
            type_repo,
            group_repo,
            db,
        }
    }

    fn conn(&self) -> Result<impl DBRunner + '_, DomainError> {
        self.db
            .conn()
            .map_err(|e| DomainError::database(e.to_string()))
    }

    // @cpt-begin:cpt-cf-resource-group-algo-type-code-validation:p1:inst-codeval-1
    fn validate_type_code(raw_code: &str) -> Result<String, DomainError> {
        // Step 1: empty/blank check
        let trimmed = raw_code.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation {
                message: "Type code must not be empty".into(),
            });
        }

        // Step 2: max length check
        if trimmed.len() > MAX_TYPE_CODE_LENGTH {
            return Err(DomainError::Validation {
                message: format!(
                    "Type code exceeds maximum length of {MAX_TYPE_CODE_LENGTH} characters"
                ),
            });
        }

        // Step 4: normalize (lowercase)
        let normalized = trimmed.to_lowercase();

        // Step 3: invalid characters check (after normalization)
        if !normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(DomainError::Validation {
                message: "Type code contains invalid characters (only lowercase alphanumeric, hyphens, and underscores allowed)".into(),
            });
        }

        Ok(normalized)
    }
    // @cpt-end:cpt-cf-resource-group-algo-type-code-validation:p1:inst-codeval-1

    fn validate_parents(parents: &[String]) -> Result<(), DomainError> {
        if parents.is_empty() {
            return Err(DomainError::Validation {
                message: "Parents must contain at least one element".into(),
            });
        }
        Ok(())
    }

    // @cpt-begin:cpt-cf-resource-group-flow-type-create:p1:inst-type-create-1
    pub async fn create_type(
        &self,
        request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, DomainError> {
        // inst-type-create-2: validate code
        let normalized_code = Self::validate_type_code(&request.code)?;

        // inst-type-create-4: validate parents
        Self::validate_parents(&request.parents)?;

        let conn = self.conn()?;

        // inst-type-create-6: insert
        let now = OffsetDateTime::now_utc();
        let active_model = resource_group_type::ActiveModel {
            code: sea_orm::ActiveValue::Set(normalized_code.clone()),
            parents: sea_orm::ActiveValue::Set(request.parents),
            created: sea_orm::ActiveValue::Set(now),
            modified: sea_orm::ActiveValue::NotSet,
        };

        let model = self
            .type_repo
            .insert(&conn, active_model)
            .await
            .map_err(|e| {
                // inst-type-create-7: unique constraint violation → TypeAlreadyExists
                if is_unique_violation(&e) {
                    return DomainError::TypeAlreadyExists {
                        code: normalized_code.clone(),
                    };
                }
                e
            })?;

        // inst-type-create-8: return created type
        Ok(to_sdk_type(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-type-create:p1:inst-type-create-1

    // @cpt-begin:cpt-cf-resource-group-flow-type-get:p2:inst-type-get-1
    pub async fn get_type(&self, code: &str) -> Result<ResourceGroupType, DomainError> {
        let conn = self.conn()?;

        // inst-type-get-2: select by code
        let model = self
            .type_repo
            .find_by_code(&conn, code)
            .await?
            // inst-type-get-4: not found
            .ok_or_else(|| DomainError::TypeNotFound {
                code: code.to_owned(),
            })?;

        // inst-type-get-3a: return type
        Ok(to_sdk_type(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-type-get:p2:inst-type-get-1

    // @cpt-begin:cpt-cf-resource-group-flow-type-list:p2:inst-type-list-1
    pub async fn list_types(
        &self,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupType>, DomainError> {
        let conn = self.conn()?;

        let top = clamp_top(query.top);
        let skip = query.skip.unwrap_or(0).max(0);

        // inst-type-list-4: query with filter, order, pagination
        let models = self
            .type_repo
            .list_filtered(&conn, query.filter.as_deref(), top, skip)
            .await?;

        // inst-type-list-5: return page
        let items = models.into_iter().map(|m| to_sdk_type(&m)).collect();
        Ok(Page {
            items,
            page_info: PageInfo { top, skip },
        })
    }
    // @cpt-end:cpt-cf-resource-group-flow-type-list:p2:inst-type-list-1

    // @cpt-begin:cpt-cf-resource-group-flow-type-update:p1:inst-type-update-1
    pub async fn update_type(
        &self,
        code: &str,
        request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, DomainError> {
        let conn = self.conn()?;

        // inst-type-update-2: check type exists
        self.type_repo
            .find_by_code(&conn, code)
            .await?
            .ok_or_else(|| DomainError::TypeNotFound {
                code: code.to_owned(),
            })?;

        // inst-type-update-4: validate parents
        Self::validate_parents(&request.parents)?;

        // inst-type-update-6: update
        let active_model = resource_group_type::ActiveModel {
            code: sea_orm::ActiveValue::Unchanged(code.to_owned()),
            parents: sea_orm::ActiveValue::Set(request.parents),
            created: sea_orm::ActiveValue::NotSet,
            modified: sea_orm::ActiveValue::Set(Some(OffsetDateTime::now_utc())),
        };

        let model = self.type_repo.update(&conn, active_model).await?;

        // inst-type-update-7: return updated type
        Ok(to_sdk_type(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-type-update:p1:inst-type-update-1

    // @cpt-begin:cpt-cf-resource-group-flow-type-delete:p1:inst-type-delete-1
    pub async fn delete_type(&self, code: &str) -> Result<(), DomainError> {
        let conn = self.conn()?;

        // inst-type-delete-2: check type exists
        self.type_repo
            .find_by_code(&conn, code)
            .await?
            .ok_or_else(|| DomainError::TypeNotFound {
                code: code.to_owned(),
            })?;

        // inst-type-delete-4: usage guard
        self.check_type_usage_guard(&conn, code).await?;

        // inst-type-delete-6: delete
        self.type_repo.delete(&conn, code).await?;

        Ok(())
    }
    // @cpt-end:cpt-cf-resource-group-flow-type-delete:p1:inst-type-delete-1

    // @cpt-begin:cpt-cf-resource-group-algo-type-delete-guard:p1:inst-delguard-1
    async fn check_type_usage_guard(
        &self,
        conn: &impl DBRunner,
        code: &str,
    ) -> Result<(), DomainError> {
        let scope = modkit_security::AccessScope::allow_all();
        let count = self.group_repo.count_by_type(conn, &scope, code).await?;

        // inst-delguard-2: reject if references exist
        if count > 0 {
            #[allow(clippy::cast_possible_wrap)]
            return Err(DomainError::ActiveReferences {
                count: count as i64,
            });
        }
        Ok(())
    }
    // @cpt-end:cpt-cf-resource-group-algo-type-delete-guard:p1:inst-delguard-1

    // @cpt-begin:cpt-cf-resource-group-flow-type-seed:p1:inst-type-seed-1
    pub async fn seed_types(&self, types: Vec<CreateTypeRequest>) -> Result<(), DomainError> {
        let conn = self.conn()?;

        // inst-type-seed-2: for each type definition
        for type_def in types {
            // inst-type-seed-2a: validate code
            let normalized_code = Self::validate_type_code(&type_def.code)?;

            // inst-type-seed-2c: upsert
            let existing = self.type_repo.find_by_code(&conn, &normalized_code).await?;
            if let Some(_existing) = existing {
                // Update parents
                let active_model = resource_group_type::ActiveModel {
                    code: sea_orm::ActiveValue::Unchanged(normalized_code),
                    parents: sea_orm::ActiveValue::Set(type_def.parents),
                    created: sea_orm::ActiveValue::NotSet,
                    modified: sea_orm::ActiveValue::Set(Some(OffsetDateTime::now_utc())),
                };
                self.type_repo.update(&conn, active_model).await?;
            } else {
                // Insert new
                let now = OffsetDateTime::now_utc();
                let active_model = resource_group_type::ActiveModel {
                    code: sea_orm::ActiveValue::Set(normalized_code),
                    parents: sea_orm::ActiveValue::Set(type_def.parents),
                    created: sea_orm::ActiveValue::Set(now),
                    modified: sea_orm::ActiveValue::NotSet,
                };
                self.type_repo.insert(&conn, active_model).await?;
            }
        }

        // inst-type-seed-3: complete
        Ok(())
    }
    // @cpt-end:cpt-cf-resource-group-flow-type-seed:p1:inst-type-seed-1
}

fn to_sdk_type(model: &resource_group_type::Model) -> ResourceGroupType {
    ResourceGroupType {
        code: model.code.clone(),
        parents: model.parents.clone(),
    }
}

/// Check if a `DomainError` is caused by a unique constraint violation.
fn is_unique_violation(err: &DomainError) -> bool {
    match err {
        DomainError::Database { message } => {
            let lower = message.to_lowercase();
            lower.contains("unique constraint")
                || lower.contains("duplicate key")
                || lower.contains("unique_violation")
                || lower.contains("already exists")
        }
        _ => false,
    }
}

/// Clamp `$top` to valid range `[1..MAX_TOP]`, defaulting to `DEFAULT_TOP`.
fn clamp_top(top: Option<i32>) -> i32 {
    match top {
        Some(t) if t < 1 => DEFAULT_TOP,
        Some(t) if t > MAX_TOP => MAX_TOP,
        Some(t) => t,
        None => DEFAULT_TOP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to call validate_type_code without specifying generic params
    fn validate_code(raw: &str) -> Result<String, DomainError> {
        TypeService::<
            crate::infra::db::repo::type_repo::TypeRepositoryImpl,
            crate::infra::db::repo::group_repo::GroupRepositoryImpl,
        >::validate_type_code(raw)
    }

    fn validate_parents(parents: &[String]) -> Result<(), DomainError> {
        TypeService::<
            crate::infra::db::repo::type_repo::TypeRepositoryImpl,
            crate::infra::db::repo::group_repo::GroupRepositoryImpl,
        >::validate_parents(parents)
    }

    #[test]
    fn validate_type_code_normalizes_to_lowercase() {
        assert_eq!(validate_code("Tenant").unwrap(), "tenant");
        assert_eq!(validate_code("my-type_123").unwrap(), "my-type_123");
        assert_eq!(validate_code("UPPER").unwrap(), "upper");
    }

    #[test]
    fn validate_type_code_rejects_empty() {
        assert!(validate_code("").is_err());
        assert!(validate_code("   ").is_err());
    }

    #[test]
    fn validate_type_code_rejects_invalid_chars() {
        assert!(validate_code("my type!").is_err());
        assert!(validate_code("a.b").is_err());
        assert!(validate_code("a/b").is_err());
    }

    #[test]
    fn validate_type_code_rejects_too_long() {
        let long_code = "a".repeat(256);
        assert!(validate_code(&long_code).is_err());
    }

    #[test]
    fn validate_type_code_accepts_max_length() {
        let code = "a".repeat(255);
        assert!(validate_code(&code).is_ok());
    }

    #[test]
    fn validate_parents_rejects_empty() {
        assert!(validate_parents(&[]).is_err());
    }

    #[test]
    fn validate_parents_accepts_root_marker() {
        assert!(validate_parents(&["".to_string()]).is_ok());
    }

    #[test]
    fn validate_parents_accepts_multiple() {
        assert!(validate_parents(&["tenant".to_string(), "".to_string()]).is_ok());
    }

    #[test]
    fn clamp_top_defaults_and_bounds() {
        assert_eq!(clamp_top(None), DEFAULT_TOP);
        assert_eq!(clamp_top(Some(0)), DEFAULT_TOP);
        assert_eq!(clamp_top(Some(-5)), DEFAULT_TOP);
        assert_eq!(clamp_top(Some(10)), 10);
        assert_eq!(clamp_top(Some(300)), 300);
        assert_eq!(clamp_top(Some(500)), MAX_TOP);
    }

    #[test]
    fn is_unique_violation_detects_constraint_errors() {
        assert!(is_unique_violation(&DomainError::Database {
            message: "UNIQUE constraint failed: resource_group_type.code".into(),
        }));
        assert!(is_unique_violation(&DomainError::Database {
            message: "duplicate key value violates unique constraint".into(),
        }));
        assert!(!is_unique_violation(&DomainError::Database {
            message: "connection refused".into(),
        }));
        assert!(!is_unique_violation(&DomainError::Validation {
            message: "test".into(),
        }));
    }
}
