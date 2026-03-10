use async_trait::async_trait;
use modkit_db::secure::{DBRunner, SecureDeleteExt, SecureEntityExt, build_scope_condition, secure_insert};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use sea_orm::sea_query::IntoColumnRef;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::infra::db::entity::{resource_group, resource_group_membership};

fn db_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::database_err(e.to_string())
}

/// `AccessScope` for unrestricted entities — allows all operations.
fn unconstrained_scope() -> AccessScope {
    AccessScope::allow_all()
}

#[async_trait]
pub trait MembershipRepository: Send + Sync {
    async fn find_by_group<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
    ) -> Result<Vec<resource_group_membership::Model>, DomainError>;

    async fn find_by_resource<C: DBRunner>(
        &self,
        conn: &C,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<resource_group_membership::Model>, DomainError>;

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_membership::ActiveModel,
    ) -> Result<resource_group_membership::Model, DomainError>;

    async fn delete<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), DomainError>;

    async fn count_by_group<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
    ) -> Result<u64, DomainError>;

    /// Delete ALL memberships for a given group.
    async fn delete_all_by_group<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
    ) -> Result<u64, DomainError>;

    /// Find a single membership by its composite key.
    async fn find_by_key<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<resource_group_membership::Model>, DomainError>;

    /// List memberships with `OData` filter, pagination, and deterministic ordering.
    /// Memberships are scoped via the group's `tenant_id` (JOIN to `resource_group`).
    async fn list_filtered<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        filter_expr: Option<&str>,
        top: i32,
        skip: i32,
    ) -> Result<Vec<resource_group_membership::Model>, DomainError>;
}

#[derive(Clone)]
pub struct MembershipRepositoryImpl;

#[async_trait]
impl MembershipRepository for MembershipRepositoryImpl {
    async fn find_by_group<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
    ) -> Result<Vec<resource_group_membership::Model>, DomainError> {
        let scope = unconstrained_scope();
        resource_group_membership::Entity::find()
            .filter(resource_group_membership::Column::GroupId.eq(group_id))
            .secure()
            .scope_with(&scope)
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn find_by_resource<C: DBRunner>(
        &self,
        conn: &C,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<resource_group_membership::Model>, DomainError> {
        let scope = unconstrained_scope();
        resource_group_membership::Entity::find()
            .filter(
                Condition::all()
                    .add(resource_group_membership::Column::ResourceType.eq(resource_type))
                    .add(resource_group_membership::Column::ResourceId.eq(resource_id)),
            )
            .secure()
            .scope_with(&scope)
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_membership::ActiveModel,
    ) -> Result<resource_group_membership::Model, DomainError> {
        let scope = unconstrained_scope();
        secure_insert::<resource_group_membership::Entity>(model, &scope, conn)
            .await
            .map_err(db_err)
    }

    async fn delete<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), DomainError> {
        let scope = unconstrained_scope();
        resource_group_membership::Entity::delete_many()
            .filter(
                Condition::all()
                    .add(resource_group_membership::Column::GroupId.eq(group_id))
                    .add(resource_group_membership::Column::ResourceType.eq(resource_type))
                    .add(resource_group_membership::Column::ResourceId.eq(resource_id)),
            )
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn count_by_group<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
    ) -> Result<u64, DomainError> {
        let scope = unconstrained_scope();
        resource_group_membership::Entity::find()
            .filter(resource_group_membership::Column::GroupId.eq(group_id))
            .secure()
            .scope_with(&scope)
            .count(conn)
            .await
            .map_err(db_err)
    }

    async fn delete_all_by_group<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
    ) -> Result<u64, DomainError> {
        let scope = unconstrained_scope();
        let result = resource_group_membership::Entity::delete_many()
            .filter(resource_group_membership::Column::GroupId.eq(group_id))
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected)
    }

    async fn find_by_key<C: DBRunner>(
        &self,
        conn: &C,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<resource_group_membership::Model>, DomainError> {
        let scope = unconstrained_scope();
        resource_group_membership::Entity::find()
            .filter(
                Condition::all()
                    .add(resource_group_membership::Column::GroupId.eq(group_id))
                    .add(resource_group_membership::Column::ResourceType.eq(resource_type))
                    .add(resource_group_membership::Column::ResourceId.eq(resource_id)),
            )
            .secure()
            .scope_with(&scope)
            .one(conn)
            .await
            .map_err(db_err)
    }

    async fn list_filtered<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        filter_expr: Option<&str>,
        top: i32,
        skip: i32,
    ) -> Result<Vec<resource_group_membership::Model>, DomainError> {
        let unrestricted_scope = unconstrained_scope();
        let mut query = resource_group_membership::Entity::find();

        // Membership entity is unrestricted (no tenant_col). Scope via group's tenant_id
        // using the same scope condition that SecureORM applies to resource_group queries.
        // This handles all scope filter types: Eq, In, InTenantSubtree, InGroup, etc.
        if scope.is_deny_all() {
            return Ok(vec![]);
        }
        if !scope.is_unconstrained() {
            let scope_cond = build_scope_condition::<resource_group::Entity>(scope);
            let mut sub = sea_orm::sea_query::Query::select();
            sub.column(resource_group::Column::Id)
                .from(resource_group::Entity)
                .cond_where(scope_cond);
            query = query.filter(
                sea_orm::sea_query::Expr::col(
                    resource_group_membership::Column::GroupId.into_column_ref(),
                )
                .in_subquery(sub),
            );
        }

        if let Some(raw_filter) = filter_expr {
            let condition = parse_membership_filter(raw_filter)?;
            query = query.filter(condition);
        }

        // Sort by group_id ASC, resource_type ASC, resource_id ASC
        query = query
            .order_by_asc(resource_group_membership::Column::GroupId)
            .order_by_asc(resource_group_membership::Column::ResourceType)
            .order_by_asc(resource_group_membership::Column::ResourceId);

        query
            .offset(u64::try_from(skip).unwrap_or(0))
            .limit(u64::try_from(top).unwrap_or(50))
            .secure()
            .scope_with(&unrestricted_scope)
            .all(conn)
            .await
            .map_err(db_err)
    }
}

// ── OData filter parsing for memberships ─────────────────────────────────

fn parse_membership_filter(raw: &str) -> Result<sea_orm::Condition, DomainError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(sea_orm::Condition::all());
    }

    let parsed = modkit_odata::parse_filter_string(raw).map_err(|e| DomainError::Validation {
        message: format!("Invalid $filter expression: {e}"),
    })?;

    membership_ast_to_condition(parsed.as_expr())
}

fn membership_ast_to_condition(
    expr: &modkit_odata::ast::Expr,
) -> Result<sea_orm::Condition, DomainError> {
    use modkit_odata::ast::{CompareOperator, Expr};
    use sea_orm::Condition;

    match expr {
        Expr::Compare(left, op, right) => {
            let field_name = membership_extract_identifier(left)?;
            let col = membership_field_to_column(&field_name)?;
            let value = membership_extract_value(right)?;
            match op {
                CompareOperator::Eq => Ok(Condition::all().add(col.eq(&value))),
                CompareOperator::Ne => Ok(Condition::all().add(col.ne(&value))),
                _ => Err(DomainError::Validation {
                    message: format!(
                        "Unsupported operator for '{field_name}'; only eq, ne, and in are supported"
                    ),
                }),
            }
        }
        Expr::In(left, values) => {
            let field_name = membership_extract_identifier(left)?;
            let col = membership_field_to_column(&field_name)?;
            let string_values: Vec<String> = values
                .iter()
                .map(membership_extract_value_string)
                .collect::<Result<_, _>>()?;
            Ok(Condition::all().add(col.is_in(string_values)))
        }
        Expr::And(left, right) => {
            let mut cond = Condition::all();
            cond = cond.add(membership_ast_to_condition(left)?);
            cond = cond.add(membership_ast_to_condition(right)?);
            Ok(cond)
        }
        Expr::Or(left, right) => {
            let mut cond = Condition::any();
            cond = cond.add(membership_ast_to_condition(left)?);
            cond = cond.add(membership_ast_to_condition(right)?);
            Ok(cond)
        }
        _ => Err(DomainError::Validation {
            message: "Unsupported filter expression".into(),
        }),
    }
}

fn membership_field_to_column(
    field: &str,
) -> Result<resource_group_membership::Column, DomainError> {
    match field {
        "group_id" => Ok(resource_group_membership::Column::GroupId),
        "resource_type" => Ok(resource_group_membership::Column::ResourceType),
        "resource_id" => Ok(resource_group_membership::Column::ResourceId),
        other => Err(DomainError::Validation {
            message: format!(
                "Filtering on field '{other}' is not supported; allowed: group_id, resource_type, resource_id"
            ),
        }),
    }
}

fn membership_extract_identifier(
    expr: &modkit_odata::ast::Expr,
) -> Result<String, DomainError> {
    match expr {
        modkit_odata::ast::Expr::Identifier(name) => Ok(name.to_lowercase()),
        _ => Err(DomainError::Validation {
            message: "Expected field name in filter expression".into(),
        }),
    }
}

fn membership_extract_value(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    membership_extract_value_string(expr)
}

fn membership_extract_value_string(
    expr: &modkit_odata::ast::Expr,
) -> Result<String, DomainError> {
    match expr {
        modkit_odata::ast::Expr::Value(modkit_odata::ast::Value::String(s)) => Ok(s.clone()),
        _ => Err(DomainError::Validation {
            message: "Expected string value in filter expression".into(),
        }),
    }
}
