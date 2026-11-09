use sea_orm::{ActiveValue::Set, entity::prelude::*, sea_query::OnConflict};
use twilight_model::id::{
    Id,
    marker::{MessageMarker, UserMarker},
};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "vote")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub poll_message_id: i64,

    #[sea_orm(
        belongs_to,
        from = "poll_message_id",
        to = "message_id",
        on_delete = "Cascade"
    )]
    pub poll: BelongsTo<super::poll::Entity>,

    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,

    pub choice: bool,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn poll_message_id(&self) -> Id<MessageMarker> {
        Id::new(self.poll_message_id as u64)
    }

    pub fn user_id(&self) -> Id<UserMarker> {
        Id::new(self.user_id as u64)
    }
}

impl Entity {
    pub async fn cast_vote(
        db: &DatabaseConnection,
        poll_message_id: Id<MessageMarker>,
        user_id: Id<UserMarker>,
        choice: bool,
    ) -> Result<Model, DbErr> {
        let active_model = ActiveModel {
            poll_message_id: Set(poll_message_id.get() as i64),
            user_id: Set(user_id.get() as i64),
            choice: Set(choice),
        };

        Entity::insert(active_model)
            .on_conflict(
                OnConflict::columns([Column::PollMessageId, Column::UserId])
                    .update_column(Column::Choice)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
