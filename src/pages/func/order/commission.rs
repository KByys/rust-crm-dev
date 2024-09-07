use axum::{extract::Path, http::HeaderMap};
use serde_json::json;

use crate::{
    bearer, database::get_db, log, pages::account::get_user, parse_jwt_macro, Response,
    ResponseResult,
};

pub async fn get_commission() -> ResponseResult {
    Ok(Response::ok(json!({
        "commission": crate::get_commission()?
    })))
}
pub async fn set_commission(header: HeaderMap, Path(value): Path<i32>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    if user.role.eq("root") {
        crate::set_commission(value)?;
        log!("已修改提成为{value}%");
        Ok(Response::ok(json!("成功修改提成")))
    } else {
        log!("仅老总权限可设置提成");
        Err(Response::permission_denied())
    }
}
