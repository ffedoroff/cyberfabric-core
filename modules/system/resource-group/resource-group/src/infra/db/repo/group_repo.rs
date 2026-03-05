use async_trait::async_trait;
use modkit_db::secure::{DBRunner, SecureEntityExt, secure_insert, secure_update_with_scope};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
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
}
