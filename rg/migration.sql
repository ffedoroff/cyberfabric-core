CREATE TABLE IF NOT EXISTS resource_group_type (
    code TEXT PRIMARY KEY,
    ancestors TEXT[] NOT NULL DEFAULT '{}'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_resource_group_type_code_lower
    ON resource_group_type (LOWER(code));

COMMENT ON TABLE resource_group_type
    IS 'Resource group type definitions with ancestor relationships';

CREATE TABLE IF NOT EXISTS resource_group (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
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
        ON DELETE RESTRICT
);

COMMENT ON TABLE resource_group
    IS 'Hierarchical resource groups with closure table pattern for efficient ancestor/descendant queries';
COMMENT ON COLUMN resource_group.group_type
    IS 'Reference to resource_group_type.code defining the type of this resource group';
COMMENT ON COLUMN resource_group.external_id
    IS 'Optional external identifier for integration with other systems';

CREATE TABLE IF NOT EXISTS resource_group_closure (
    ancestor_id UUID NOT NULL,
    descendant_id UUID NOT NULL,
    depth INTEGER NOT NULL,
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

CREATE TABLE IF NOT EXISTS resource_group_membership (
    group_id UUID NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    tenant_id UUID NOT NULL,
    CONSTRAINT fk_resource_group_membership_group_id
        FOREIGN KEY (group_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_membership_unique
        UNIQUE (group_id, resource_type, resource_id)
);
