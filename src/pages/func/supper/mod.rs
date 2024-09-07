use axum::{extract::Path, http::HeaderMap, routing::{delete, post}, Json, Router};
use mysql::{params, prelude::{FromValue, Queryable}, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{bearer, commit_or_rollback, database::get_db, libs::{gen_id, TIME}, log, mysql_stmt, pages::{account::get_user, func::supper}, parse_jwt_macro, Response, ResponseResult};


pub fn router() -> Router {
    Router::new()
    .route("/store/delete/supper/:id", delete(delete_supper))
    .route("/store/query/supper", post(query_supper))
    .route("/store/update/supper", post(update_supper))
    .route("/store/create/supper", post(create_supper))
}

#[derive(FromRow, Serialize, Deserialize, Clone)]
struct Custom {

}
#[derive(Default, Clone)]
struct WrapCustom {
    inner: Vec<Custom>
}
impl From<String> for WrapCustom {
    fn from(_value: String) -> Self {
        Default::default()       
    }
}
impl FromValue for WrapCustom {
    type Intermediate = String;
}
impl Serialize for WrapCustom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WrapCustom {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        Ok(WrapCustom {
            inner: Deserialize::deserialize(deserializer)?
        })
    }
}

#[derive(Deserialize, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Supper {
    #[serde(default)]
    id: String,
    company: String,
    contact: String,
    phone: String,
    mobile_phone: String,
    address: String,
    #[serde(default)]
    create_time: String,
    blank: String,
    account: String,
    custom: WrapCustom,
    remark: String
}

async fn create_supper(header: HeaderMap, Json(param): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn);
    let user = get_user(&uid, &mut conn).await?;
    log!("{user} 添加供应商 {}", param);
    let mut param: Supper = serde_json::from_value(param)?;
    commit_or_rollback!(async __create, &mut conn, &mut param)?;

    Ok(Response::ok(json!("添加成功")))

}

async fn __create(conn: &mut PooledConn, supper: &mut Supper) -> Result<(), Response> {
    let time = TIME::now()?;
    supper.id = gen_id(&time, &supper.contact);
    let stmt = mysql_stmt!(
        "supper",
        id,
        company,
        contact,
        create_time,
        phone,
        mobile_phone,
        address,
        blank,
        account,
        remark,
    );
    conn.exec_drop(
        stmt
        , params! {
            "id" => &supper.id,
            "company" => &supper.company,
            "contact" => &supper.contact,
            "create_time" => &supper.create_time,
            "phone" => &supper.phone,
            "mobile_phone" => &supper.mobile_phone,
            "address" => &supper.address,
            "blank" => &supper.blank,
            "account" => &supper.account,
            "remark" => &supper.remark
    })?;
    Ok(())
}

async fn update_supper(header: HeaderMap, Json(param): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn);
    let user = get_user(&uid, &mut conn).await?;
    log!("{user}正在修改供应商数据， 数据为：{:#?}", param);
    let mut supper: Supper = serde_json::from_value(param)?;
    commit_or_rollback!(async __update_supper, &mut conn, &mut supper)?;
    Ok(Response::ok(json!("修改成功")))
}
async fn __update_supper(conn: &mut PooledConn, supper: &mut Supper) -> Result<(), Response> {
    conn.exec_drop("update supper set company=:company,
    ", params! {
        "id" => &supper.id,
        "company" => &supper.company,
        "contact" => &supper.contact,
        "phone" => &supper.phone,
        "mobile_phone" => &supper.mobile_phone,
        "address" => &supper.address,
        "blank" => &supper.blank,
        "account" => &supper.account,
        "remark" => &supper.remark
    })?;
    Ok(())
}
async fn delete_supper(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header); 
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let _uid = parse_jwt_macro!(&bearer, &mut conn);
    conn.exec_drop("delete supper where id = ? limit 1", (&id, ))?;
    Ok(Response::ok(json!("删除成功")))
}

#[derive(Deserialize)]
struct QueryParam {
    page: usize,
    limit: usize,
    keyword: String
}
#[derive(Serialize, Default)]
struct QueryResponse {
    page: usize,
    limit: usize,
    total: usize,
    records: Vec<Supper>
}

async fn query_supper(header: HeaderMap, Json(param): Json<Value>) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let param: QueryParam = serde_json::from_value(param)?;
    let buf: Vec<Supper> = conn.query(
        "select *, 1 as custom from supper order by create_time")?;
    let mut res = QueryResponse::default();
    res.page = param.page;
    res.limit = param.limit;
    let buf: Vec<Supper> = buf.into_iter()
    .filter(|supper|
         match_keyword(&[&supper.company, &supper.contact], &param.keyword)
        )
        
    .collect();
    res.total = buf.len();
    let max = res.total.min(res.limit * (res.page + 1));
    res.records = buf[res.limit * (res.page - 1)..max].to_vec();
    Ok(Response::ok(json!(res)))

    
}

fn match_keyword(src: &[&str], key: &str) -> bool {
    if key.is_empty() {
        return true;
    }

    true
}