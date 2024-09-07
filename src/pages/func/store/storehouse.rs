use crate::{
    bearer,
    database::get_db,
    libs::{cache::STORE_HOUSE_CACHE, gen_id, TIME},
    log,
    pages::account::get_user,
    parse_jwt_macro, verify_perms, Response, ResponseResult,
};
use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::Storehouse;

pub fn storehouse_router() -> Router {
    Router::new()
        .route("/store/query/storehouse", get(query_storehouse))
        .route("/store/create/storehouse", post(create_storehouse))
        .route("/store/update/storehouse", post(update_storehouse))
        .route("/store/delete/storehouse/:id", delete(delete_storehouse))
}

#[derive(Deserialize, Serialize)]
struct QueryParam {
    page: usize,
    limit: usize,
    #[serde(default)]
    total: usize,
    #[serde(default)]
    records: Vec<Storehouse>,
}

async fn query_storehouse(Json(mut param): Json<QueryParam>) -> ResponseResult {
    let mut buf: Vec<Storehouse> = STORE_HOUSE_CACHE
        .iter()
        .map(|v| v.value().clone())
        .collect();
    buf.sort_by(|v1, v2| v1.create_time.cmp(&v2.create_time));
    param.total = buf.len();
    let max = buf.len().min(param.limit * param.page);
    param.records = buf[param.limit * (param.page - 1)..max].to_owned();
    Ok(Response::ok(json!(param)))
}

async fn create_storehouse(header: HeaderMap, Json(mut value): Json<Storehouse>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    if !verify_perms!(&user.role, "storehouse", "add_storehouse") {
        return Err(Response::permission_denied());
    }
    let time = TIME::now()?;
    value.id = gen_id(&time, "storehouse");
    value.create_time = time.format(crate::libs::TimeFormat::YYYYMMDD_HHMMSS);
    conn.exec_drop(
        "insert into storehouse (id, name, create_time, description) 
            values (?, ?, ?, ?)",
        (
            &value.id,
            &value.name,
            &value.create_time,
            &value.description,
        ),
    )?;
    log!("{user}成功创建仓库 {}", value.name);
    STORE_HOUSE_CACHE.insert(value.id.clone(), value);
    Ok(Response::ok(json!("创建仓库成功")))
}

async fn update_storehouse(header: HeaderMap, Json(mut value): Json<Storehouse>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    if !verify_perms!(&user.role, "storehouse", "update_storehouse") {
        return Err(Response::permission_denied());
    }
    if let Some(store) = STORE_HOUSE_CACHE.get(&value.id) {
        value.create_time = store.create_time.clone();
        conn.exec_drop(
            "update storehouse set name=?, description=? where id = ? limit 1",
            (&value.name, &value.description, &value.id),
        )?;
        drop(store);
        let stmt = format!("{user}成功更新库房{}", value.id);
        STORE_HOUSE_CACHE.insert(value.id.clone(), value);
        log!("{}", stmt);
        Ok(Response::ok(json!(stmt)))
    } else {
        Err(Response::not_exist(format!("不存在仓库 {}", value.id)))
    }
}

async fn delete_storehouse(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    if !verify_perms!(&user.role, "storehouse", "delete_storehouse") {
        return Err(Response::permission_denied());
    }
    if STORE_HOUSE_CACHE.contains_key(&id) {
        conn.exec_drop("delete from storehouse where id = ? LIMIT 1", (&id,))?;
        let stmt = format!("{user}成功删除库房{id}");
        log!("{}", stmt);
        STORE_HOUSE_CACHE.remove(&id);
        Ok(Response::ok(json!(stmt)))
    } else {
        Err(Response::not_exist(format!("不存在仓库 {}", id)))
    }
}
