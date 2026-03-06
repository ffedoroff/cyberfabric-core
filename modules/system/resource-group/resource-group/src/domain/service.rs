use std::sync::Arc;

use async_trait::async_trait;
use modkit_security::SecurityContext;
use resource_group_sdk::{
    AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery, Page,
    RemoveMembershipRequest, ResourceGroup, ResourceGroupClient, ResourceGroupError,
    ResourceGroupMembership, ResourceGroupReadHierarchy, ResourceGroupType, ResourceGroupWithDepth,
    UpdateGroupRequest, UpdateTypeRequest,
};
use uuid::Uuid;

use crate::domain::type_service::TypeService;
use crate::infra::db::repo::group_repo::GroupRepositoryImpl;
use crate::infra::db::repo::type_repo::TypeRepositoryImpl;

/// Unified service facade implementing both SDK traits.
/// Backed by domain services and repositories.
pub struct RgService {
    type_service: TypeService<TypeRepositoryImpl, GroupRepositoryImpl>,
    _db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
}

impl RgService {
    #[must_use]
    pub fn new(db: Arc<modkit_db::DBProvider<modkit_db::DbError>>) -> Self {
        let type_service =
            TypeService::new(TypeRepositoryImpl, GroupRepositoryImpl, Arc::clone(&db));
        Self {
            type_service,
            _db: db,
        }
    }
}

// Stub for non-type methods — domain logic is out of scope for Feature 0002.
fn not_implemented() -> ResourceGroupError {
    ResourceGroupError::Internal
}

#[async_trait]
impl ResourceGroupClient for RgService {
    async fn create_type(
        &self,
        _ctx: &SecurityContext,
        request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        self.type_service
            .create_type(request)
            .await
            .map_err(Into::into)
    }

    async fn get_type(
        &self,
        _ctx: &SecurityContext,
        code: &str,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        self.type_service.get_type(code).await.map_err(Into::into)
    }

    async fn list_types(
        &self,
        _ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupType>, ResourceGroupError> {
        self.type_service
            .list_types(query)
            .await
            .map_err(Into::into)
    }

    async fn update_type(
        &self,
        _ctx: &SecurityContext,
        code: &str,
        request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        self.type_service
            .update_type(code, request)
            .await
            .map_err(Into::into)
    }

    async fn delete_type(
        &self,
        _ctx: &SecurityContext,
        code: &str,
    ) -> Result<(), ResourceGroupError> {
        self.type_service
            .delete_type(code)
            .await
            .map_err(Into::into)
    }

    async fn create_group(
        &self,
        _ctx: &SecurityContext,
        _request: CreateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn get_group(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_groups(
        &self,
        _ctx: &SecurityContext,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroup>, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn update_group(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
        _request: UpdateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn delete_group(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
        _force: bool,
    ) -> Result<(), ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_group_depth(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn add_membership(
        &self,
        _ctx: &SecurityContext,
        _request: AddMembershipRequest,
    ) -> Result<ResourceGroupMembership, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn remove_membership(
        &self,
        _ctx: &SecurityContext,
        _request: RemoveMembershipRequest,
    ) -> Result<(), ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_memberships(
        &self,
        _ctx: &SecurityContext,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        Err(not_implemented())
    }
}

#[async_trait]
impl ResourceGroupReadHierarchy for RgService {
    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        ResourceGroupClient::list_group_depth(self, ctx, group_id, query).await
    }
}
