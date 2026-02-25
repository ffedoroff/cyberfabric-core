CREATE TABLE IF NOT EXISTS resource_group_type (
    code TEXT PRIMARY KEY,
    parents TEXT[] NOT NULL DEFAULT '{}',
    application_id UUID NOT NULL,
    allowed_app_ids UUID[] NOT NULL DEFAULT '{}'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_resource_group_type_code_lower
    ON resource_group_type (LOWER(code));

COMMENT ON TABLE resource_group_type
    IS 'Resource group type definitions with parent relationships and application permissions';

CREATE TABLE IF NOT EXISTS resource_group (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_code TEXT NOT NULL,
    name TEXT NOT NULL,
    external_id TEXT,
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    CONSTRAINT fk_resource_group_type
        FOREIGN KEY (type_code)
        REFERENCES resource_group_type(code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_resource_group_type_code ON resource_group(type_code);
CREATE INDEX IF NOT EXISTS idx_resource_group_created ON resource_group(created);
CREATE INDEX IF NOT EXISTS idx_resource_group_external_id
    ON resource_group(external_id)
    WHERE external_id IS NOT NULL;

COMMENT ON TABLE resource_group
    IS 'Hierarchical resource groups with closure table pattern for efficient ancestor/descendant queries';
COMMENT ON COLUMN resource_group.type_code
    IS 'Reference to resource_group_type.code defining the type of this resource group';
COMMENT ON COLUMN resource_group.external_id
    IS 'Optional external identifier for integration with other systems';

CREATE TABLE IF NOT EXISTS resource_group_closure (
    parent_id UUID NOT NULL,
    child_id UUID NOT NULL,
    depth INTEGER NOT NULL,
    CONSTRAINT fk_closure_parent
        FOREIGN KEY (parent_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_closure_child
        FOREIGN KEY (child_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_closure_parent_child
        UNIQUE (parent_id, child_id)
);

CREATE INDEX IF NOT EXISTS idx_closure_parent_id ON resource_group_closure(parent_id);
CREATE INDEX IF NOT EXISTS idx_closure_child_id ON resource_group_closure(child_id);
CREATE INDEX IF NOT EXISTS idx_closure_depth ON resource_group_closure(depth);

COMMENT ON TABLE resource_group_closure
    IS 'Closure table for resource group hierarchy - stores all ancestor-descendant relationships with depth';
COMMENT ON COLUMN resource_group_closure.depth
    IS 'Distance between parent and child: 0 = self-reference, 1 = direct child, 2+ = deeper descendants';

CREATE TABLE IF NOT EXISTS resource_group_reference (
    group_id UUID NOT NULL,
    reference_type TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    application_id UUID NOT NULL,
    CONSTRAINT fk_resource_group_reference_group_id
        FOREIGN KEY (group_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_reference_unique
        UNIQUE (group_id, reference_type, reference_id, application_id)
);

CREATE INDEX IF NOT EXISTS idx_resource_group_reference_group_id
    ON resource_group_reference(group_id);
