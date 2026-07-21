use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "poll_options")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    pub poll_id: Uuid,

    pub label: String,

    #[sea_orm(column_type = "Double")]
    pub weight: f64, // default to 1.0. set to 1.5 for the old hard no logic

    #[sea_orm(belongs_to, from = "poll_id", to = "id")]
    pub poll: BelongsTo<super::poll::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
