use std::collections::HashMap;

use axum::{extract::Path, http::HeaderMap, Json};
use mysql::{prelude::Queryable, PooledConn};
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    database::DBC,
    libs::time::{TimeFormat, TIME},
    parse_jwt_macro,
    response::Response,
    ResponseResult,
};

pub fn check_drop_down_box(key: &str, value: &str) -> Option<bool> {
    unsafe {
        if value.is_empty() {
            return None;
        }
        DROP_DOWN_BOX.map().get(key).map(|v| v.contains_key(value))
    }
}

pub const DROP_DOWN_BOX_ALL: [&str; 17] = [
    "customer_type",
    "customer_status",
    "customer_tag",
    "department",
    "customer_role",
    "industry",
    "customer_source",
    "visit_theme",
    "order_type",
    "sales_unit",
    "storehouse",
    "product_type",
    "product_unit",
    "payment",
    "order_progress",
    "customer_level",
    "invoice_type",
];

macro_rules! get_drop_down_box {
    ($index:expr) => {
        op::some!(DROP_DOWN_BOX_ALL.get($index); ret Err(Response::invalid_value("ty值非法")))
    };
}

#[derive(serde::Deserialize, Debug)]
struct ReceiveOptionInfo {
    ty: usize,
    info: OptionValue,
}

#[derive(serde::Deserialize, Default, Debug)]
#[serde(default)]
struct OptionValue {
    value: String,
    new_value: String,
    old_value: String,
    delete_value: String,
    next_value: String,
}
use crate::verify_perms;

use crate::perm::action::OtherGroup;
macro_rules! parse_option {
    ($headers:expr, $value:expr, $begin:expr) => {
        {
            let bearer = bearer!(&$headers);
            let mut conn = DBC.lock().await;
            let id = parse_jwt_macro!(&bearer, &mut conn => true);

            let role: String = op::some!(conn.query_first(format!("SELECT role FROM user WHERE id = '{id}'"))?; ret Err(Response::not_exist("用户不存在")));
            if !verify_perms!(&role, OtherGroup::NAME, OtherGroup::DROP_DOWN_BOX) {
                return Err(Response::permission_denied())
            }

            if $begin {
                conn.query_drop("BEGIN")?;
            }
            let info: ReceiveOptionInfo = serde_json::from_value($value)?;
            (id, conn, info)
        }
    };
}

pub async fn insert_options(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let (_, mut conn, info) = parse_option!(headers, value, false);
    let time = TIME::now()?;
    if info.info.value.is_empty() {
        return Err(Response::invalid_value("value不能为空字符串"));
    }
    let mut value = info.info.value;
    let name = *get_drop_down_box!(info.ty);
    let mut level_key: Option<String> = None;
    if info.ty == 15 {
        let level = split_level(&value);
        unsafe {
            if let Some(map) = DROP_DOWN_BOX.map().get("customer_level") {
                for v in map.keys() {
                    let l = Level::from(v.as_str());
                    if l.level == level.level {
                        level_key = Some(v.clone());
                        break;
                    }
                }
            }
        }

        value = level.to_string();
    }
    conn.query_drop(format!(
        "INSERT IGNORE INTO drop_down_box (name, value, create_time) VALUES ('{}', '{}', '{}')",
        name,
        value,
        time.format(TimeFormat::YYYYMMDD_HHMMSS)
    ))?;
    unsafe {
        if let Some(k) = level_key {
            conn.query_drop(format!(
                "DELETE FROM drop_down_box WHERE name='customer_level' AND value='{k}' LIMIT 1"
            ))?;
        }
        DROP_DOWN_BOX.init(&mut conn)?;
    }
    Ok(Response::empty())
}
pub static mut DROP_DOWN_BOX: DropDownBox = DropDownBox::new();
#[derive(Debug)]
pub struct DropDownBox {
    // pub inner: Vec<(String, String, String)>,
    pub map: Option<HashMap<String, HashMap<String, String>>>,
}
impl DropDownBox {
    pub const fn new() -> DropDownBox {
        DropDownBox {
            // inner: Vec::new(),
            map: None,
        }
    }
    pub fn init(&mut self, conn: &mut PooledConn) -> mysql::Result<()> {
        // self.inner = conn.query_map(
        //     "SELECT name, value, create_time FROM drop_down_box ORDER BY create_time",
        //     |s| s,
        // )?;

        let vec = conn.query_map(
            "SELECT name, value, create_time FROM drop_down_box ORDER BY create_time",
            |s: (String, String, String)| s,
        )?;
        let mut map: HashMap<_, HashMap<String, String>> = HashMap::new();
        for (name, v, t) in vec {
            map.entry(name).or_default().insert(v, t);
        }
        self.map = Some(map);

        Ok(())
    }
    pub fn map(&self) -> &HashMap<String, HashMap<String, String>> {
        self.map.as_ref().expect("unreachable code: map")
    }
    pub fn map_mut(&mut self) -> &mut HashMap<String, HashMap<String, String>> {
        match &mut self.map {
            Some(map) => map,
            _ => unreachable!(),
        }
    }
    pub fn remove(&mut self, name: &str, value: &str) {
        if let Some(values) = self.map_mut().get_mut(name) {
            values.remove_entry(value);
        }
    }

    pub fn contains(&self, name: &str, value: &str) -> bool {
        // self.inner.iter().any(|(n, v, _)| n.eq(name) && v.eq(value))
        self.map().get(name).and_then(|v| v.get(value)).is_some()
    }

    pub fn get(&self, name: &str) -> Vec<&str> {
        self.map()
            .get(name)
            .map(|v| {
                let mut values: Vec<(&str, &str)> =
                    v.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                if name.eq("customer_level") {
                    values.sort_by(|(k1, _), (k2, _)| k1.chars().next().cmp(&k2.chars().next()));
                } else {
                    values.sort_by(|(_, v1), (_, v2)| v1.cmp(v2))
                }
                values.into_iter().map(|(k, _)| k).collect()
            })
            .unwrap_or_default()
    }
}

struct Level {
    level: char,
    value: String,
}
impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.level, self.value))
    }
}
impl From<&str> for Level {
    fn from(value: &str) -> Self {
        let value = value.trim();
        let sp: Vec<_> = value.splitn(2, '-').collect();
        if sp.len() == 2 {
            let upper = sp[0].trim().to_uppercase();
            println!("{}", upper);
            let ch = upper.chars().next().unwrap_or(' ');
            if ch.is_ascii_uppercase() {
                return Level {
                    level: ch,
                    value: sp[1].to_string(),
                };
            }
        } else {
            let ch = value.chars().next().unwrap_or(' ').to_ascii_uppercase();
            if ch.is_ascii_uppercase() {
                return Level {
                    level: ch,
                    value: value[1..].to_owned(),
                };
            }
        }
        Level {
            level: 'Z',
            value: value.to_string(),
        }
    }
}

fn split_level(level: &str) -> Level {
    unsafe {
        let mut lsp = Level::from(level);
        if lsp.level != 'Z' {
            return lsp;
        }
        if let Some(levels) = DROP_DOWN_BOX.map().get("customer_level") {
            let mut buf = Vec::new();
            for k in levels.keys() {
                let l1 = Level::from(k.as_str());
                if lsp.level == l1.level {
                    return lsp;
                }
                buf.push(l1.level);
            }
            buf.sort();
            lsp.level = buf
                .last()
                .map(|last| op::ternary!(*last == 'Z' => 'Z'; (*last as u8 % b'Z' + 1) as char))
                .unwrap_or('A');
        } else {
            lsp.level = 'A'
        }
        lsp
    }
}

pub async fn update_option_value(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let (_, mut conn, info) = parse_option!(headers, value, true);
    // conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    commit_or_rollback!(_update, &mut conn, &info)?;

    Ok(Response::empty())
}
fn _update(conn: &mut PooledConn, param: &ReceiveOptionInfo) -> Result<(), Response> {
    let name = *get_drop_down_box!(param.ty);
    let mut new_value = param.info.new_value.clone();
    unsafe {
        if DROP_DOWN_BOX
            .map()
            .get(name)
            .is_some_and(|m| m.contains_key(&new_value))
        {
            return Err(Response::already_exist("修改的数据不能重复"));
        }
    }
    match name {
        "department" => {
            if param.info.old_value.eq("总经办") || param.info.new_value.eq("总经办") {
                return Err(Response::invalid_value("总经办这个部门不允许被修改"));
            }
            conn.query_drop(format!(
                "UPDATE user SET department = '{}' WHERE department = '{}'",
                param.info.new_value, param.info.old_value
            ))?;
        }
        "customer_level" => {
            let old_level = Level::from(param.info.old_value.as_str());
            let mut new_level = split_level(&param.info.new_value);
            new_level.level = old_level.level;
            new_value = new_level.to_string();
        }
        "storehouse" => {
            update_storehouse(conn, &param.info.old_value, &param.info.new_value)?;
        }
        _ => {}
    }

    conn.query_drop(format!(
        "UPDATE drop_down_box SET value = '{}' WHERE value = '{}' AND name = '{}' LIMIT 1",
        new_value, param.info.old_value, name
    ))?;
    unsafe {
        DROP_DOWN_BOX.init(conn)?;
        println!("{:#?}", DROP_DOWN_BOX);
    }
    Ok(())
}

fn update_storehouse(conn: &mut PooledConn, old: &str, new: &str) -> mysql::Result<()> {
    conn.exec_drop(
        "update product_store set storehouse = ? where storehouse = ?",
        (new, old),
    )?;
    conn.exec_drop(
        "update order_data set shipped_storehouse = ? where shipped_storehouse = ?",
        (new, old),
    )?;

    Ok(())
}

pub async fn delete_option_value(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let (_, mut conn, info) = parse_option!(headers, value, true);
    let name = *get_drop_down_box!(info.ty);
    if name.eq("department") {
        if info.info.delete_value.eq("总经办") {
            return Err(Response::invalid_value("总经办这个部门不允许被删除"));
        }
        unsafe {
            if !DROP_DOWN_BOX.contains("department", &info.info.next_value) {
                return Err(Response::invalid_value("next_value必须存在"));
            }
        }
    }
    commit_or_rollback!(_delete, &mut conn, (&info, name))?;
    unsafe {
        DROP_DOWN_BOX.init(&mut conn)?;
        println!("{:#?}", DROP_DOWN_BOX);
    }
    Ok(Response::empty())
}
fn _delete(
    conn: &mut PooledConn,
    (param, name): (&ReceiveOptionInfo, &str),
) -> Result<(), Response> {
    match name {
        "department" => {
            conn.query_drop(format!(
                "UPDATE user SET department = '{}' WHERE department = '{}'",
                param.info.next_value, param.info.delete_value
            ))?;
        }
        "storehouse" => {
            conn.exec_drop(
                "delete from product_store where storehouse = ?",
                (&param.info.delete_value,),
            )?;
        }
        _ => {}
    }
    conn.query_drop(format!(
        "DELETE FROM drop_down_box WHERE name = '{}' AND value = '{}' LIMIT 1",
        name, param.info.delete_value
    ))?;

    Ok(())
}

pub async fn query_option_value() -> ResponseResult {
    let data: Vec<Value> = unsafe {
        DROP_DOWN_BOX_ALL
            .iter()
            .enumerate()
            .map(|(i, k)| {
                json!({
                    "ty": i,
                    "info": DROP_DOWN_BOX.get(k)
                })
            })
            .collect()
    };
    Ok(Response::ok(json!(data)))
}

pub async fn query_specific_info(Path(ty): Path<usize>) -> ResponseResult {
    let name = *get_drop_down_box!(ty);
    let info: Vec<&str> = unsafe { DROP_DOWN_BOX.get(name) };
    Ok(Response::ok(json!({
        "ty": ty,
        "info": info
    }
    )))
}
