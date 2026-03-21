//! Infrastructure layer mapping from type-safe `FilterNode` to `SeaORM` Conditions.
//!
//! This module maps from DTO-level filter fields to `SeaORM` Column types.

use modkit_db::odata::sea_orm_filter::{FieldToColumn, ODataFieldMapping};

use crate::infra::storage::entity::gts_type::{
    Column as TypeColumn, Entity as TypeEntity, Model as TypeModel,
};
use crate::infra::storage::entity::resource_group::{
    Column as GroupColumn, Entity as GroupEntity, Model as GroupModel,
};
use resource_group_sdk::odata::{GroupFilterField, HierarchyFilterField, TypeFilterField};

/// `OData` mapper for GTS types.
pub struct TypeODataMapper;

impl FieldToColumn<TypeFilterField> for TypeODataMapper {
    type Column = TypeColumn;

    fn map_field(field: TypeFilterField) -> TypeColumn {
        match field {
            TypeFilterField::Code => TypeColumn::SchemaId,
        }
    }
}

impl ODataFieldMapping<TypeFilterField> for TypeODataMapper {
    type Entity = TypeEntity;

    fn extract_cursor_value(model: &TypeModel, field: TypeFilterField) -> sea_orm::Value {
        match field {
            TypeFilterField::Code => {
                sea_orm::Value::String(Some(Box::new(model.schema_id.clone())))
            }
        }
    }
}

/// `OData` mapper for resource groups.
pub struct GroupODataMapper;

impl FieldToColumn<GroupFilterField> for GroupODataMapper {
    type Column = GroupColumn;

    fn map_field(field: GroupFilterField) -> GroupColumn {
        match field {
            GroupFilterField::Type => GroupColumn::GtsTypeId,
            GroupFilterField::HierarchyParentId => GroupColumn::ParentId,
            GroupFilterField::Id => GroupColumn::Id,
            GroupFilterField::Name => GroupColumn::Name,
        }
    }
}

impl ODataFieldMapping<GroupFilterField> for GroupODataMapper {
    type Entity = GroupEntity;

    fn extract_cursor_value(model: &GroupModel, field: GroupFilterField) -> sea_orm::Value {
        match field {
            GroupFilterField::Id => sea_orm::Value::Uuid(Some(Box::new(model.id))),
            GroupFilterField::Name => sea_orm::Value::String(Some(Box::new(model.name.clone()))),
            GroupFilterField::HierarchyParentId => match model.parent_id {
                Some(pid) => sea_orm::Value::Uuid(Some(Box::new(pid))),
                None => sea_orm::Value::Uuid(None),
            },
            GroupFilterField::Type => sea_orm::Value::SmallInt(Some(model.gts_type_id)),
        }
    }
}

/// `OData` mapper for hierarchy queries (not used for `paginate_odata`; hierarchy
/// queries are handled manually). Included for completeness.
pub struct HierarchyODataMapper;

impl FieldToColumn<HierarchyFilterField> for HierarchyODataMapper {
    type Column = GroupColumn;

    fn map_field(field: HierarchyFilterField) -> GroupColumn {
        match field {
            HierarchyFilterField::HierarchyDepth => GroupColumn::Id, // placeholder
            HierarchyFilterField::Type => GroupColumn::GtsTypeId,
        }
    }
}

impl ODataFieldMapping<HierarchyFilterField> for HierarchyODataMapper {
    type Entity = GroupEntity;

    fn extract_cursor_value(model: &GroupModel, field: HierarchyFilterField) -> sea_orm::Value {
        match field {
            HierarchyFilterField::HierarchyDepth => sea_orm::Value::Int(None),
            HierarchyFilterField::Type => sea_orm::Value::SmallInt(Some(model.gts_type_id)),
        }
    }
}
