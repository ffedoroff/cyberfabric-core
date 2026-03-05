use async_trait::async_trait;
use modkit_db::secure::{DBRunner, SecureDeleteExt, SecureEntityExt, secure_insert};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::infra::db::entity::resource_group_membership;

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
}
