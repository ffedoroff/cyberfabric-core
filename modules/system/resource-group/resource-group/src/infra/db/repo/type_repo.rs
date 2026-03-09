use async_trait::async_trait;
use modkit_db::secure::{
    DBRunner, SecureDeleteExt, SecureEntityExt, SecureUpdateExt, secure_insert,
};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::domain::error::DomainError;
use crate::infra::db::entity::resource_group_type;

fn db_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::database_err(e.to_string())
}

#[async_trait]
pub trait TypeRepository: Send + Sync {
    async fn find_by_code<C: DBRunner>(
        &self,
        conn: &C,
        code: &str,
    ) -> Result<Option<resource_group_type::Model>, DomainError>;

    async fn list<C: DBRunner>(
        &self,
        conn: &C,
    ) -> Result<Vec<resource_group_type::Model>, DomainError>;

    async fn list_filtered<C: DBRunner>(
        &self,
        conn: &C,
        filter_expr: Option<&str>,
        top: i32,
        skip: i32,
    ) -> Result<Vec<resource_group_type::Model>, DomainError>;

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_type::ActiveModel,
    ) -> Result<resource_group_type::Model, DomainError>;

    async fn update<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_type::ActiveModel,
    ) -> Result<resource_group_type::Model, DomainError>;

    async fn delete<C: DBRunner>(&self, conn: &C, code: &str) -> Result<(), DomainError>;
}

#[derive(Clone)]
pub struct TypeRepositoryImpl;

fn unconstrained_scope() -> AccessScope {
    AccessScope::allow_all()
}

#[async_trait]
impl TypeRepository for TypeRepositoryImpl {
    async fn find_by_code<C: DBRunner>(
        &self,
        conn: &C,
        code: &str,
    ) -> Result<Option<resource_group_type::Model>, DomainError> {
        resource_group_type::Entity::find_by_id(code.to_owned())
            .secure()
            .scope_with(&unconstrained_scope())
            .one(conn)
            .await
            .map_err(db_err)
    }

    async fn list<C: DBRunner>(
        &self,
        conn: &C,
    ) -> Result<Vec<resource_group_type::Model>, DomainError> {
        resource_group_type::Entity::find()
            .secure()
            .scope_with(&unconstrained_scope())
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn list_filtered<C: DBRunner>(
        &self,
        conn: &C,
        filter_expr: Option<&str>,
        top: i32,
        skip: i32,
    ) -> Result<Vec<resource_group_type::Model>, DomainError> {
        use sea_orm::QuerySelect;

        let mut query = resource_group_type::Entity::find();

        // Apply filter on `code` field
        if let Some(raw_filter) = filter_expr {
            let condition = parse_code_filter(raw_filter)?;
            query = query.filter(condition);
        }

        // Order by code ASC for deterministic pagination
        query = query.order_by_asc(resource_group_type::Column::Code);

        query
            .offset(u64::try_from(skip).unwrap_or(0))
            .limit(u64::try_from(top).unwrap_or(50))
            .secure()
            .scope_with(&unconstrained_scope())
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_type::ActiveModel,
    ) -> Result<resource_group_type::Model, DomainError> {
        let scope = unconstrained_scope();
        secure_insert::<resource_group_type::Entity>(model, &scope, conn)
            .await
            .map_err(db_err)
    }

    async fn update<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_type::ActiveModel,
    ) -> Result<resource_group_type::Model, DomainError> {
        let code = match &model.code {
            sea_orm::ActiveValue::Set(v) | sea_orm::ActiveValue::Unchanged(v) => v.clone(),
            sea_orm::ActiveValue::NotSet => {
                return Err(DomainError::validation("code is required for update"));
            }
        };

        let scope = unconstrained_scope();
        let mut update = resource_group_type::Entity::update_many()
            .filter(resource_group_type::Column::Code.eq(&code));

        if let sea_orm::ActiveValue::Set(ref v) = model.parents {
            update = update.col_expr(
                resource_group_type::Column::Parents,
                sea_orm::sea_query::Expr::value(sea_orm::Value::Json(Some(Box::new(v.clone())))),
            );
        }
        if let sea_orm::ActiveValue::Set(ref v) = model.modified {
            update = update.col_expr(
                resource_group_type::Column::Modified,
                sea_orm::sea_query::Expr::value(*v),
            );
        }

        update
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;

        self.find_by_code(conn, &code)
            .await?
            .ok_or_else(|| DomainError::TypeNotFound { code: code.clone() })
    }

    async fn delete<C: DBRunner>(&self, conn: &C, code: &str) -> Result<(), DomainError> {
        let scope = unconstrained_scope();
        resource_group_type::Entity::delete_many()
            .filter(resource_group_type::Column::Code.eq(code))
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

/// Parse a simple `OData` `$filter` expression on the `code` field.
///
/// Supported forms:
/// - `code eq 'value'`
/// - `code ne 'value'`
/// - `code in ('v1', 'v2', ...)`
///
/// Returns a `sea_orm::Condition` or a validation error.
fn parse_code_filter(raw: &str) -> Result<sea_orm::Condition, DomainError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(sea_orm::Condition::all());
    }

    // Try parsing with modkit-odata filter parser
    let parsed = modkit_odata::parse_filter_string(raw).map_err(|e| DomainError::Validation {
        message: format!("Invalid $filter expression: {e}"),
    })?;

    // Convert parsed AST to SeaORM condition for the `code` column
    ast_to_condition(parsed.as_expr())
}

/// Convert an `OData` filter AST expression to a `sea_orm::Condition`.
/// Only supports filtering on the `code` field.
fn ast_to_condition(expr: &modkit_odata::ast::Expr) -> Result<sea_orm::Condition, DomainError> {
    use modkit_odata::ast::{CompareOperator, Expr};
    use sea_orm::Condition;

    match expr {
        Expr::Compare(left, op, right) => {
            let field_name = extract_identifier(left)?;
            if field_name != "code" {
                return Err(DomainError::Validation {
                    message: format!(
                        "Filtering on field '{field_name}' is not supported; only 'code' is allowed"
                    ),
                });
            }
            let value = extract_string_value(right)?;
            let col = resource_group_type::Column::Code;
            match op {
                CompareOperator::Eq => Ok(Condition::all().add(col.eq(&value))),
                CompareOperator::Ne => Ok(Condition::all().add(col.ne(&value))),
                _ => Err(DomainError::Validation {
                    message:
                        "Unsupported operator for 'code' filter; only eq, ne, and in are supported"
                            .to_owned(),
                }),
            }
        }
        Expr::In(left, values) => {
            let field_name = extract_identifier(left)?;
            if field_name != "code" {
                return Err(DomainError::Validation {
                    message: format!(
                        "Filtering on field '{field_name}' is not supported; only 'code' is allowed"
                    ),
                });
            }
            let string_values: Vec<String> = values
                .iter()
                .map(extract_value_string)
                .collect::<Result<_, _>>()?;
            let col = resource_group_type::Column::Code;
            Ok(Condition::all().add(col.is_in(string_values)))
        }
        Expr::And(left, right) => {
            let left_cond = ast_to_condition(left)?;
            let right_cond = ast_to_condition(right)?;
            let mut cond = Condition::all();
            cond = cond.add(left_cond);
            cond = cond.add(right_cond);
            Ok(cond)
        }
        Expr::Or(left, right) => {
            let left_cond = ast_to_condition(left)?;
            let right_cond = ast_to_condition(right)?;
            let mut cond = Condition::any();
            cond = cond.add(left_cond);
            cond = cond.add(right_cond);
            Ok(cond)
        }
        _ => Err(DomainError::Validation {
            message: "Unsupported filter expression".into(),
        }),
    }
}

fn extract_identifier(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    match expr {
        modkit_odata::ast::Expr::Identifier(name) => Ok(name.to_lowercase()),
        _ => Err(DomainError::Validation {
            message: "Expected field name in filter expression".into(),
        }),
    }
}

fn extract_string_value(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    extract_value_string(expr)
}

fn extract_value_string(expr: &modkit_odata::ast::Expr) -> Result<String, DomainError> {
    match expr {
        modkit_odata::ast::Expr::Value(modkit_odata::ast::Value::String(s)) => Ok(s.clone()),
        _ => Err(DomainError::Validation {
            message: "Expected string value in filter expression".into(),
        }),
    }
}
