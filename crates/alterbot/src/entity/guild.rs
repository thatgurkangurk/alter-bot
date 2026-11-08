use sea_orm::{ActiveValue::Set, entity::prelude::*, sea_query::OnConflict};
use twilight_model::id::{Id, marker::GuildMarker};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "guild")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,

    pub welcome_channel_id: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}

// impl Model {
//     pub fn discord_id(&self) -> Id<GuildMarker> {
//         Id::new(self.id as u64)
//     }
// }

impl Entity {
    pub async fn get_or_initialise(
        db: &DatabaseConnection,
        guild_id: Id<GuildMarker>,
    ) -> Result<Model, DbErr> {
        let id_i64 = guild_id.get() as i64;

        let active_model = ActiveModel {
            id: Set(id_i64),
            ..Default::default()
        };

        Entity::insert(active_model)
            .on_conflict(
                OnConflict::column(Column::Id)
                    .update_column(Column::Id) // dummy update to trigger RETURNING in postgres
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
