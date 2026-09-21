use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[sea_orm::model]
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

    #[sea_orm(belongs_to, from = "guild_id", to = "id")]
    pub guild: BelongsTo<super::guild::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
