CREATE TABLE resource_group_type (
    code TEXT PRIMARY KEY,
    parents TEXT[] NOT NULL DEFAULT '{}',
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified TIMESTAMP WITH TIME ZONE DEFAULT NULL
);

CREATE UNIQUE INDEX idx_resource_group_type_code_lower
    ON resource_group_type (LOWER(code));

COMMENT ON TABLE resource_group_type
    IS 'Resource group type definitions with parent type relationships';

CREATE TABLE resource_group (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID,
    group_type TEXT NOT NULL,
    name TEXT NOT NULL,
    tenant_id UUID NOT NULL,
    external_id TEXT,
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    CONSTRAINT fk_resource_group_type
        FOREIGN KEY (group_type)
        REFERENCES resource_group_type(code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_resource_group_parent
        FOREIGN KEY (parent_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

-- ── resource_group indexes ─────────────────────────────────────────────────

-- parent_id: queries 4h (eq), 4j (in), 4aa (group_type + parent_id)
CREATE INDEX idx_rg_parent_id
    ON resource_group (parent_id);

-- name: queries 4k (eq), 4m (in)
CREATE INDEX idx_rg_name
    ON resource_group (name);

-- external_id: queries 4q (eq), 4s (in), 4ac (group_type + external_id)
CREATE INDEX idx_rg_external_id
    ON resource_group (external_id);

-- group_type + id: queries 4e (eq), 4g (in), 4aa, 4ab, 4ac, 4ae, 4af
-- Composite allows seek by group_type and ordered scan by id (avoids PK scan + filter)
CREATE INDEX idx_rg_group_type
    ON resource_group (group_type, id);

COMMENT ON TABLE resource_group
    IS 'Hierarchical resource groups with closure table pattern for efficient ancestor/descendant queries';
COMMENT ON COLUMN resource_group.parent_id
    IS 'Direct parent group reference; NULL for root groups (e.g. top-level tenants)';
COMMENT ON COLUMN resource_group.group_type
    IS 'Reference to resource_group_type.code defining the type of this resource group';
COMMENT ON COLUMN resource_group.external_id
    IS 'Optional external identifier for integration with other systems';

CREATE TABLE resource_group_closure (
    ancestor_id UUID NOT NULL,
    descendant_id UUID NOT NULL,
    depth INTEGER NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id),
    CONSTRAINT fk_closure_ancestor
        FOREIGN KEY (ancestor_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_closure_descendant
        FOREIGN KEY (descendant_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

COMMENT ON TABLE resource_group_closure
    IS 'Closure table for resource group hierarchy - stores all ancestor-descendant relationships with depth';
COMMENT ON COLUMN resource_group_closure.depth
    IS 'Distance between ancestor and descendant: 0 = self-reference, 1 = direct descendant, 2+ = deeper descendants';

-- Closure indexes: JOIN on descendant_id and filter by ancestor+depth
CREATE INDEX idx_rgc_descendant_id
    ON resource_group_closure (descendant_id);

CREATE INDEX idx_rgc_ancestor_depth
    ON resource_group_closure (ancestor_id, depth);

CREATE TABLE resource_group_membership (
    group_id UUID NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_resource_group_membership_group_id
        FOREIGN KEY (group_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_membership_unique
        UNIQUE (group_id, resource_type, resource_id)
);

-- ── resource_group_membership indexes ──────────────────────────────────────

-- resource_type + resource_id (without group_id): queries 5q, and 5h/5j (rare, via full index scan)
CREATE INDEX idx_rgm_resource_type_id
    ON resource_group_membership (resource_type, resource_id);
