use std::collections::HashMap;

use dashmap::DashMap;
use mysql::{prelude::FromValue, Row, Value};
use serde::{Deserialize, Serialize};

pub struct PermissionGroup {}

pub type PE = DashMap<String, DashMap<String, Vec<String>>>;

#[derive(Deserialize, Serialize, Default)]
pub struct Perm {
    name: String,
    value: String,
    parent: Vec<String>,
    children: Option<Box<Perm>>,
    data: Vec<String>,
}

impl Perm {
    pub fn from_row(row: Row) -> Self {
        let columns = row.columns();
        let values = row.unwrap();
        let map: HashMap<String, Value> = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| (columns[i].name_str().to_string(), v))
            .collect();
        let parent = String::from_value(map["parent"].clone())
            .split('-')
            .map(|s| s.to_owned())
            .collect();
        Self {
            name: String::from_value(map["name"].clone()),
            value: String::from_value(map["value"].clone()),
            parent,
            children: None,
            data: String::from_value(map["data"].clone())
                .split('-')
                .map(|s| s.to_owned())
                .collect(),
        }
    }
    pub fn has_children(&self) -> bool {
        self.children.is_none()
    }
    pub fn is_root(&self) -> bool {
        self.parent.is_empty()
    }
}
