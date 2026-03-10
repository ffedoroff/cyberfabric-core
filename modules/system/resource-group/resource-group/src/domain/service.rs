use std::sync::Arc;

use async_trait::async_trait;
use authz_resolver_sdk::pep::ResourceType;
use authz_resolver_sdk::{AccessRequest, PolicyEnforcer};
use modkit_security::SecurityContext;
use resource_group_sdk::{
    AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery, Page,
    RemoveMembershipRequest, ResourceGroup, ResourceGroupClient, ResourceGroupError,
    ResourceGroupMembership, ResourceGroupReadHierarchy, ResourceGroupType, ResourceGroupWithDepth,
    UpdateGroupRequest, UpdateTypeRequest,
};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::group_service::GroupService;
use crate::domain::membership_service::MembershipService;
use crate::domain::type_service::TypeService;
use crate::infra::db::repo::closure_repo::ClosureRepositoryImpl;
use crate::infra::db::repo::group_repo::GroupRepositoryImpl;
use crate::infra::db::repo::membership_repo::MembershipRepositoryImpl;
use crate::infra::db::repo::type_repo::TypeRepositoryImpl;

// ── Resource type descriptors for PEP ────────────────────────────────────

/// Authorization resource types for the resource-group module.
pub(crate) mod resources {
    use super::ResourceType;
    use modkit_security::pep_properties;

    /// Resource groups — tenant-scoped via `tenant_id` column.
    pub const RESOURCE_GROUP: ResourceType = ResourceType {
        name: "gts.cf.core.resource_group.group.v1",
        supported_properties: &[
            pep_properties::OWNER_TENANT_ID,
            pep_properties::RESOURCE_ID,
        ],
    };

    /// Resource group types — global/instance-level, no tenant scoping.
    pub const RESOURCE_GROUP_TYPE: ResourceType = ResourceType {
        name: "gts.cf.core.resource_group.type.v1",
        supported_properties: &[pep_properties::RESOURCE_ID],
    };
}

/// Authorization action identifiers.
pub(crate) mod actions {
    pub const CREATE: &str = "create";
    pub const READ: &str = "read";
    pub const LIST: &str = "list";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    pub const READ_HIERARCHY: &str = "read_hierarchy";
}

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
    enforcer: PolicyEnforcer,
}

impl RgService {
    #[must_use]
    pub fn new(
        db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
        max_depth: Option<usize>,
        max_width: Option<usize>,
        enforcer: PolicyEnforcer,
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
            enforcer,
        }
    }

    #[cfg(test)]
    pub(crate) fn type_service(
        &self,
    ) -> &TypeService<TypeRepositoryImpl, GroupRepositoryImpl> {
        &self.type_service
    }

    #[cfg(test)]
    pub(crate) fn group_service(
        &self,
    ) -> &GroupService<
        TypeRepositoryImpl,
        GroupRepositoryImpl,
        ClosureRepositoryImpl,
        MembershipRepositoryImpl,
    > {
        &self.group_service
    }

    #[cfg(test)]
    pub(crate) fn membership_service(
        &self,
    ) -> &MembershipService<GroupRepositoryImpl, MembershipRepositoryImpl> {
        &self.membership_service
    }
}


#[async_trait]
impl ResourceGroupClient for RgService {
    async fn create_type(
        &self,
        ctx: &SecurityContext,
        request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        let _scope = self
            .enforcer
            .access_scope_with(
                ctx,
                &resources::RESOURCE_GROUP_TYPE,
                actions::CREATE,
                None,
                &AccessRequest::new().require_constraints(false),
            )
            .await
            .map_err(DomainError::from)?;
        self.type_service
            .create_type(request)
            .await
            .map_err(Into::into)
    }

    async fn get_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        let _scope = self
            .enforcer
            .access_scope_with(
                ctx,
                &resources::RESOURCE_GROUP_TYPE,
                actions::READ,
                None,
                &AccessRequest::new().require_constraints(false),
            )
            .await
            .map_err(DomainError::from)?;
        self.type_service.get_type(code).await.map_err(Into::into)
    }

    async fn list_types(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupType>, ResourceGroupError> {
        let _scope = self
            .enforcer
            .access_scope_with(
                ctx,
                &resources::RESOURCE_GROUP_TYPE,
                actions::LIST,
                None,
                &AccessRequest::new().require_constraints(false),
            )
            .await
            .map_err(DomainError::from)?;
        self.type_service
            .list_types(query)
            .await
            .map_err(Into::into)
    }

    async fn update_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
        request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        let _scope = self
            .enforcer
            .access_scope_with(
                ctx,
                &resources::RESOURCE_GROUP_TYPE,
                actions::UPDATE,
                None,
                &AccessRequest::new().require_constraints(false),
            )
            .await
            .map_err(DomainError::from)?;
        self.type_service
            .update_type(code, request)
            .await
            .map_err(Into::into)
    }

    async fn delete_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
    ) -> Result<(), ResourceGroupError> {
        let _scope = self
            .enforcer
            .access_scope_with(
                ctx,
                &resources::RESOURCE_GROUP_TYPE,
                actions::DELETE,
                None,
                &AccessRequest::new().require_constraints(false),
            )
            .await
            .map_err(DomainError::from)?;
        self.type_service
            .delete_type(code)
            .await
            .map_err(Into::into)
    }

    async fn create_group(
        &self,
        ctx: &SecurityContext,
        request: CreateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(ctx, &resources::RESOURCE_GROUP, actions::CREATE, None)
            .await
            .map_err(DomainError::from)?;
        self.group_service
            .create_group(request, &scope)
            .await
            .map_err(Into::into)
    }

    async fn get_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(
                ctx,
                &resources::RESOURCE_GROUP,
                actions::READ,
                Some(group_id),
            )
            .await
            .map_err(DomainError::from)?;
        self.group_service
            .get_group(group_id, &scope)
            .await
            .map_err(Into::into)
    }

    async fn list_groups(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroup>, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(ctx, &resources::RESOURCE_GROUP, actions::LIST, None)
            .await
            .map_err(DomainError::from)?;
        self.group_service
            .list_groups(query, &scope)
            .await
            .map_err(Into::into)
    }

    async fn update_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        request: UpdateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(
                ctx,
                &resources::RESOURCE_GROUP,
                actions::UPDATE,
                Some(group_id),
            )
            .await
            .map_err(DomainError::from)?;
        self.group_service
            .update_group(group_id, request, &scope)
            .await
            .map_err(Into::into)
    }

    async fn delete_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        force: bool,
    ) -> Result<(), ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(
                ctx,
                &resources::RESOURCE_GROUP,
                actions::DELETE,
                Some(group_id),
            )
            .await
            .map_err(DomainError::from)?;
        self.group_service
            .delete_group(group_id, force, &scope)
            .await
            .map_err(Into::into)
    }

    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(
                ctx,
                &resources::RESOURCE_GROUP,
                actions::READ_HIERARCHY,
                Some(group_id),
            )
            .await
            .map_err(DomainError::from)?;
        self.group_service
            .list_group_depth(group_id, query, &scope)
            .await
            .map_err(Into::into)
    }

    async fn add_membership(
        &self,
        ctx: &SecurityContext,
        request: AddMembershipRequest,
    ) -> Result<ResourceGroupMembership, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(
                ctx,
                &resources::RESOURCE_GROUP,
                actions::UPDATE,
                Some(request.group_id),
            )
            .await
            .map_err(DomainError::from)?;
        self.membership_service
            .add_membership(request, &scope)
            .await
            .map_err(Into::into)
    }

    async fn remove_membership(
        &self,
        ctx: &SecurityContext,
        request: RemoveMembershipRequest,
    ) -> Result<(), ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(
                ctx,
                &resources::RESOURCE_GROUP,
                actions::UPDATE,
                Some(request.group_id),
            )
            .await
            .map_err(DomainError::from)?;
        self.membership_service
            .remove_membership(request, &scope)
            .await
            .map_err(Into::into)
    }

    async fn list_memberships(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        let scope = self
            .enforcer
            .access_scope(ctx, &resources::RESOURCE_GROUP, actions::LIST, None)
            .await
            .map_err(DomainError::from)?;
        self.membership_service
            .list_memberships(query, &scope)
            .await
            .map_err(Into::into)
    }
}

/// `ResourceGroupReadHierarchy` implementation bypasses `PolicyEnforcer` intentionally.
///
/// This trait is the system-level read contract consumed by AuthZ plugins.
/// Using the enforced path (`ResourceGroupClient`) would create an infinite loop:
/// Plugin.evaluate → RG.list_group_depth → PolicyEnforcer → AuthZ → Plugin.evaluate → …
///
/// The hierarchy read uses `AccessScope::allow_all()` — the AuthZ plugin is a trusted
/// system-level consumer that determines access for others.
#[async_trait]
impl ResourceGroupReadHierarchy for RgService {
    async fn list_group_depth(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        let system_scope = modkit_security::AccessScope::allow_all();
        self.group_service
            .list_group_depth(group_id, query, &system_scope)
            .await
            .map_err(Into::into)
    }
}
