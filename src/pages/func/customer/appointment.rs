use axum::extract::Path;
use axum::routing::{delete, get, post};
use axum::{http::HeaderMap, Json, Router};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::database::{get_db, DB};
use crate::libs::dser::deser_yyyy_mm_dd_hh_mm_ss;
use crate::libs::TimeFormat;
use crate::perm::action::CustomerGroup;
use crate::perm::get_role;
use crate::{
    bearer,
    libs::{gen_id, TIME},
    parse_jwt_macro, Response, ResponseResult,
};

pub fn appointment_router() -> Router {
    Router::new()
        .route("/customer/appointment/add", post(add_appointments))
        .route("/customer/appointment/update", post(update_appointment))
        .route(
            "/customer/appointment/delete/:id",
            delete(delete_appointment),
        )
        .route("/customer/appointment/finish/:id", post(finish_appointment))
        .route(
            "/customer/appointment/data/:id/:limit",
            post(query_appointment),
        )
        .route("/customer/appoint/comment/add", post(insert_comment))
        .route("/customer/appoint/comment/update", post(update_comment))
        .route(
            "/customer/appoint/comment/delete/:id",
            delete(delete_comment),
        )
        .route("/customer/appoint/comment/query/:id", get(query_comment))
}
#[derive(Debug, Deserialize)]
struct InsertParams {
    #[serde(rename = "visitor")]
    salesman: String,
    customer: String,
    #[serde(deserialize_with = "deser_yyyy_mm_dd_hh_mm_ss")]
    appointment: String,
    theme: String,
    content: String,
    #[allow(dead_code)]
    #[serde(default)]
    notify: bool,
}

// 安排业务员拜访客户需要验证权限
// 修改和删除拜访需要拜访发起者
// 完成拜访需要拜访者
use crate::{commit_or_rollback, verify_perms};

use super::CUSTOMER_CACHE;
async fn add_appointments(
    header: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: Vec<InsertParams> = serde_json::from_value(value)?;
    commit_or_rollback!(async __add_appoint, &mut conn, (&params, &uid))?;
    CUSTOMER_CACHE.clear();
    Ok(Response::empty())
}

async fn __add_appoint(
    conn: &mut PooledConn,
    (params, uid): (&[InsertParams], &str),
) -> Result<(), Response> {
    let role = get_role(uid, conn)?;
    let flag = verify_perms!(&role, CustomerGroup::NAME, CustomerGroup::ADD_APPOINT);
    for param in params {
        let time = TIME::now()?;
        if !param.salesman.eq(uid) && !flag {
            return Err(Response::permission_denied());
        }
        let id = gen_id(&time, &rand::random::<i32>().to_string());
        conn.query_drop(format!(
            "INSERT INTO appointment 
            (id, customer, applicant, salesman, appointment, finish_time, theme, content) VALUES (
                '{}', '{}', '{}', '{}', '{}', NULL, '{}', '{}'
            )",
            id, param.customer, uid, param.salesman, param.appointment, param.theme, param.content
        ))?;
    }
    Ok(())
}

async fn delete_appointment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    commit_or_rollback!(async __delete_appointment, &mut conn, &id, &uid)?;
    CUSTOMER_CACHE.clear();
    Ok(Response::empty())
}

async fn __delete_appointment<'err>(conn: &mut DB<'err>, id: &str, uid: &str) -> Result<(), Response> {
    let _: String = op::some!(conn.query_first(
        format!("select 1 from appointment where id = '{id}' and applicant='{uid}' LIMIT 1"))?;
        ret Err(Response::permission_denied())
    );
    conn.query_drop(format!("delete from appointment where id = '{id}' limit 1"))?;
    conn.query_drop(format!(
        "delete from appoint_comment where appoint = '{id}'"
    ))?;

    Ok(())
}

async fn finish_appointment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let _: String = op::some!(conn.query_first(
        format!("select 1 from appointment where id = '{id}' and salesman='{uid}' LIMIT 1"))?;
        ret Err(Response::permission_denied())
    );
    let time = TIME::now()?;
    let finish_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!(
        "UPDATE appointment SET finish_time = '{}' WHERE id = '{}' LIMIT 1",
        finish_time, id
    ))?;
    CUSTOMER_CACHE.clear();
    Ok(Response::ok(json!(finish_time)))
}
#[derive(Deserialize)]
struct UpdateParams {
    id: String,
    visitor: String,
    appointment: String,
    theme: String,
    content: String,
    #[allow(dead_code)]
    #[serde(default)]
    notify: bool,
}

async fn update_appointment(
    header: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);

    let data: UpdateParams = serde_json::from_value(value)?;
    let _: String = op::some!(conn.query_first(
        format!("select 1 from appointment where id = '{}' and applicant='{uid}' LIMIT 1", data.id))?;
        ret Err(Response::permission_denied())
    );
    conn.query_drop(format!(
        "update appointment set salesman='{}', appointment='{}', theme='{}', content='{}' 
        where id='{}' and applicant='{}' limit 1",
        data.visitor, data.appointment, data.theme, data.content, data.id, uid
    ))?;
    CUSTOMER_CACHE.clear();
    Ok(Response::empty())
}

#[derive(Debug, Serialize, FromRow)]
struct AppointmentResponse {
    id: String,
    salesman: String,
    salesman_name: String,
    applicant: String,
    applicant_name: String,
    appointment: String,
    finish_time: Option<String>,
    theme: String,
    content: String,
}

fn join_to_json(appoint: &AppointmentResponse, comments: &[Comment]) -> Value {
    json!({
        "id": appoint.id,
        "visitor": appoint.salesman,
        "visitor_name": appoint.salesman_name,
        "applicant": appoint.applicant,
        "applicant_name": appoint.applicant_name,
        "appointment": appoint.appointment,
        "finish_time": appoint.finish_time,
        "theme": appoint.theme,
        "content": appoint.content,
        "comments": comments
    })
}

#[derive(Serialize, FromRow)]
struct Comment {
    applicant: String,
    applicant_name: String,
    id: String,
    appoint: String,
    create_time: String,
    comment: String,
}

async fn query_appointment(Path((id, limit)): Path<(String, usize)>) -> ResponseResult {
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let res: Vec<AppointmentResponse> = conn.query(format!(
        "SELECT app.*, a.name as applicant_name, s.name as salesman_name FROM appointment app
        JOIN user a ON a.id = app.applicant
        JOIN user s ON s.id = app.salesman
        WHERE app.customer = '{}' ORDER BY appointment DESC LIMIT {limit}",
        id
    ))?;
    let mut data = Vec::new();
    for a in res {
        let comments = conn.query(format!(
            "SELECT com.*, a.name as applicant_name FROM appoint_comment com 
            JOIN user a ON a.id = com.applicant
            WHERE com.appoint = '{}'",
            a.id
        ))?;
        data.push(join_to_json(&a, &comments));
    }

    Ok(Response::ok(json!(data)))
}
#[derive(Debug, Deserialize)]
struct InsertCommentParams {
    comment: String,
    appoint: String,
}

async fn insert_comment(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: InsertCommentParams = serde_json::from_value(value)?;
    let time = TIME::now()?;
    let id = gen_id(&time, "comment");
    let create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!(
        "INSERT INTO appoint_comment (id, applicant, appoint, create_time, comment) VALUES (
        '{id}', '{uid}', '{}', '{}', '{}'
    )",
        data.appoint, create_time, data.comment
    ))?;
    let name: Option<String> =
        conn.query_first(format!("select name from user where id = '{uid}' limit 1"))?;
    Ok(Response::ok(json!({
        "applicant": uid,
        "applicant_name": name,
        "id": id,
        "appoint": data.appoint,
        "create_time": create_time,
        "comment": data.comment
    })))
}
#[derive(Deserialize)]
struct UpdateCommentParams {
    id: String,
    comment: String,
}

async fn update_comment(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: UpdateCommentParams = serde_json::from_value(value)?;
    conn.query_drop(format!(
        "UPDATE appoint_comment SET comment = '{}' WHERE id = '{}' AND applicant = '{uid}' LIMIT 1
    ",
        data.comment, data.id
    ))?;
    Ok(Response::empty())
}

async fn delete_comment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    conn.query_drop(format!(
        "DELETE FROM appoint_comment WHERE id = '{id}' AND applicant = '{uid}' LIMIT 1"
    ))?;
    Ok(Response::empty())
}
async fn query_comment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let _uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let comments: Vec<Comment> = conn.query(format!(
        "select c.*, u.name as applicant_name 
        from appoint_comment c 
        join user u on u.id=c.applicant 
        where c.appoint='{id}' 
        order by c.create_time"
    ))?;
    Ok(Response::ok(json!(comments)))
}
