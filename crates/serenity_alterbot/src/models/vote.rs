use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "votes")]
#[allow(clippy::struct_field_names)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub poll_id: Uuid,

    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,

    pub option_id: Uuid,

    #[sea_orm(belongs_to, from = "poll_id", to = "id")]
    pub poll: BelongsTo<super::poll::Entity>,

    #[sea_orm(belongs_to, from = "option_id", to = "id")]
    pub option: BelongsTo<super::poll_option::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
