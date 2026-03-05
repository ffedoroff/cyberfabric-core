use modkit_db_macros::Scopable;
use sea_orm::entity::prelude::*;
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Scopable)]
#[sea_orm(table_name = "resource_group")]
#[secure(tenant_col = "tenant_id", resource_col = "id", no_owner, no_type)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub group_type: String,
    pub name: String,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
    pub created: OffsetDateTime,
    pub modified: Option<OffsetDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::resource_group_type::Entity",
        from = "Column::GroupType",
        to = "super::resource_group_type::Column::Code"
    )]
    GroupType,
    #[sea_orm(belongs_to = "Entity", from = "Column::ParentId", to = "Column::Id")]
    Parent,
}

impl Related<super::resource_group_type::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GroupType.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
