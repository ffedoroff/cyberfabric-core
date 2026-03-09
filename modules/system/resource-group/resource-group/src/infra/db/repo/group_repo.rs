use async_trait::async_trait;
use modkit_db::secure::{DBRunner, SecureEntityExt, secure_insert, secure_update_with_scope};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::infra::db::entity::resource_group;

fn db_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::database_err(e.to_string())
}

#[async_trait]
pub trait GroupRepository: Send + Sync {
    async fn find_by_id<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        id: Uuid,
    ) -> Result<Option<resource_group::Model>, DomainError>;

    async fn list<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
    ) -> Result<Vec<resource_group::Model>, DomainError>;

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        model: resource_group::ActiveModel,
    ) -> Result<resource_group::Model, DomainError>;

    async fn update<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        id: Uuid,
        model: resource_group::ActiveModel,
    ) -> Result<resource_group::Model, DomainError>;

    async fn delete<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        id: Uuid,
    ) -> Result<(), DomainError>;

    async fn count_by_type<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        type_code: &str,
    ) -> Result<u64, DomainError>;

    async fn count_children<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        parent_id: Uuid,
    ) -> Result<u64, DomainError>;

    async fn list_filtered<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        filter_expr: Option<&str>,
        top: i32,
        skip: i32,
    ) -> Result<Vec<resource_group::Model>, DomainError>;
}

#[derive(Clone)]
pub struct GroupRepositoryImpl;

#[async_trait]
impl GroupRepository for GroupRepositoryImpl {
    async fn find_by_id<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        id: Uuid,
    ) -> Result<Option<resource_group::Model>, DomainError> {
        resource_group::Entity::find_by_id(id)
            .secure()
            .scope_with(scope)
            .one(conn)
            .await
            .map_err(db_err)
    }

    async fn list<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
    ) -> Result<Vec<resource_group::Model>, DomainError> {
        resource_group::Entity::find()
            .secure()
            .scope_with(scope)
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        model: resource_group::ActiveModel,
    ) -> Result<resource_group::Model, DomainError> {
        secure_insert::<resource_group::Entity>(model, scope, conn)
            .await
            .map_err(db_err)
    }

    async fn update<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        id: Uuid,
        model: resource_group::ActiveModel,
    ) -> Result<resource_group::Model, DomainError> {
        secure_update_with_scope::<resource_group::Entity>(model, scope, id, conn)
            .await
            .map_err(db_err)
    }

    async fn delete<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        id: Uuid,
    ) -> Result<(), DomainError> {
        use modkit_db::secure::SecureDeleteExt;
        resource_group::Entity::delete_many()
            .filter(resource_group::Column::Id.eq(id))
            .secure()
            .scope_with(scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn count_by_type<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        type_code: &str,
    ) -> Result<u64, DomainError> {
        resource_group::Entity::find()
            .filter(resource_group::Column::GroupType.eq(type_code))
            .secure()
            .scope_with(scope)
            .count(conn)
            .await
            .map_err(db_err)
    }

    async fn count_children<C: DBRunner>(
        &self,
        conn: &C,
        scope: &AccessScope,
        parent_id: Uuid,
    ) -> Result<u64, DomainError> {
        resource_group::Entity::find()
            .filter(resource_group::Column::ParentId.eq(parent_id))
            .secure()
            .scope_with(scope)
            .count(conn)
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
    ) -> Result<Vec<resource_group::Model>, DomainError> {
        let mut query = resource_group::Entity::find();

        if let Some(raw_filter) = filter_expr {
            let condition = parse_group_filter(raw_filter)?;
            query = query.filter(condition);
        }

        query = query.order_by_asc(resource_group::Column::Id);

        query
            .offset(u64::try_from(skip).unwrap_or(0))
            .limit(u64::try_from(top).unwrap_or(50))
            .secure()
            .scope_with(scope)
            .all(conn)
            .await
            .map_err(db_err)
    }
}

// ── OData filter parsing for groups ──────────────────────────────────────

fn parse_group_filter(raw: &str) -> Result<sea_orm::Condition, DomainError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(sea_orm::Condition::all());
    }

    let parsed = modkit_odata::parse_filter_string(raw).map_err(|e| DomainError::Validation {
        message: format!("Invalid $filter expression: {e}"),
    })?;

    group_ast_to_condition(parsed.as_expr())
}

fn group_ast_to_condition(
    expr: &modkit_odata::ast::Expr,
) -> Result<sea_orm::Condition, DomainError> {
    use modkit_odata::ast::{CompareOperator, Expr};
    use sea_orm::Condition;

    match expr {
        Expr::Compare(left, op, right) => {
            let field_name = group_extract_identifier(left)?;
            let col = group_field_to_column(&field_name)?;
            let value = group_extract_value(right)?;
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
            let field_name = group_extract_identifier(left)?;
            let col = group_field_to_column(&field_name)?;
            let string_values: Vec<String> = values
                .iter()
                .map(group_extract_value_string)
                .collect::<Result<_, _>>()?;
            Ok(Condition::all().add(col.is_in(string_values)))
        }
        Expr::And(left, right) => {
            let mut cond = Condition::all();
            cond = cond.add(group_ast_to_condition(left)?);
            cond = cond.add(group_ast_to_condition(right)?);
            Ok(cond)
        }
        Expr::Or(left, right) => {
            let mut cond = Condition::any();
            cond = cond.add(group_ast_to_condition(left)?);
            cond = cond.add(group_ast_to_condition(right)?);
            Ok(cond)
        }
        _ => Err(DomainError::Validation {
            message: "Unsupported filter expression".into(),
        }),
    }
}

fn group_field_to_column(
    field: &str,
) -> Result<resource_group::Column, DomainError> {
    match field {
        "group_type" => Ok(resource_group::Column::GroupType),
        "parent_id" => Ok(resource_group::Column::ParentId),
        "group_id" | "id" => Ok(resource_group::Column::Id),
        "name" => Ok(resource_group::Column::Name),
        "external_id" => Ok(resource_group::Column::ExternalId),
        other => Err(DomainError::Validation {
            message: format!(
                "Filtering on field '{other}' is not supported; allowed: group_type, parent_id, group_id, name, external_id"
            ),
        }),
    }
}

fn group_extract_identifier(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    match expr {
        modkit_odata::ast::Expr::Identifier(name) => Ok(name.to_lowercase()),
        _ => Err(DomainError::Validation {
            message: "Expected field name in filter expression".into(),
        }),
    }
}

fn group_extract_value(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    group_extract_value_string(expr)
}

fn group_extract_value_string(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    match expr {
        modkit_odata::ast::Expr::Value(modkit_odata::ast::Value::String(s)) => Ok(s.clone()),
        _ => Err(DomainError::Validation {
            message: "Expected string value in filter expression".into(),
        }),
    }
}
