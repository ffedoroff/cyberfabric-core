---
status: accepted
date: 2026-03-10
---

# Graceful Degradation When AuthZ Plugin Cannot Resolve Group Hierarchy

**ID**: `cpt-cf-resource-group-adr-graceful-degradation-hierarchy`

## Context and Problem Statement

The static-authz-plugin uses `ResourceGroupReadHierarchy` (via ClientHub) to resolve group hierarchy data when a PEP declares `Capability::GroupHierarchy`. This call can fail in two ways: (1) `ResourceGroupReadHierarchy` is not registered in ClientHub (RG module not deployed or not initialized), and (2) the hierarchy call succeeds at resolution but fails at runtime (DB error, timeout, etc.). How should the plugin behave when group hierarchy data is unavailable?

## Decision Drivers

* **Availability** — AuthZ evaluation must not become a single point of failure; a hierarchy lookup failure should not cascade into a total access denial for the entire platform
* **Security** — Degraded mode must not grant broader access than intended; it should reduce scope, not expand it
* **Observability** — Degraded behavior must be visible in logs and metrics for operators to detect and remediate
* **Consistency with PDP/PEP contract** — The PEP declares capabilities; the PDP returns predicates matching those capabilities. If a capability cannot be fulfilled, the system must degrade safely within the constraint model
* **Module independence** — The static-authz-plugin must function even when optional modules (like resource-group) are not deployed

## Considered Options

* **Option A**: Hard fail — return `EvaluationFailed` error, causing 503 at PEP
* **Option B**: Deny access — return `decision: false`
* **Option C**: Graceful degradation — fall back to tenant-scoped constraint (drop group constraint, keep tenant isolation)

## Decision Outcome

Chosen option: "Option C — Graceful degradation with tenant-scoped fallback", because it maintains service availability while preserving tenant isolation as the security baseline. Group-level constraints are an optimization (narrower scope) on top of tenant isolation; losing them reduces precision but does not compromise the fundamental security boundary.

### Consequences

* The plugin returns `allow_with_tenant(tenant_id)` instead of `allow_with_group_subtree(tenant_id, group_id)` when hierarchy lookup fails
* Callers may see a wider result set (all tenant resources instead of group-scoped resources) during degradation — this is a broadening within the tenant boundary, not a cross-tenant leak
* Module init behavior is split:
  - **Runtime hierarchy call failure** → graceful fallback with warning log
  - **ClientHub resolution failure at init** → currently hard-fails module init. This is acceptable because the static-authz-plugin explicitly declares RG as a dependency; if RG is not deployed, the plugin should not claim `GroupHierarchy` support
* Operators must monitor `warn` logs with pattern `"Group hierarchy lookup failed, falling back to tenant-only scope"` to detect persistent degradation
* Future: consider adding a metric counter for degradation events to enable alerting

### Confirmation

* Unit test `group_hierarchy_fallback_on_error` in `static-authz-plugin/src/domain/service.rs` — verifies runtime fallback returns `decision: true` with tenant-only constraint
* Unit test `group_hierarchy_with_no_hierarchy_client_falls_back` — verifies `Service::new()` (no hierarchy client) falls through to TenantHierarchy or default
* Integration test `cross_module_authz_rg` suite — verifies end-to-end hierarchy resolution

## Pros and Cons of the Options

### Option A: Hard fail — return EvaluationFailed

Return `EvaluationFailed` error from the plugin, which PEP maps to `DomainError::ServiceUnavailable` → HTTP 503.

* Good, because it clearly signals the failure to callers
* Good, because it does not change authorization scope
* Bad, because a transient hierarchy lookup failure causes complete request rejection
* Bad, because it creates a hard dependency on RG module availability for all requests that declare `GroupHierarchy`
* Bad, because 503 errors may trigger cascading retries and load amplification

### Option B: Deny access — return decision false

Return `decision: false` from the plugin evaluation.

* Good, because it is the most conservative security stance (fail-closed)
* Bad, because a transient failure denies legitimate access to users who should have tenant-level access
* Bad, because it punishes all callers for an infrastructure issue they cannot resolve
* Bad, because deny with no clear deny_reason is confusing for operators and users

### Option C: Graceful degradation — tenant-scoped fallback

Return `decision: true` with `allow_with_tenant(tenant_id)` — flat `In(OWNER_TENANT_ID, [tid])` constraint instead of `InGroupSubtree`.

* Good, because service remains available during transient hierarchy failures
* Good, because tenant isolation is preserved (security baseline maintained)
* Good, because callers still get correct, if broader, results
* Good, because it matches the natural degradation hierarchy: group scope ⊂ tenant scope
* Neutral, because callers may see more results than expected (wider scope within tenant)
* Bad, because prolonged degradation without operator attention could expose resources that should be group-scoped

## More Information

### Current Implementation

The degradation behavior is implemented in `static-authz-plugin/src/domain/service.rs`:

```
evaluate() flow:
1. GroupHierarchy capability + hierarchy client available + group_id present
   → evaluate_with_hierarchy()
     → On Ok: compound constraint (tenant + group subtree)
     → On Err: warn log + allow_with_tenant(tenant_id)  ← DEGRADATION
2. GroupHierarchy capability + hierarchy client NOT available
   → Falls through to TenantHierarchy or default  ← DEGRADATION
3. TenantHierarchy capability
   → allow_with_tenant_subtree(tenant_id)
4. No capabilities
   → allow_with_tenant(tenant_id)
```

### Degradation Scenarios Matrix

| Scenario | Hierarchy Client | Runtime Call | Result | Security |
|----------|-----------------|-------------|--------|----------|
| Normal operation | Available | Success | GroupSubtree + Tenant | Full |
| Group not in tenant | Available | Success (wrong tenant) | DENY | Full |
| Runtime failure | Available | Error | Tenant-only | Degraded (wider scope) |
| Client not available | None | N/A | TenantHierarchy or flat In | Degraded (wider scope) |
| No tenant resolvable | Either | N/A | DENY | Full |

### Monitoring Recommendations

* Log pattern: `"Group hierarchy lookup failed, falling back to tenant-only scope"` (level: WARN)
* Future: add `authz_plugin_hierarchy_degradation_total` counter metric
* Alert threshold: > 10 degradation events per minute sustained for > 5 minutes

## Traceability

- **PRD**: [PRD.md](../PRD.md)
- **DESIGN**: [DESIGN.md](../DESIGN.md)

This decision directly addresses:

* `cpt-cf-resource-group-feature-authz-constraint-types` — Advanced constraint types depend on hierarchy resolution; this ADR defines behavior when resolution fails
* `cpt-cf-resource-group-principle-policy-agnostic` — RG module does not make policy decisions; degradation is handled by the plugin, not the module
* `docs/arch/authorization/DESIGN.md` — PDP/PEP contract; capability negotiation implies graceful handling when capabilities cannot be fulfilled
