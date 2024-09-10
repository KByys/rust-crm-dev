use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::{json, Value};

mod login;
// mod logout;
mod register;
use crate::{
    bearer,
    database::{get_db, DB},
    libs::{
        cache::{TOKEN_CACHE, USER_CACHE},
        dser::*,
        time::TIME,
    },
    parse_jwt_macro,
    perm::verify_permissions,
    Response, ResponseResult,
};

/// 员工数据
#[derive(Debug, serde::Serialize, FromRow, serde::Deserialize, Clone)]
pub struct User {
    #[serde(default)]
    pub id: String,
    pub smartphone: String,
    pub name: String,
    #[allow(unused)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub password: Vec<u8>,
    #[serde(default)]
    pub department: String,
    #[serde(deserialize_with = "deserialize_role")]
    #[serde(serialize_with = "serialize_role")]
    pub role: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    pub sex: i32,
}
impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.department, self.name))
    }
}
pub fn account_router() -> Router {
    Router::new()
        .route("/user/login", post(login::user_login))
        .route("/root/register", post(register::register_root))
        .route("/user/list/:id", post(query_list_data))
        .route("/user/count/:id", post(query_depart_count))
        // .route("/customer/login", post(login::customer_login))
        .route("/user/register", post(register::register_user))
        .route("/user/set/psw", post(set_user_password))
        .route("/user/full/data/:id", post(query_full_data))
        // .route("/customer/set/psw", post(set_customer_password))
        .route("/role/infos", get(get_role))
}

async fn get_role() -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let roles = conn.query_map("SELECT name FROM roles WHERE id != 'root'", |s: String| s)?;
    Ok(Response::ok(json!(roles)))
}

#[derive(serde::Deserialize)]
struct Password {
    password: String,
}
// async fn set_customer_password(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
//     let bearer = bearer!(&headers);
//     let mut conn = get_conn()?;
//     let id = parse_jwt_macro!(&bearer, &mut conn => false);
//     let password: Password = serde_json::from_value(value)?;
//     let digest = md5::compute(password.password);
//     let time = TIME::now()?;
//     conn.exec_drop(
//         "UPDATE customer_login SET password = :password WHERE id = :id",
//         params! {
//             "password" => digest.0,
//             "id" => &id
//         },
//     )?;
//     conn.query_drop(format!(
//         "INSERT INTO token (ty, id, tbn) VALUES (1, '{}', {}) ON DUPLICATE KEY UPDATE tbn = {}",
//         id,
//         time.naos(),
//         time.naos()
//     ))?;
//     Ok(Response::empty())
// }
async fn set_user_password(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let password: Password = serde_json::from_value(value)?;
    let digest = md5::compute(password.password);
    let time = TIME::now()?;
    conn.exec_drop(
        "UPDATE user SET password = :password WHERE id = :id",
        params! (
            "password" => digest.0,
            "id" => &id
        ),
    )?;
    conn.query_drop(format!(
        "INSERT INTO token (ty, id, tbn) VALUES (0, '{}', {}) ON DUPLICATE KEY UPDATE tbn = {}",
        id,
        time.naos(),
        time.naos()
    ))?;
    TOKEN_CACHE.clear();
    Ok(Response::empty())
}

pub async fn get_user<'err>(id: &str, conn: &mut DB<'err>) -> Result<Arc<User>, Response> {
    if let Some(user) = USER_CACHE.get(id) {
        Ok(Arc::clone(user.value()))
    } else {
        let query = format!(
            "SELECT u.* FROM user u WHERE u.id = '{id}' 
        AND NOT EXISTS (SELECT 1 FROM leaver l WHERE l.id=u.id) LIMIT 1"
        );
        let result = conn.query_first(query)?;
        let u: User = op::some!(result; ret Err(Response::not_exist("用户不存在")));
        let u = Arc::new(u);
        USER_CACHE.insert(id.to_owned(), Arc::clone(&u));
        Ok(u)
    }
}
pub fn get_user_with_phone_number(number: &str, conn: &mut PooledConn) -> Result<User, Response> {
    let u: User = op::some!(conn.query_first(format!("SELECT u.* FROM user u WHERE u.smartphone = '{number}' 
        AND NOT EXISTS (SELECT 1 FROM leaver l WHERE l.id=u.id) LIMIT 1"))?; ret Err(Response::not_exist("手机号错误，用户不存在")));
    Ok(u)
}

async fn query_depart_count(header: HeaderMap, Path(depart): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let u = get_user(&id, &mut conn).await?;
    let count: usize = match depart.as_str() {
        "all" => conn
            .query::<i32, &str>(
                "SELECT 1 FROM user u WHERE NOT EXISTS 
                (SELECT 1 FROM leaver l WHERE l.id=u.id)",
            )?
            .len(),
        _ => conn
            .query::<i32, String>(format!(
                "SELECT 1 FROM user u WHERE u.department='{}' AND NOT EXISTS 
                (SELECT 1 FROM leaver l WHERE l.id=u.id)",
                op::ternary!(depart.eq("my")
            => &u.department; &depart)
            ))?
            .len(),
    };
    Ok(Response::ok(json!(count)))
}

async fn query_full_data(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let _id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user: Option<User> =
        conn.query_first(format!("SELECT * FROM user WHERE id = '{id}' LIMIT 1"))?;
    Ok(Response::ok(json!(user)))
}

async fn query_list_data(header: HeaderMap, Path(depart): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let u = get_user(&id, &mut conn).await?;
    let data: Vec<Value> = match depart.as_str() {
        "all" => {
            if !verify_permissions(&u.role, "other", "company_staff_data", None).await {
                return Err(Response::permission_denied());
            }
            let users: Vec<User> = conn.query(
                "SELECT u.* FROM user u WHERE NOT EXISTS 
                   (SELECT 1 FROM leaver l WHERE l.id=u.id)",
            )?;
            let mut map: HashMap<String, Vec<User>> = HashMap::new();
            for u in users {
                map.entry(u.department.clone()).or_default().push(u);
            }
            map.into_iter()
                .map(|(k, v)| {
                    json!({
                        "department": k,
                        "data": v
                    })
                })
                .collect()
        }
        _ => {
            if !depart.eq("my")
                && !verify_permissions(&u.role, "other", "company_staff_data", None).await
            {
                return Err(Response::permission_denied());
            }
            let d = op::ternary!(depart.eq("my") => &u.department; &depart);
            let query = format!(
                "SELECT u.* FROM user u WHERE u.department='{d}' AND NOT EXISTS 
                (SELECT 1 FROM leaver l WHERE l.id=u.id)"
            );
            println!("{}", query);
            let users: Vec<User> = conn.query(query)?;
            vec![json!({
                "department": d,
                "data": users
            })]
        }
    };
    Ok(Response::ok(json!(data)))
}
