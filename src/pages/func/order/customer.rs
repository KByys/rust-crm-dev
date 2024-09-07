use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Customer {
    pub id: String,
    pub address: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub company: String,
    pub purchase_unit: String,
}


