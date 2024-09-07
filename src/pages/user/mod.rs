use std::collections::HashMap;

use axum::{extract::Path, http::HeaderMap, routing::post, Json, Router};
use mysql::prelude::Queryable;
use serde_json::json;

use crate::{
    bearer, database::get_db, libs::dser::deserialize_roles, parse_jwt_macro, Response, ResponseResult,
};

use super::account::User;

pub fn user_router() -> Router {
    Router::new()
        .route("/user/name/:id", post(get_user_name))
        .route("/user/list/limit", post(query_limit_user))
}

async fn get_user_name(Path(id): Path<String>) -> ResponseResult {
     let db = get_db().await?;
    let mut conn = db.lock().await;
    let name: Option<String> =
        conn.query_first(format!("SELECT name FROM user WHERE id = '{id}' LIMIT 1"))?;
    Ok(Response::ok(json!(name)))
}

#[derive(serde::Deserialize)]
struct LimitParams {
    customer: String,
    #[serde(deserialize_with = "deserialize_roles")]
    roles: Vec<String>,
}
async fn query_limit_user(
    header: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let _uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: LimitParams = serde_json::from_value(value)?;
    let filter = if data.customer.is_empty() {
        String::new()
    } else {
        format!("and exists (select 1 from extra_customer_data ex where ex.id='{}' and ex.salesman=u.id)", data.customer)
    };
    // TODO 后面需要考虑共享情况
    let users: Vec<User> = conn.query(format!(
        "select * from user u
        where NOT EXISTS (SELECT 1 FROM leaver l WHERE l.id=u.id) {filter}"
    ))?;
    let mut map: HashMap<String, Vec<User>> = HashMap::new();
    for u in users {
        if data.roles.is_empty() || data.roles.contains(&u.role) {
            map.entry(u.department.clone()).or_default().push(u);
        }
    }
    let values: Vec<serde_json::Value> = map
        .into_iter()
        .map(|(k, v)| {
            serde_json::json!({
                "department": k,
                "data": v
            })
        })
        .collect();

    Ok(Response::ok(serde_json::json!(values)))
}
