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

use crate::domain::group_service::GroupService;
use crate::domain::membership_service::MembershipService;
use crate::domain::type_service::TypeService;
use crate::infra::db::repo::closure_repo::ClosureRepositoryImpl;
use crate::infra::db::repo::group_repo::GroupRepositoryImpl;
use crate::infra::db::repo::membership_repo::MembershipRepositoryImpl;
use crate::infra::db::repo::type_repo::TypeRepositoryImpl;

/// Unified service facade implementing both SDK traits.
/// Backed by domain services and repositories.
#[allow(clippy::struct_field_names)]
pub struct RgService {
    type_service: TypeService<TypeRepositoryImpl, GroupRepositoryImpl>,
    group_service: GroupService<
        TypeRepositoryImpl,
        GroupRepositoryImpl,
        ClosureRepositoryImpl,
        MembershipRepositoryImpl,
    >,
    membership_service: MembershipService<GroupRepositoryImpl, MembershipRepositoryImpl>,
}

impl RgService {
    #[must_use]
    pub fn new(
        db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
        max_depth: Option<usize>,
        max_width: Option<usize>,
    ) -> Self {
        let type_service =
            TypeService::new(TypeRepositoryImpl, GroupRepositoryImpl, Arc::clone(&db));
        let membership_service = MembershipService::new(
            GroupRepositoryImpl,
            MembershipRepositoryImpl,
            Arc::clone(&db),
        );
        let group_service = GroupService::new(
            TypeRepositoryImpl,
            GroupRepositoryImpl,
            ClosureRepositoryImpl,
            MembershipRepositoryImpl,
            db,
            max_depth,
            max_width,
        );
        Self {
            type_service,
            group_service,
            membership_service,
        }
    }
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
        request: CreateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        self.group_service
            .create_group(request)
            .await
            .map_err(Into::into)
    }

    async fn get_group(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        self.group_service
            .get_group(group_id)
            .await
            .map_err(Into::into)
    }

    async fn list_groups(
        &self,
        _ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroup>, ResourceGroupError> {
        self.group_service
            .list_groups(query)
            .await
            .map_err(Into::into)
    }

    async fn update_group(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        request: UpdateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        self.group_service
            .update_group(group_id, request)
            .await
            .map_err(Into::into)
    }

    async fn delete_group(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        force: bool,
    ) -> Result<(), ResourceGroupError> {
        self.group_service
            .delete_group(group_id, force)
            .await
            .map_err(Into::into)
    }

    async fn list_group_depth(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        self.group_service
            .list_group_depth(group_id, query)
            .await
            .map_err(Into::into)
    }

    async fn add_membership(
        &self,
        _ctx: &SecurityContext,
        request: AddMembershipRequest,
    ) -> Result<ResourceGroupMembership, ResourceGroupError> {
        self.membership_service
            .add_membership(request)
            .await
            .map_err(Into::into)
    }

    async fn remove_membership(
        &self,
        _ctx: &SecurityContext,
        request: RemoveMembershipRequest,
    ) -> Result<(), ResourceGroupError> {
        self.membership_service
            .remove_membership(request)
            .await
            .map_err(Into::into)
    }

    async fn list_memberships(
        &self,
        _ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        self.membership_service
            .list_memberships(query)
            .await
            .map_err(Into::into)
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
