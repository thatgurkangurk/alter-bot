use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "polls")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: Option<i64>,
    pub title: String,
    pub ends_at: DateTimeWithTimeZone,
    pub is_active: bool,
    pub required_role_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::guild::Entity",
        from = "Column::GuildId",
        to = "super::guild::Column::Id"
    )]
    Guild,
    #[sea_orm(has_many = "super::vote::Entity")]
    Vote,
    #[sea_orm(has_many = "super::poll_option::Entity")]
    PollOption,
}

impl Related<super::guild::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Guild.def()
    }
}

impl Related<super::vote::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vote.def()
    }
}

impl Related<super::poll_option::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PollOption.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
