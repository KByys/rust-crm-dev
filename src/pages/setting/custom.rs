use std::collections::HashMap;

use axum::{extract::Path, http::HeaderMap, Json};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback, database::{get_db, DB}, libs::time::{TimeFormat, TIME}, pages::account::get_user, 
    parse_jwt_macro, perm::action::OtherGroup, verify_perms, Response, ResponseResult
};

#[derive(serde::Deserialize, Debug)]
pub struct CustomInfos {
    ty: usize,
    display: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    old_value: String,
    #[serde(default)]
    new_value: String,
}

async fn verify_perm<'err>(headers: HeaderMap, conn: &mut DB<'err>) -> Result<String, Response> {
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, conn => true);
    let user = get_user(&id, conn).await?;
    if verify_perms!(&user.role, OtherGroup::NAME, OtherGroup::CUSTOM_FIELD) {
        Ok(id)
    } else {
        Err(Response::permission_denied())
    }
}

#[derive(FromRow, serde::Serialize, Clone, Debug)]
pub struct Field {
    ty: usize,
    display: String,
    value: String,
    #[serde(skip_serializing)]
    #[allow(unused)]
    create_time: String,
}
impl Field {
    pub fn is_eq(&self, ty: usize, display: &str, value: &str) -> bool {
        self.ty == ty && self.display == display && self.value == value
    }
}

pub static mut STATIC_CUSTOM_FIELDS: CustomFields = CustomFields::new();
pub static mut STATIC_CUSTOM_BOX_OPTIONS: CustomFields = CustomFields::new();
#[derive(Debug)]
pub struct CustomFields {
    fields: Vec<Field>,
}
impl CustomFields {
    pub const fn new() -> Self {
        CustomFields { fields: vec![] }
    }
    pub fn init(&mut self, conn: &mut PooledConn, table: &str) -> mysql::Result<()> {
        self.fields = conn.query_map(format!("SELECT * FROM {table}"), |f| f)?;
        Ok(())
    }
    pub fn contains(&self, ty: usize, display: &str, value: &str) -> bool {
        self.fields
            .iter()
            .any(|f| f.ty == ty && f.display == display && f.value == value)
    }
    pub fn push(&mut self, ty: usize, display: String, value: String, create_time: String) {
        if !self.contains(ty, &display, &value) {
            self.fields.push(Field {
                ty,
                display,
                value,
                create_time,
            });
        }
    }
    pub fn remove(&mut self, ty: usize, display: &str, value: &str) {
        let mut index = 0;
        while index < self.fields.len() {
            let data = &self.fields[index];
            if data.ty == ty && data.display == display && data.value == value {
                self.fields.remove(index);
                break;
            }
            index += 1;
        }
    }
    pub fn remove_display(&mut self, ty: usize, display: &str) {
        let mut buf = Vec::new();
        for item in &self.fields {
            if item.ty != ty || item.display != display {
                buf.push(item.clone())
            }
        }
        self.fields = buf;
    }
    pub fn update(&mut self, ty: usize, display: &str, old_value: &str, new_value: String) {
        for item in &mut self.fields {
            if item.is_eq(ty, display, old_value) {
                item.value = new_value;
                break;
            }
        }
    }
    pub fn update_display(&mut self, ty: usize, old_display: &str, new_display: String) {
        for item in &mut self.fields {
            if item.ty == ty && item.display == old_display {
                item.display = new_display;
                break;
            }
        }
    }
    pub fn get_displays(&self, ty: usize) -> (Vec<&str>, Vec<&str>, Vec<&str>) {
        let mut texts = Vec::new();
        let mut times = Vec::new();
        let mut boxes = Vec::new();
        for item in &self.fields {
            if item.ty == ty {
                match item.display.as_str() {
                    "0" => texts.push(item.display.as_str()),
                    "1" => times.push(item.display.as_str()),
                    "2" => boxes.push(item.display.as_str()),
                    _ => (),
                }
            }
        }
        (texts, times, boxes)
    }
    pub fn get_fields(&self, ty: usize) -> (Vec<&str>, Vec<&str>, Vec<&str>) {
        let mut texts = Vec::new();
        let mut times = Vec::new();
        let mut boxes = Vec::new();
        for item in &self.fields {
            if item.ty == ty {
                match item.display.as_str() {
                    "0" => texts.push(item.value.as_str()),
                    "1" => times.push(item.value.as_str()),
                    "2" => boxes.push(item.value.as_str()),
                    _ => (),
                }
            }
        }
        (texts, times, boxes)
    }
    pub fn get_boxes(&self, ty: usize) -> HashMap<&str, Vec<&str>> {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for item in &self.fields {
            if item.ty == ty {
                map.entry(item.display.as_str())
                    .or_default()
                    .push(item.value.as_str());
            }
        }
        map
    }
}

pub async fn insert_custom_field(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = verify_perm(headers, &mut conn).await?;
    let data: CustomInfos = serde_json::from_value(value)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    }
    if data.value.is_empty() {
        // 字段为空字符串则忽略
        return Err(Response::empty());
    }
    if !matches!(data.display.as_str(), "0" | "1" | "2") {
        return Err(Response::invalid_value("display 非法"));
    }
    commit_or_rollback!(_insert_field, &mut conn, &data)?;
    Ok(Response::empty())
}
fn _insert_field(conn: &mut PooledConn, param: &CustomInfos) -> Result<(), Response> {
    let create_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);
    unsafe {
        if STATIC_CUSTOM_FIELDS.contains(param.ty, &param.display, &param.value) {
            return Ok(());
        }
    }
    conn.query_drop(format!(
        "INSERT INTO custom_fields (ty, display, value, create_time) VALUES ({}, '{}', '{}', '{}')",
        param.ty, param.display, param.value, create_time
    ))?;
    let id: Vec<String> = if param.ty == 0 {
        conn.query_map("SELECT id FROM customer", |s| s)?
    } else {
        conn.query_map("SELECT id FROM product", |s| s)?
    };
    if !id.is_empty() {
        let mut values: String = id.iter().fold(String::new(), |output, id| {
            format!(
                "{output} ({}, {},'{}' ,'{}', ''),",
                param.ty, param.display, param.value, id
            )
        });
        values.pop();
        conn.query_drop(format!(
            "INSERT INTO custom_field_data (fields, ty, display, id, value) VALUES {}",
            values
        ))?;
    }
    unsafe {
        STATIC_CUSTOM_FIELDS.push(
            param.ty,
            param.display.clone(),
            param.value.clone(),
            create_time,
        )
    }
    Ok(())
}

pub async fn insert_box_option(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = verify_perm(headers, &mut conn).await?;
    let data: CustomInfos = serde_json::from_value(value)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    } else if data.value.is_empty() {
        // 字段为空字符串则忽略
        return Err(Response::empty());
    }
    let create_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);
    unsafe {
        if !STATIC_CUSTOM_FIELDS.contains(data.ty, "2", &data.display) {
            return Err(Response::not_exist("该自定义下拉字段不存在"));
        }
        if STATIC_CUSTOM_BOX_OPTIONS.contains(data.ty, &data.display, &data.value) {
            return Ok(Response::empty());
        }

        conn.query_drop(format!(
        "INSERT INTO custom_field_option (ty, display, value, create_time) VALUES ({}, '{}', '{}', '{create_time}')",
        data.ty, data.display, data.value
    ))?;
        STATIC_CUSTOM_BOX_OPTIONS.push(data.ty, data.display, data.value, create_time);
    }
    Ok(Response::empty())
}

pub async fn update_custom_field(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = verify_perm(headers, &mut conn).await?;
    let data: CustomInfos = serde_json::from_value(value)?;
    if !matches!(data.display.as_str(), "0" | "1" | "2") {
        return Err(Response::invalid_value(format!(
            "display的值 `{}`，非法",
            data.display
        )));
    }
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    } else if data.new_value.is_empty() || data.old_value.is_empty() {
        return Err(Response::invalid_value("new_value 或 old_value 不能为空"));
    }
    commit_or_rollback!(_update_custom_field, &mut conn, &data)?;
    Ok(Response::empty())
}

fn _update_custom_field(conn: &mut PooledConn, param: &CustomInfos) -> Result<(), Response> {
    // 更新字段
    conn.query_drop(format!(
        "UPDATE custom_fields SET value = '{}' WHERE value = '{}' AND ty = {} AND display = '{}'",
        param.new_value, param.old_value, param.ty, param.display
    ))?;
    conn.query_drop(format!(
        "UPDATE custom_field_data SET display = '{}' WHERE display = '{}' AND fields = {} AND ty = {}",
        param.new_value, param.old_value, param.ty, param.display 
    ))?;
    if param.display.eq("2") {
        conn.query_drop(format!(
            "UPDATE custom_field_option SET display = '{}' WHERE display = '{}' AND ty = {}",
            param.new_value, param.old_value, param.ty
        ))?;
        unsafe {
            STATIC_CUSTOM_BOX_OPTIONS.update_display(
                param.ty,
                &param.old_value,
                param.new_value.to_owned(),
            )
        }
    }

    unsafe {
        STATIC_CUSTOM_FIELDS.update(
            param.ty,
            &param.display,
            &param.old_value,
            param.new_value.to_owned(),
        )
    }
    Ok(())
}

pub async fn update_box_option(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = verify_perm(headers, &mut conn).await?;

    let data: CustomInfos = serde_json::from_value(value)?;
    if data.new_value.is_empty() {
        return Ok(Response::empty());
    }

    // let table = CUSTOM_BOX_FIELDS[data.ty];
    conn.query_drop(format!(
        "UPDATE custom_field_option SET value = '{}' WHERE value = '{}' AND display = '{}' AND ty = {}",
        data.new_value, data.old_value, data.display, data.ty
    ))?;
    unsafe {
        STATIC_CUSTOM_BOX_OPTIONS.update(data.ty, &data.display, &data.old_value, data.new_value);
    }
    Ok(Response::empty())
}

pub async fn delete_custom_field(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = verify_perm(headers, &mut conn).await?;
    let data: CustomInfos = serde_json::from_value(value)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    }
    conn.query_drop("BEGIN")?;
    commit_or_rollback!(_delete_custom_field, &mut conn, &data)?;
    Ok(Response::empty())
}

fn _delete_custom_field(conn: &mut PooledConn, param: &CustomInfos) -> Result<(), Response> {
    if !matches!(param.display.as_str(), "0" | "1" | "2") {
        return Err(Response::invalid_value("display非法"));
    }
    // 删除字段
    conn.query_drop(format!(
        "DELETE FROM custom_fields WHERE value = '{}' AND ty = {} AND display = '{}'",
        param.value, param.ty, param.display
    ))?;
    // 删除客户或产品对应的字段值
    conn.query_drop(format!(
        "DELETE FROM custom_field_data WHERE display = '{}' AND ty = {} AND fields={}",
        param.value, param.display, param.ty
    ))?;
    // 删除下拉字段选项对应的字段
    if param.display.eq("2") {
        conn.query_drop(format!(
            "DELETE FROM custom_field_option WHERE display = '{}' AND ty = {}",
            param.display, param.ty
        ))?;
        unsafe {
            STATIC_CUSTOM_BOX_OPTIONS.remove_display(param.ty, &param.value);
        }
    }
    unsafe {
        STATIC_CUSTOM_FIELDS.remove(param.ty, &param.display, &param.value);
    }
    Ok(())
}

pub async fn delete_box_option(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = verify_perm(headers, &mut conn).await?;
    let data: CustomInfos = serde_json::from_value(value)?;
    // let table = CUSTOM_BOX_FIELDS[data.ty];
    conn.query_drop(format!(
        "DELETE FROM custom_field_option WHERE value = '{}' AND display = '{}' AND ty = {}",
        data.value, data.display, data.ty
    ))?;
    unsafe {
        STATIC_CUSTOM_BOX_OPTIONS.remove(data.ty, &data.display, &data.value);
    }
    Ok(Response::empty())
}

fn _get_custom_infos(ty: usize) -> Value {
    unsafe {
        let (texts, times, boxes) = STATIC_CUSTOM_FIELDS.get_fields(ty);
        let options = STATIC_CUSTOM_BOX_OPTIONS.get_boxes(ty);
        let boxes: Vec<_> = boxes
            .iter()
            .map(|v| {
                json!({
                    "display": v,
                    "values": options.get(v).unwrap_or(&vec![])
                })
            })
            .collect();
        json!({
            "ty": ty,
            "text_infos": texts,
            "time_infos": times,
            "box_infos": boxes
        })
    }
}
pub async fn get_custom_info_with(Path(ty): Path<usize>) -> ResponseResult {
    op::ternary!(ty >= 2 => return Err(Response::invalid_value("ty 错误")); ());
    Ok(Response::ok(_get_custom_infos(ty)))
}

pub async fn get_custom_info() -> ResponseResult {
    Ok(Response::ok(json!(vec![
        _get_custom_infos(0),
        _get_custom_infos(1)
    ])))
}

pub async fn query_custom_fields(Path((ty, id)): Path<(u8, String)>) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let data = crate::pages::func::get_custom_fields(&mut conn, &id, ty)?;
    Ok(Response::ok(json!(data)))
}

pub async fn query_box(Path((ty, display)): Path<(u8, String)>) -> ResponseResult {
    unsafe {
        let option = STATIC_CUSTOM_BOX_OPTIONS.get_boxes(ty as usize);
        Ok(Response::ok(json!(option.get(display.as_str()))))
    }
}


