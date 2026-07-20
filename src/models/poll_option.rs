use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "poll_options")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub poll_id: Uuid,
    pub label: String,
    #[sea_orm(column_type = "Double")]
    pub weight: f64, // default to 1.0. set to 1.5 for the old hard no logic
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::poll::Entity",
        from = "Column::PollId",
        to = "super::poll::Column::Id"
    )]
    Poll,
    #[sea_orm(has_many = "super::vote::Entity")]
    Vote,
}

impl Related<super::poll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Poll.def()
    }
}

impl Related<super::vote::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vote.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
