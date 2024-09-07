mod storehouse;
use axum::Router;
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, FromRow, Clone)]
pub struct Storehouse {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(skip_deserializing)]
    pub create_time: String,
    pub description: String
}
impl PartialEq for Storehouse {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
pub fn store_router() -> Router {
    storehouse::storehouse_router()
}
