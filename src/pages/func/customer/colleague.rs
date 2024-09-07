use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::{json, Value};

use crate::{
    bearer,
    database::get_db,
    libs::{gen_id, time::TIME, TimeFormat},
    log,
    pages::account::get_user,
    parse_jwt_macro, Response, ResponseResult,
};

pub fn colleague_router() -> Router {
    Router::new()
        .route("/customer/colleague/data/:customer", get(query_colleagues))
        .route(
            "/customer/colleague/insert/:customer",
            post(insert_colleague),
        )
        .route("/customer/colleague/update", post(update_colleague))
        .route("/customer/colleague/delete/:id", delete(delete_colleague))
}

#[derive(serde::Deserialize, serde::Serialize, FromRow, Debug)]
struct Colleague {
    #[serde(default)]
    id: String,
    phone: String,
    name: String,
}
use super::index::check_user_customer;

async fn insert_colleague(
    headers: HeaderMap,
    Path(customer): Path<String>,
    Json(value): Json<Value>,
) -> ResponseResult {
    let bearer = bearer!(&headers);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn).await?;
    let mut params: Colleague = serde_json::from_value(value)?;
    log!(
        "{user} 发起添加客户联系人请求，客户{}, 客户联系人{}",
        params.id,
        params.name
    );
    check_user_customer(&id, &customer, &mut conn)?;
    let time = TIME::now()?;
    params.id = gen_id(&time, &params.name);
    conn.query_drop(format!(
        "INSERT INTO customer_colleague (id, customer, phone, name, create_time) VALUES (
        '{}', '{}', '{}', '{}', '{}')",
        params.id,
        customer,
        params.phone,
        params.name,
        time.format(TimeFormat::YYYYMMDD_HHMMSS)
    ))?;

    log!(
        "{user} 成功添加客户联系人，客户{}, 客户联系人{}",
        params.id,
        params.name
    );
    Ok(Response::ok(json!(params.id)))
}

async fn update_colleague(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: Colleague = serde_json::from_value(value)?;
    check(&id, &params.id, &mut conn)?;
    conn.query_drop(format!(
        "UPDATE customer_colleague SET phone = '{}', name = '{}' WHERE  id = '{}' LIMIT 1",
        params.phone, params.name, params.id,
    ))?;
    Ok(Response::empty())
}

fn check(id: &str, col: &str, conn: &mut PooledConn) -> Result<(), Response> {
    let query = format!(
        "SELECT 1 FROM user u
        JOIN extra_customer_data ex ON ex.salesman=u.id
         JOIN customer_colleague c ON c.customer = ex.id
         WHERE c.id='{col}' AND u.id = '{id}' LIMIT 1"
    );
    println!("{}", query);
    let flag: Option<String> = conn.query_first(query)?;
    if flag.is_some() {
        Ok(())
    } else {
        Err(Response::permission_denied())
    }
}

async fn delete_colleague(headers: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let user_id = parse_jwt_macro!(&bearer, &mut conn => true);
    check(&user_id, &id, &mut conn)?;
    conn.query_drop(format!(
        "DELETE FROM customer_colleague WHERE id = '{}' LIMIT 1",
        id,
    ))?;
    Ok(Response::empty())
}

async fn query_colleagues(Path(customer): Path<String>) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let data: Vec<Colleague> = conn.query(format!(
        "SELECT id, name, phone FROM customer_colleague WHERE customer='{}' ORDER BY create_time",
        customer
    ))?;
    Ok(Response::ok(json!(data)))
}
