//! Internal: construct an [`EvaluationRequest`] from PEP inputs.
//!
//! Extracted from [`PolicyEnforcer`](crate::pep::PolicyEnforcer) so the
//! request-assembly logic (and its dependency on the `models` request types)
//! lives off the high-traffic enforcer module. Not part of the public API.

use std::collections::HashMap;

use toolkit_security::SecurityContext;
use uuid::Uuid;

use crate::models::{
    Action, Capability, EvaluationRequest, EvaluationRequestContext, Resource, Subject,
};
use crate::pep::access_request::AccessRequest;
use crate::pep::resource_type::ResourceType;

/// Build an evaluation request with per-request overrides from [`AccessRequest`].
pub(crate) fn build_request_with(
    capabilities: &[Capability],
    ctx: &SecurityContext,
    resource: &ResourceType,
    action: &str,
    resource_id: Option<Uuid>,
    require_constraints: bool,
    request: &AccessRequest,
) -> EvaluationRequest {
    // Pass through the caller's tenant context as-is.
    // If no context_tenant_id was set, the PDP determines it by its own rules
    // (e.g. falling back to subject.properties["tenant_id"]).
    let tenant_context = request.tenant_context.clone();

    // Put subject's tenant_id into properties per AuthZEN spec
    let mut subject_properties = HashMap::new();
    subject_properties.insert(
        "tenant_id".to_owned(),
        serde_json::Value::String(ctx.subject_tenant_id().to_string()),
    );

    let bearer_token = ctx.bearer_token().cloned();

    EvaluationRequest {
        subject: Subject {
            id: ctx.subject_id(),
            subject_type: ctx.subject_type().map(ToOwned::to_owned),
            properties: subject_properties,
        },
        action: Action {
            name: action.to_owned(),
        },
        resource: Resource {
            resource_type: resource.name().to_owned(),
            id: resource_id,
            properties: request.resource_properties.clone(),
        },
        context: EvaluationRequestContext {
            tenant_context,
            token_scopes: ctx.token_scopes().to_vec(),
            require_constraints,
            capabilities: capabilities.to_vec(),
            supported_properties: resource
                .supported_properties()
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            bearer_token,
        },
    }
}
