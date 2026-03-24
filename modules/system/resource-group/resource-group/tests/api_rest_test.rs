#![allow(clippy::expect_used, clippy::unwrap_used)]
//! API-level tests using `Router::oneshot` pattern.
//!
//! Verifies HTTP-level behavior: status codes, response shapes,
//! `OData` query parsing, and RFC 9457 error format.

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use authz_resolver_sdk::{
    AuthZResolverClient, AuthZResolverError, EvaluationRequest, EvaluationResponse,
    EvaluationResponseContext, PolicyEnforcer,
    constraints::{Constraint, InPredicate, Predicate},
};
use modkit::api::OpenApiRegistry;
use modkit::api::operation_builder::OperationSpec;
use modkit_db::{
    ConnectOpts, DBProvider, DbError, connect_db, migration_runner::run_migrations_for_testing,
};
use modkit_security::{SecurityContext, pep_properties};
use sea_orm_migration::MigratorTrait;

use cf_resource_group::domain::group_service::{GroupService, QueryProfile};
use cf_resource_group::domain::membership_service::MembershipService;
use cf_resource_group::domain::type_service::TypeService;
use cf_resource_group::infra::storage::migrations::Migrator;

// ── Noop OpenAPI Registry for tests ─────────────────────────────────────

struct NoopOpenApiRegistry;

impl OpenApiRegistry for NoopOpenApiRegistry {
    fn register_operation(&self, _spec: &OperationSpec) {}

    fn ensure_schema_raw(
        &self,
        name: &str,
        _schemas: Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) -> String {
        name.to_owned()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── Mock AuthZ: allow-all with tenant scoping ───────────────────────────

struct AllowAllAuthZ;

#[async_trait]
impl AuthZResolverClient for AllowAllAuthZ {
    async fn evaluate(
        &self,
        request: EvaluationRequest,
    ) -> Result<EvaluationResponse, AuthZResolverError> {
        let tenant_id = request
            .subject
            .properties
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or(Uuid::nil());

        Ok(EvaluationResponse {
            decision: true,
            context: EvaluationResponseContext {
                constraints: vec![Constraint {
                    predicates: vec![Predicate::In(InPredicate::new(
                        pep_properties::OWNER_TENANT_ID,
                        [tenant_id],
                    ))],
                }],
                deny_reason: None,
            },
        })
    }
}

// ── Test setup ──────────────────────────────────────────────────────────

async fn test_db() -> Arc<DBProvider<DbError>> {
    let opts = ConnectOpts {
        max_conns: Some(1),
        min_conns: Some(1),
        ..Default::default()
    };
    let db = connect_db("sqlite::memory:", opts)
        .await
        .expect("connect to in-memory SQLite");

    run_migrations_for_testing(&db, Migrator::migrations())
        .await
        .expect("run migrations");

    Arc::new(DBProvider::new(db))
}

fn make_ctx(tenant_id: Uuid) -> SecurityContext {
    SecurityContext::builder()
        .subject_id(Uuid::now_v7())
        .subject_tenant_id(tenant_id)
        .build()
        .expect("valid SecurityContext")
}

fn make_enforcer() -> PolicyEnforcer {
    let authz: Arc<dyn AuthZResolverClient> = Arc::new(AllowAllAuthZ);
    PolicyEnforcer::new(authz)
}

async fn build_test_router() -> (Router, Arc<TypeService>) {
    let db = test_db().await;
    let enforcer = make_enforcer();

    let type_svc = Arc::new(TypeService::new(db.clone()));
    let group_svc = Arc::new(GroupService::new(
        db.clone(),
        QueryProfile::default(),
        enforcer.clone(),
    ));
    let membership_svc = Arc::new(MembershipService::new(db, enforcer));

    let openapi = NoopOpenApiRegistry;
    let router = cf_resource_group::api::rest::routes::register_routes(
        Router::new(),
        &openapi,
        type_svc.clone(),
        group_svc,
        membership_svc,
    );

    (router, type_svc)
}

fn json_request(
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
    tenant_id: Uuid,
) -> Request<Body> {
    let ctx = make_ctx(tenant_id);
    let mut builder = Request::builder().method(method).uri(uri);

    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }

    let body = match body {
        Some(json) => Body::from(serde_json::to_vec(&json).unwrap()),
        None => Body::empty(),
    };

    let mut req = builder.body(body).unwrap();
    req.extensions_mut().insert(ctx);
    req
}

async fn response_body(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or_default()
}

// ── Type CRUD Tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn create_type_returns_201() {
    let (router, _) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let code = format!(
        "gts.x.system.rg.type.v1~test.api.{}.v1~",
        Uuid::now_v7().as_simple()
    );

    let req = json_request(
        "POST",
        "/types-registry/v1/types",
        Some(serde_json::json!({
            "code": code,
            "can_be_root": true,
            "allowed_parents": [],
            "allowed_memberships": []
        })),
        tenant_id,
    );

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = response_body(resp).await;
    assert_eq!(body["code"], code);
    assert_eq!(body["can_be_root"], true);
}

#[tokio::test]
async fn create_type_duplicate_returns_409() {
    let (router, type_svc) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let code = format!(
        "gts.x.system.rg.type.v1~test.dup.{}.v1~",
        Uuid::now_v7().as_simple()
    );

    // Pre-create via service
    type_svc
        .create_type(resource_group_sdk::CreateTypeRequest {
            code: code.clone(),
            can_be_root: true,
            allowed_parents: vec![],
            allowed_memberships: vec![],
            metadata_schema: None,
        })
        .await
        .unwrap();

    let req = json_request(
        "POST",
        "/types-registry/v1/types",
        Some(serde_json::json!({
            "code": code,
            "can_be_root": true,
            "allowed_parents": [],
            "allowed_memberships": []
        })),
        tenant_id,
    );

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_type_invalid_code_returns_400() {
    let (router, _) = build_test_router().await;
    let tenant_id = Uuid::now_v7();

    let req = json_request(
        "POST",
        "/types-registry/v1/types",
        Some(serde_json::json!({
            "code": "wrong.prefix",
            "can_be_root": true,
            "allowed_parents": [],
            "allowed_memberships": []
        })),
        tenant_id,
    );

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_types_returns_200_with_page() {
    let (router, type_svc) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let code = format!(
        "gts.x.system.rg.type.v1~test.list.{}.v1~",
        Uuid::now_v7().as_simple()
    );

    type_svc
        .create_type(resource_group_sdk::CreateTypeRequest {
            code: code.clone(),
            can_be_root: true,
            allowed_parents: vec![],
            allowed_memberships: vec![],
            metadata_schema: None,
        })
        .await
        .unwrap();

    let req = json_request("GET", "/types-registry/v1/types", None, tenant_id);
    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = response_body(resp).await;
    assert_eq!(status, StatusCode::OK, "list_types failed: {body}");

    assert!(body["items"].is_array());
    assert!(!body["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn get_type_returns_200() {
    let (router, type_svc) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let code = format!(
        "gts.x.system.rg.type.v1~test.get.{}.v1~",
        Uuid::now_v7().as_simple()
    );

    type_svc
        .create_type(resource_group_sdk::CreateTypeRequest {
            code: code.clone(),
            can_be_root: true,
            allowed_parents: vec![],
            allowed_memberships: vec![],
            metadata_schema: None,
        })
        .await
        .unwrap();

    let encoded = code.replace('~', "%7E");
    let req = json_request(
        "GET",
        &format!("/types-registry/v1/types/{encoded}"),
        None,
        tenant_id,
    );
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = response_body(resp).await;
    assert_eq!(body["code"], code);
}

#[tokio::test]
async fn get_type_not_found_returns_404() {
    let (router, _) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let code = "gts.x.system.rg.type.v1~nonexistent.v1~";
    let encoded = code.replace('~', "%7E");

    let req = json_request(
        "GET",
        &format!("/types-registry/v1/types/{encoded}"),
        None,
        tenant_id,
    );
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_type_returns_204() {
    let (router, type_svc) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let code = format!(
        "gts.x.system.rg.type.v1~test.del.{}.v1~",
        Uuid::now_v7().as_simple()
    );

    type_svc
        .create_type(resource_group_sdk::CreateTypeRequest {
            code: code.clone(),
            can_be_root: true,
            allowed_parents: vec![],
            allowed_memberships: vec![],
            metadata_schema: None,
        })
        .await
        .unwrap();

    let encoded = code.replace('~', "%7E");
    let req = json_request(
        "DELETE",
        &format!("/types-registry/v1/types/{encoded}"),
        None,
        tenant_id,
    );
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ── Group CRUD Tests ────────────────────────────────────────────────────

#[tokio::test]
async fn create_group_returns_201() {
    let (router, type_svc) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let type_code = format!(
        "gts.x.system.rg.type.v1~test.grp.{}.v1~",
        Uuid::now_v7().as_simple()
    );

    type_svc
        .create_type(resource_group_sdk::CreateTypeRequest {
            code: type_code.clone(),
            can_be_root: true,
            allowed_parents: vec![],
            allowed_memberships: vec![],
            metadata_schema: None,
        })
        .await
        .unwrap();

    let req = json_request(
        "POST",
        "/resource-group/v1/groups",
        Some(serde_json::json!({
            "type": type_code,
            "name": "Test Group"
        })),
        tenant_id,
    );

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = response_body(resp).await;
    assert_eq!(body["name"], "Test Group");
    assert!(body["id"].is_string());
    assert_eq!(body["hierarchy"]["tenant_id"], tenant_id.to_string());
}

#[tokio::test]
async fn list_groups_returns_200() {
    let (router, _) = build_test_router().await;
    let tenant_id = Uuid::now_v7();

    let req = json_request("GET", "/resource-group/v1/groups", None, tenant_id);
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = response_body(resp).await;
    assert!(body["items"].is_array());
}

#[tokio::test]
async fn get_group_not_found_returns_404() {
    let (router, _) = build_test_router().await;
    let tenant_id = Uuid::now_v7();
    let fake_id = Uuid::now_v7();

    let req = json_request(
        "GET",
        &format!("/resource-group/v1/groups/{fake_id}"),
        None,
        tenant_id,
    );
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Error format tests (RFC 9457 Problem Details) ───────────────────────

#[tokio::test]
async fn error_response_has_problem_fields() {
    let (router, _) = build_test_router().await;
    let tenant_id = Uuid::now_v7();

    // Trigger a validation error
    let req = json_request(
        "POST",
        "/types-registry/v1/types",
        Some(serde_json::json!({
            "code": "invalid",
            "can_be_root": true
        })),
        tenant_id,
    );

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = response_body(resp).await;
    // RFC 9457 requires these fields
    assert!(
        body["title"].is_string(),
        "Problem must have 'title': {body}"
    );
    assert!(
        body["status"].is_number(),
        "Problem must have 'status': {body}"
    );
    assert!(
        body["detail"].is_string(),
        "Problem must have 'detail': {body}"
    );
}
