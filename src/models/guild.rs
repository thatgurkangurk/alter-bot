use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "guilds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub log_channel_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::poll::Entity")]
    Poll,
    #[sea_orm(has_many = "super::voter_ban::Entity")]
    VoterBan,
}

impl Related<super::poll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Poll.def()
    }
}

impl Related<super::voter_ban::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VoterBan.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
