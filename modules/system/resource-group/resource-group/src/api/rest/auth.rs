// @cpt-dod:cpt-cf-resource-group-dod-integration-auth-dual-auth:p1
//! Dual authentication mode routing for the resource-group module.
//!
//! JWT mode: standard `AuthN` -> `PolicyEnforcer` -> `AccessScope` pipeline.
//! MTLS mode: hierarchy-only bypass for `AuthZ` plugin (certificate verification
//! is handled by infrastructure/API gateway).

use std::path::PathBuf;

/// MTLS configuration for trusted system clients.
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    /// Path to the trusted CA certificate bundle.
    pub ca_cert: PathBuf,
    /// Allowed client certificate CNs (e.g., "authz-resolver-plugin").
    pub allowed_clients: Vec<String>,
    /// Allowed method+path pairs for MTLS mode (e.g.,
    /// "GET /api/resource-group/v1/groups/{group_id}/hierarchy").
    pub allowed_endpoints: Vec<AllowedEndpoint>,
}

/// An endpoint allowed for MTLS access.
#[derive(Debug, Clone)]
pub struct AllowedEndpoint {
    pub method: http::Method,
    pub path_pattern: String,
}

/// Authentication mode determined per-request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    /// Standard JWT authentication with full `AuthZ` evaluation.
    Jwt,
    /// MTLS authentication -- trusted system principal, `AuthZ` bypassed.
    Mtls,
}

// @cpt-algo:cpt-cf-resource-group-algo-integration-auth-auth-mode-decision:p1
/// Determine authentication mode from request context.
///
/// If the request has a valid client certificate header (forwarded by API gateway)
/// AND the endpoint is in the MTLS allowlist, returns [`AuthMode::Mtls`].
/// Otherwise returns [`AuthMode::Jwt`].
#[must_use]
pub fn determine_auth_mode(
    client_cn: Option<&str>,
    method: &http::Method,
    path: &str,
    config: &MtlsConfig,
) -> AuthMode {
    // @cpt-flow:cpt-cf-resource-group-flow-integration-auth-mtls-request:p1
    if let Some(cn) = client_cn
        && config.allowed_clients.iter().any(|c| c == cn)
        && is_endpoint_allowed(method, path, &config.allowed_endpoints)
    {
        return AuthMode::Mtls;
    }
    // @cpt-flow:cpt-cf-resource-group-flow-integration-auth-jwt-request:p1
    AuthMode::Jwt
}

/// Check if the given method+path matches any allowed endpoint pattern.
fn is_endpoint_allowed(method: &http::Method, path: &str, endpoints: &[AllowedEndpoint]) -> bool {
    endpoints
        .iter()
        .any(|ep| ep.method == *method && path_matches_pattern(path, &ep.path_pattern))
}

/// Simple path pattern matching supporting `{param}` segments.
fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    let path_segments: Vec<&str> = path.split('/').collect();
    let pattern_segments: Vec<&str> = pattern.split('/').collect();

    if path_segments.len() != pattern_segments.len() {
        return false;
    }

    path_segments
        .iter()
        .zip(pattern_segments.iter())
        .all(|(p, pat)| (pat.starts_with('{') && pat.ends_with('}')) || p == pat)
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            ca_cert: PathBuf::from("/etc/ssl/certs/rg-mtls-ca.pem"),
            allowed_clients: vec!["authz-resolver-plugin".to_owned()],
            allowed_endpoints: vec![AllowedEndpoint {
                method: http::Method::GET,
                path_pattern: "/api/resource-group/v1/groups/{group_id}/hierarchy".to_owned(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> MtlsConfig {
        MtlsConfig::default()
    }

    #[test]
    fn jwt_mode_when_no_client_cn() {
        let config = default_config();
        let mode = determine_auth_mode(
            None,
            &http::Method::GET,
            "/api/resource-group/v1/groups/123/hierarchy",
            &config,
        );
        assert_eq!(mode, AuthMode::Jwt);
    }

    #[test]
    fn mtls_mode_for_allowed_client_and_endpoint() {
        let config = default_config();
        let mode = determine_auth_mode(
            Some("authz-resolver-plugin"),
            &http::Method::GET,
            "/api/resource-group/v1/groups/some-uuid/hierarchy",
            &config,
        );
        assert_eq!(mode, AuthMode::Mtls);
    }

    #[test]
    fn jwt_mode_for_unknown_client_cn() {
        let config = default_config();
        let mode = determine_auth_mode(
            Some("unknown-client"),
            &http::Method::GET,
            "/api/resource-group/v1/groups/some-uuid/hierarchy",
            &config,
        );
        assert_eq!(mode, AuthMode::Jwt);
    }

    #[test]
    fn jwt_mode_for_disallowed_endpoint() {
        let config = default_config();
        let mode = determine_auth_mode(
            Some("authz-resolver-plugin"),
            &http::Method::POST,
            "/api/resource-group/v1/groups",
            &config,
        );
        assert_eq!(mode, AuthMode::Jwt);
    }

    #[test]
    fn path_matching_with_param_segments() {
        assert!(path_matches_pattern(
            "/api/resource-group/v1/groups/abc-123/hierarchy",
            "/api/resource-group/v1/groups/{group_id}/hierarchy"
        ));
    }

    #[test]
    fn path_matching_rejects_different_length() {
        assert!(!path_matches_pattern(
            "/api/resource-group/v1/groups",
            "/api/resource-group/v1/groups/{group_id}/hierarchy"
        ));
    }

    #[test]
    fn path_matching_rejects_wrong_literal_segment() {
        assert!(!path_matches_pattern(
            "/api/resource-group/v1/groups/abc-123/members",
            "/api/resource-group/v1/groups/{group_id}/hierarchy"
        ));
    }

    // --- Phase 3: MTLS edge case tests ---

    #[test]
    fn mtls_mode_with_multiple_allowed_clients() {
        let config = MtlsConfig {
            allowed_clients: vec![
                "authz-resolver-plugin".to_owned(),
                "billing-service".to_owned(),
            ],
            ..MtlsConfig::default()
        };
        let mode = determine_auth_mode(
            Some("billing-service"),
            &http::Method::GET,
            "/api/resource-group/v1/groups/some-uuid/hierarchy",
            &config,
        );
        assert_eq!(mode, AuthMode::Mtls);
    }

    #[test]
    fn mtls_rejected_for_put_to_hierarchy() {
        let config = default_config();
        let mode = determine_auth_mode(
            Some("authz-resolver-plugin"),
            &http::Method::PUT,
            "/api/resource-group/v1/groups/some-uuid/hierarchy",
            &config,
        );
        assert_eq!(
            mode,
            AuthMode::Jwt,
            "PUT to hierarchy should not be MTLS-allowed"
        );
    }

    #[test]
    fn mtls_rejected_for_delete_to_groups() {
        let config = default_config();
        let mode = determine_auth_mode(
            Some("authz-resolver-plugin"),
            &http::Method::DELETE,
            "/api/resource-group/v1/groups/some-uuid",
            &config,
        );
        assert_eq!(
            mode,
            AuthMode::Jwt,
            "DELETE to groups should not be MTLS-allowed"
        );
    }

    #[test]
    fn mtls_with_empty_client_cn() {
        let config = default_config();
        let mode = determine_auth_mode(
            Some(""),
            &http::Method::GET,
            "/api/resource-group/v1/groups/some-uuid/hierarchy",
            &config,
        );
        assert_eq!(mode, AuthMode::Jwt, "Empty CN should fall back to JWT");
    }

    #[test]
    fn mtls_with_multiple_endpoints() {
        let config = MtlsConfig {
            allowed_endpoints: vec![
                AllowedEndpoint {
                    method: http::Method::GET,
                    path_pattern: "/api/resource-group/v1/groups/{group_id}/hierarchy".to_owned(),
                },
                AllowedEndpoint {
                    method: http::Method::GET,
                    path_pattern: "/api/resource-group/v1/types".to_owned(),
                },
            ],
            ..MtlsConfig::default()
        };
        // First endpoint
        assert_eq!(
            determine_auth_mode(
                Some("authz-resolver-plugin"),
                &http::Method::GET,
                "/api/resource-group/v1/groups/uuid/hierarchy",
                &config,
            ),
            AuthMode::Mtls,
        );
        // Second endpoint
        assert_eq!(
            determine_auth_mode(
                Some("authz-resolver-plugin"),
                &http::Method::GET,
                "/api/resource-group/v1/types",
                &config,
            ),
            AuthMode::Mtls,
        );
        // Not in list
        assert_eq!(
            determine_auth_mode(
                Some("authz-resolver-plugin"),
                &http::Method::POST,
                "/api/resource-group/v1/types",
                &config,
            ),
            AuthMode::Jwt,
        );
    }
}
