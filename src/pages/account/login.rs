use axum::{http::HeaderMap, Json};
use mysql::PooledConn;
use serde_json::{json, Value};

use crate::{
    bearer,
    database::{get_db, DB},
    libs::headers::Bearer,
    log,
    pages::account::get_user,
    perm::{roles::role_to_name, ROLES_GROUP_MAP},
    response::Response,
    token::{generate_jwt, parse_jwt, TokenVerification},
    ResponseResult,
};

use super::get_user_with_phone_number;

#[derive(serde::Deserialize)]
struct LoginID {
    smartphone: String,
    password: String,
}

pub async fn user_login(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    if let Some(bearer) = bearer!(&headers, Allow Missing) {
        match verify_login_token(&bearer, &mut conn).await {
            Ok(res) => Ok(res) ,
            Err(err) => {
                if let Ok(value) = verify_password(value, &mut conn).await {
                    Ok(value)
                } else {
                    Err(err)
                }
            }
        }
    } else {
        verify_password(value, &mut conn).await
    }
}

async fn verify_login_token<'err>(bearer: &Bearer, conn: &mut DB<'err>) -> ResponseResult {
    let token = match parse_jwt(bearer) {
        Some(token) if !token.sub => return Err(Response::token_error("客户账号无法进行员工登录")),
        None => {
            log(format_args!("非法token登录"));
            return Err(Response::token_error("Invalid token"));
        }
        Some(token) => token,
    };
    match token.verify(conn)? {
        TokenVerification::Ok => {
            let user = get_user(&token.id, conn).await?;
            log!(
                "{}-{}({})登录成功",
                role_to_name(&user.role),
                user.name,
                user.id
            );
            if user.role.eq("root") {
                Ok(Response::ok(json!({
                    "token": bearer.token(),
                    "perm": "all",
                    "info": user.as_ref()
                })))
            } else {
                let perms = ROLES_GROUP_MAP.lock().await;
                Ok(Response::ok(json!({
                    "token": bearer.token(),
                    "perm": perms.get(&user.role),
                    "info": user.as_ref()
                })))
            }
        }
        TokenVerification::Expired => {
            if token.is_refresh() {
                let user = get_user(&token.id, conn).await?;
                let token = generate_jwt(true, &token.id);
                log!(
                    "{}-{}({})登录成功, token已刷新",
                    role_to_name(&user.role),
                    user.name,
                    user.id
                );
                if user.role.eq("root") {
                    Ok(Response::ok(json!({
                        "token": token,
                        "info": user,
                        "perm": "all"
                    })))
                } else {
                    let perms = ROLES_GROUP_MAP.lock().await;
                    Ok(Response::ok(json!({
                        "token": token,
                        "info": user,
                        "perm": perms.get(&user.role)
                    })))
                }
            } else {
                log!("用户登录失败，原因: token已过期");
                Err(Response::token_error("Token已过期"))
            }
        }
        TokenVerification::Error => {
            log!("用户登录失败，原因: token非法");
            Err(Response::token_error("Invalid token"))
        }
    }
}
async fn verify_password(value: Value, conn: &mut PooledConn) -> ResponseResult {
    let params: LoginID = serde_json::from_value(value)?;
    let digest = md5::compute(&params.password);
    let user = get_user_with_phone_number(&params.smartphone, conn)?;
    if user.password.as_slice() != digest.0.as_slice() {
        log!(
            "{}-{}(手机号：{})登录失败，密码错误",
            user.role,
            user.name,
            user.smartphone
        );
        Err(Response::wrong_password())
    } else {
        let token = generate_jwt(true, &user.id);

        log!(
            "{}-{}(手机号：{})登录成功",
            user.role,
            user.name,
            user.smartphone
        );
        if user.role.eq("root") {
            Ok(Response::ok(
                json!({"token": token, "info": user, "perms": "all"}),
            ))
        } else {
            let perms = ROLES_GROUP_MAP.lock().await;
            Ok(Response::ok(
                json!({"token": token, "info": user, "perms": perms.get(&user.role)}),
            ))
        }
    }
}
