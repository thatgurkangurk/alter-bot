#![allow(clippy::future_not_send)]
#![allow(clippy::derive_partial_eq_without_eq)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "guilds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub log_channel_id: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}
