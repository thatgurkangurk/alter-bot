use sea_orm::{ActiveValue::Set, entity::prelude::*, sea_query::OnConflict};
use twilight_model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "guild_member")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guild_id: i64,

    #[sea_orm(belongs_to, from = "guild_id", to = "id")]
    pub guild: BelongsTo<super::guild::Entity>,

    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn guild_id(&self) -> Id<GuildMarker> {
        Id::new(self.guild_id as u64)
    }

    pub fn user_id(&self) -> Id<UserMarker> {
        Id::new(self.user_id as u64)
    }
}

impl Entity {
    pub async fn get_or_initialise(
        db: &DatabaseConnection,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) -> Result<Model, DbErr> {
        let g_id = guild_id.get() as i64;
        let u_id = user_id.get() as i64;

        let active_model = ActiveModel {
            guild_id: Set(g_id),
            user_id: Set(u_id),
        };

        Entity::insert(active_model)
            .on_conflict(
                OnConflict::columns([Column::GuildId, Column::UserId])
                    .update_column(Column::GuildId) // Dummy update forces Postgres to touch the row
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
