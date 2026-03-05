use modkit_db_macros::Scopable;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Scopable)]
#[sea_orm(table_name = "resource_group_closure")]
#[secure(unrestricted)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub ancestor_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub descendant_id: Uuid,
    pub depth: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::resource_group::Entity",
        from = "Column::AncestorId",
        to = "super::resource_group::Column::Id"
    )]
    Ancestor,
    #[sea_orm(
        belongs_to = "super::resource_group::Entity",
        from = "Column::DescendantId",
        to = "super::resource_group::Column::Id"
    )]
    Descendant,
}

impl ActiveModelBehavior for ActiveModel {}
