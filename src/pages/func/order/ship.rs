use crate::libs::dser::{op_deser_yyyy_mm_dd_hh_mm_ss, op_deserialize_storehouse};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize, FromRow, Default)]
pub struct Ship {
    pub shipped: i32,
    #[serde(deserialize_with = "op_deser_yyyy_mm_dd_hh_mm_ss")]
    pub date: Option<String>,
    #[serde(deserialize_with = "op_deserialize_storehouse")]
    pub storehouse: Option<String>,
}
