// @cpt-algo:cpt-cf-resource-group-algo-cdc-event-schema:p2

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Queue name for tenant hierarchy change events.
pub const TENANT_HIERARCHY_QUEUE: &str = "tenant_hierarchy_changes";

/// CDC event emitted by tenant-resolver when tenant hierarchy mutates.
///
/// The resource-group module consumes these events to maintain the
/// `tenant_closure` local projection table.
// @cpt-begin:cpt-cf-resource-group-algo-cdc-event-schema:p2:inst-delta-1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantHierarchyChanged {
    pub event_type: String,
    pub version: u32,
    pub tenant_id: Uuid,
    pub change_type: ChangeType,
    /// Parent tenant ID (for `Created` and `Moved` events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ancestor_id: Option<Uuid>,
    /// Previous parent (for `Moved` events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_ancestor_id: Option<Uuid>,
    /// Current tenant status.
    pub status: String,
    /// Barrier flag (0 = not a barrier, 1 = self-managed barrier).
    #[serde(default)]
    pub barrier: i32,
}
// @cpt-end:cpt-cf-resource-group-algo-cdc-event-schema:p2:inst-delta-1

/// Type of hierarchy mutation that triggered the CDC event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Moved,
    Deleted,
    StatusChanged,
    BarrierChanged,
}

impl TenantHierarchyChanged {
    pub fn new(tenant_id: Uuid, change_type: ChangeType, status: &str) -> Self {
        Self {
            event_type: "TenantHierarchyChanged".into(),
            version: 1,
            tenant_id,
            change_type,
            ancestor_id: None,
            old_ancestor_id: None,
            status: status.into(),
            barrier: 0,
        }
    }

    pub fn with_ancestor(mut self, ancestor_id: Uuid) -> Self {
        self.ancestor_id = Some(ancestor_id);
        self
    }

    pub fn with_old_ancestor(mut self, old_ancestor_id: Uuid) -> Self {
        self.old_ancestor_id = Some(old_ancestor_id);
        self
    }

    pub fn with_barrier(mut self, barrier: i32) -> Self {
        self.barrier = barrier;
        self
    }
}
