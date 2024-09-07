use axum::{http::HeaderMap, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{bearer, database::get_db, pages::account::get_user, parse_jwt_macro, Response, ResponseResult};

pub fn custom_router() -> Router {
    Router::new()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CustomField {
    ty: i32,
    display: String,
    value: String,
    #[serde(skip_serializing)]
    #[serde(default)]
    old_value: String,
    #[serde(skip_serializing)]
    #[serde(default)]
    new_value: String,
}


async fn add_custom_field(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    Ok(Response::ok(json!("")))
}
