use super::{get_user, User};

use crate::database::get_db;
use crate::libs::{dser::*, gen_id, TIME};
use crate::pages::check_drop_down_box;
use crate::perm::action::AccountGroup;
use crate::perm::roles::ROLE_TABLES;
use crate::{bearer, parse_jwt_macro, Response, ResponseResult};
use crate::{catch, verify_perms};
use axum::{http::HeaderMap, Json};
use mysql::{params, prelude::Queryable};
use serde_json::{json, Value};

/// 12345678 的md5值
pub static DEFAULT_PASSWORD: [u8; 16] = [
    37, 213, 90, 210, 131, 170, 64, 10, 244, 100, 199, 109, 113, 60, 7, 173,
];

#[derive(serde::Deserialize)]
struct Root {
    #[serde(skip_deserializing)]
    id: String,
    smartphone: String,
    password: String,
    name: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
}

macro_rules! __insert_user {
    ($conn:expr, $params:expr) => {
        $conn.exec_drop(
            "INSERT INTO user (id, smartphone, password, name,  sex, role, department) VALUES (
                :id, :smartphone, :password, :name, :sex, :role, :department)",
            $params,
        )
    };
}
pub async fn register_root(Json(value): Json<Value>) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let mut root: Root = serde_json::from_value(value)?;
    let k: Option<String> = conn.query_first("SELECT 1 FROM user WHERE role = 'root'")?;
    if k.is_some() {
        return Err(Response::dissatisfy("只允许有一位最高权限者"));
    }

    root.id = gen_id(&TIME::now()?, &root.name);
    __insert_user!(
        conn,
        params! {
            "id" => root.id,
            "smartphone" => root.smartphone,
            "password" => md5::compute(root.password.as_bytes()).0,
            "name" => root.name,
            "sex" => root.sex,
            "role" => "root",
            "department" => "总经办",
        }
    )?;
    Ok(Response::ok(json!({})))
}

pub async fn register_user(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut regis: User = serde_json::from_value(value)?;
    if let Some(true) = check_drop_down_box("department", &regis.department) {
        // nothing
    } else {
        return Err(Response::not_exist("部门不存在"));
    }
    let adm = get_user(&id, &mut conn).await?;
    regis.id = gen_id(&TIME::now()?, &regis.name);
    if ver_user_perm(&adm, &regis).await {
        catch!(__insert_user!(conn, params! {
            "id" => regis.id,
            "password" => DEFAULT_PASSWORD,
            "name" => regis.name,
            "role" => regis.role,
            "department" => regis.department,
            "sex" => regis.sex,
            "smartphone" => regis.smartphone
        }) => dup)?;
        Ok(Response::ok(json!({})))
    } else {
        Err(Response::permission_denied())
    }
}
/// 验证用户创建账号的权限
async fn ver_user_perm(adm: &User, regis: &User) -> bool {
    let role_name = unsafe { op::some!(ROLE_TABLES.get_name(&regis.role); ret false) };
    adm.role.eq("root")
        || (adm.department == regis.department
            && verify_perms!(
                &adm.role,
                AccountGroup::NAME,
                AccountGroup::CREATE,
                Some([role_name].as_slice())
            ))
}
