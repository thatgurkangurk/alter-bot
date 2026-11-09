use chrono::Utc;
use sea_orm::{ActiveValue::Set, entity::prelude::*};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "poll")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub message_id: i64,

    pub channel_id: i64,
    pub guild_id: i64,
    pub author_id: i64,

    #[sea_orm(belongs_to, from = "(guild_id, author_id)", to = "(guild_id, user_id)")]
    pub started_by: BelongsTo<super::guild_member::Entity>,

    #[sea_orm(has_many)]
    pub votes: HasMany<super::vote::Entity>,

    pub question: String,
    pub description: Option<String>,
    pub end_time: DateTimeUtc,
    pub is_closed: bool,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn message_id(&self) -> Id<MessageMarker> {
        Id::new(self.message_id as u64)
    }

    pub fn channel_id(&self) -> Id<ChannelMarker> {
        Id::new(self.channel_id as u64)
    }

    pub fn guild_id(&self) -> Id<GuildMarker> {
        Id::new(self.guild_id as u64)
    }

    pub fn author_id(&self) -> Id<UserMarker> {
        Id::new(self.author_id as u64)
    }

    pub fn is_expired(&self) -> bool {
        self.end_time <= Utc::now()
    }
}

pub struct CreatePoll {
    pub message_id: Id<MessageMarker>,
    pub channel_id: Id<ChannelMarker>,
    pub guild_id: Id<GuildMarker>,
    pub author_id: Id<UserMarker>,
    pub question: String,
    pub description: Option<String>,
    pub end_time: DateTimeUtc,
}

impl Entity {
    pub async fn create_poll(db: &DatabaseConnection, params: CreatePoll) -> Result<Model, DbErr> {
        let active_model = ActiveModel {
            message_id: Set(params.message_id.get() as i64),
            channel_id: Set(params.channel_id.get() as i64),
            guild_id: Set(params.guild_id.get() as i64),
            author_id: Set(params.author_id.get() as i64),
            question: Set(params.question),
            description: Set(params.description),
            end_time: Set(params.end_time),
            is_closed: Set(false),
        };

        Entity::insert(active_model).exec_with_returning(db).await
    }
}
