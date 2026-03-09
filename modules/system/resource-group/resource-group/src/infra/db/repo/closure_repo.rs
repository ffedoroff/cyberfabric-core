use async_trait::async_trait;
use modkit_db::secure::{DBRunner, SecureDeleteExt, SecureEntityExt, secure_insert};
use modkit_security::AccessScope;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::infra::db::entity::resource_group_closure;

fn db_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::database_err(e.to_string())
}

/// `AccessScope` for unrestricted entities — allows all operations.
fn unconstrained_scope() -> AccessScope {
    AccessScope::allow_all()
}

#[async_trait]
pub trait ClosureRepository: Send + Sync {
    async fn find_ancestors<C: DBRunner>(
        &self,
        conn: &C,
        descendant_id: Uuid,
    ) -> Result<Vec<resource_group_closure::Model>, DomainError>;

    async fn find_descendants<C: DBRunner>(
        &self,
        conn: &C,
        ancestor_id: Uuid,
    ) -> Result<Vec<resource_group_closure::Model>, DomainError>;

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_closure::ActiveModel,
    ) -> Result<resource_group_closure::Model, DomainError>;

    async fn delete_by_descendant<C: DBRunner>(
        &self,
        conn: &C,
        descendant_id: Uuid,
    ) -> Result<u64, DomainError>;

    async fn exists_path<C: DBRunner>(
        &self,
        conn: &C,
        ancestor_id: Uuid,
        descendant_id: Uuid,
    ) -> Result<bool, DomainError>;

    /// Delete ALL closure rows involving a node (ancestor or descendant).
    async fn delete_all_for_node<C: DBRunner>(
        &self,
        conn: &C,
        node_id: Uuid,
    ) -> Result<u64, DomainError>;

    /// Delete external ancestor paths for a set of subtree nodes.
    /// Removes rows where `descendant_id` IN `subtree_ids` AND `ancestor_id` NOT IN `subtree_ids`.
    /// Preserves internal subtree structure.
    async fn delete_external_ancestor_paths<C: DBRunner>(
        &self,
        conn: &C,
        subtree_ids: &[Uuid],
    ) -> Result<u64, DomainError>;
}

#[derive(Clone)]
pub struct ClosureRepositoryImpl;

#[async_trait]
impl ClosureRepository for ClosureRepositoryImpl {
    async fn find_ancestors<C: DBRunner>(
        &self,
        conn: &C,
        descendant_id: Uuid,
    ) -> Result<Vec<resource_group_closure::Model>, DomainError> {
        let scope = unconstrained_scope();
        resource_group_closure::Entity::find()
            .filter(resource_group_closure::Column::DescendantId.eq(descendant_id))
            .order_by_asc(resource_group_closure::Column::Depth)
            .secure()
            .scope_with(&scope)
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn find_descendants<C: DBRunner>(
        &self,
        conn: &C,
        ancestor_id: Uuid,
    ) -> Result<Vec<resource_group_closure::Model>, DomainError> {
        let scope = unconstrained_scope();
        resource_group_closure::Entity::find()
            .filter(resource_group_closure::Column::AncestorId.eq(ancestor_id))
            .order_by_asc(resource_group_closure::Column::Depth)
            .secure()
            .scope_with(&scope)
            .all(conn)
            .await
            .map_err(db_err)
    }

    async fn insert<C: DBRunner>(
        &self,
        conn: &C,
        model: resource_group_closure::ActiveModel,
    ) -> Result<resource_group_closure::Model, DomainError> {
        let scope = unconstrained_scope();
        secure_insert::<resource_group_closure::Entity>(model, &scope, conn)
            .await
            .map_err(db_err)
    }

    async fn delete_by_descendant<C: DBRunner>(
        &self,
        conn: &C,
        descendant_id: Uuid,
    ) -> Result<u64, DomainError> {
        let scope = unconstrained_scope();
        let result = resource_group_closure::Entity::delete_many()
            .filter(resource_group_closure::Column::DescendantId.eq(descendant_id))
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected)
    }

    async fn exists_path<C: DBRunner>(
        &self,
        conn: &C,
        ancestor_id: Uuid,
        descendant_id: Uuid,
    ) -> Result<bool, DomainError> {
        let scope = unconstrained_scope();
        let count = resource_group_closure::Entity::find()
            .filter(
                Condition::all()
                    .add(resource_group_closure::Column::AncestorId.eq(ancestor_id))
                    .add(resource_group_closure::Column::DescendantId.eq(descendant_id)),
            )
            .secure()
            .scope_with(&scope)
            .count(conn)
            .await
            .map_err(db_err)?;
        Ok(count > 0)
    }

    async fn delete_all_for_node<C: DBRunner>(
        &self,
        conn: &C,
        node_id: Uuid,
    ) -> Result<u64, DomainError> {
        let scope = unconstrained_scope();
        let result = resource_group_closure::Entity::delete_many()
            .filter(
                Condition::any()
                    .add(resource_group_closure::Column::AncestorId.eq(node_id))
                    .add(resource_group_closure::Column::DescendantId.eq(node_id)),
            )
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected)
    }

    async fn delete_external_ancestor_paths<C: DBRunner>(
        &self,
        conn: &C,
        subtree_ids: &[Uuid],
    ) -> Result<u64, DomainError> {
        let scope = unconstrained_scope();
        let result = resource_group_closure::Entity::delete_many()
            .filter(
                Condition::all()
                    .add(
                        resource_group_closure::Column::DescendantId
                            .is_in(subtree_ids.to_vec()),
                    )
                    .add(
                        resource_group_closure::Column::AncestorId
                            .is_not_in(subtree_ids.to_vec()),
                    ),
            )
            .secure()
            .scope_with(&scope)
            .exec(conn)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected)
    }
}
