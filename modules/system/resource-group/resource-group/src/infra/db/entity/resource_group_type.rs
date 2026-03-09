use modkit_db_macros::Scopable;
use sea_orm::entity::prelude::*;
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Scopable)]
#[sea_orm(table_name = "resource_group_type")]
#[secure(unrestricted)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub code: String,
    #[sea_orm(column_type = "Json")]
    pub parents: serde_json::Value,
    pub created: OffsetDateTime,
    pub modified: Option<OffsetDateTime>,
}

impl Model {
    /// Get parents as `Vec<String>`.
    #[must_use]
    pub fn parents_vec(&self) -> Vec<String> {
        serde_json::from_value(self.parents.clone()).unwrap_or_default()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::resource_group::Entity")]
    ResourceGroups,
}

impl Related<super::resource_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ResourceGroups.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
