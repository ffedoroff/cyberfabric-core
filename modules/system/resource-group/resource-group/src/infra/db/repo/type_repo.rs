use async_trait::async_trait;
use modkit_db::secure::{
    DBRunner, SecureDeleteExt, SecureEntityExt, SecureUpdateExt, secure_insert,
};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

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
                sea_orm::sea_query::Expr::value(sea_orm::Value::Array(
                    sea_orm::sea_query::ArrayType::String,
                    Some(Box::new(
                        v.iter()
                            .map(|s| sea_orm::Value::String(Some(Box::new(s.clone()))))
                            .collect(),
                    )),
                )),
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
