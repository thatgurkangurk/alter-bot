use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "voter_bans")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guild_id: i64,

    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,

    #[sea_orm(belongs_to, from = "guild_id", to = "id")]
    pub guild: BelongsTo<super::guild::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
