use std::collections::HashMap;

use axum::{
    extract::{Multipart, Path},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use chrono::{Days, TimeZone};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

use crate::{
    bearer, catch, commit_or_rollback,
    database::{get_db, DB},
    get_cache,
    libs::{gen_id, parse_multipart, TimeFormat, TIME},
    log,
    pages::{
        account::{get_user, User},
        func::{__update_custom_fields, customer::CUSTOMER_CACHE, get_custom_fields},
    },
    parse_jwt_macro,
    perm::{action::CustomerGroup, roles::role_to_name},
    verify_perms, Field, Response, ResponseResult,
};

pub fn customer_router() -> Router {
    Router::new()
        .route("/customer/list/data", post(query_customer))
        .route("/customer/full/data/:id", post(query_full_data))
        .route("/customer/update", post(update_customer))
        .route("/customer/add", post(insert_customer))
        .route("/customer/upload/excel", post(upload_excel))
}

use crate::libs::dser::{
    deser_empty_to_none, deserialize_bool_to_i32, deserialize_mm_dd, serialize_i32_to_bool,
    serialize_null_to_default,
};

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct Customer {
    pub id: String,
    pub smartphone: String,
    pub name: String,
    pub company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
    level: String,
    chat: String,
    need: String,
    create_time: String,
    fax: String,
    post: String,
    industry: String,
    #[serde(deserialize_with = "deserialize_mm_dd")]
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    role: String,
    ty: String,
    tag: String,
    pub salesman: Option<String>,
    #[serde(default)]
    salesman_name: Option<String>,
    pub visited_count: usize,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub next_visit_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_visited_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_transaction_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub push_to_sea_date: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub pop_from_sea_date: Option<String>,
    pub custom_fields: CustomCustomerData,
}

#[derive(Serialize, Debug, FromRow)]
pub struct ListData {
    id: String,
    smartphone: String,
    name: String,
    company: String,
    salesman: Option<String>,
    salesman_name: Option<String>,
    level: String,
    #[serde(serialize_with = "serialize_null_to_default")]
    next_visit_time: Option<String>,
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
    address: String,
    ty: String,
    status: String,
    create_time: String,
    visited_count: usize,
    #[serde(serialize_with = "serialize_null_to_default")]
    last_visited_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    last_transaction_time: Option<String>,
}
#[derive(Default, Debug)]
pub struct CustomCustomerData {
    pub inner: HashMap<String, Vec<Field>>,
}

impl<'de> Deserialize<'de> for CustomCustomerData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            inner: Deserialize::deserialize(deserializer)?,
        })
    }
}

impl Serialize for CustomCustomerData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}
impl From<String> for CustomCustomerData {
    fn from(_: String) -> Self {
        Self::default()
    }
}
impl mysql::prelude::FromValue for CustomCustomerData {
    type Intermediate = String;
}

fn __insert_customer(conn: &mut PooledConn, table: &InsertParams) -> Result<(), Response> {
    let time = TIME::now()?;
    let id = gen_id(&time, &table.name);
    let create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    catch!(conn.exec_drop(
                "INSERT INTO customer 
                (id, create_time, smartphone, name, company, is_share, sex, chat, need,
                fax, post, industry, birthday, address, remark, status, source,
                role, ty, tag, level) 
                VALUES
                (:id, :create_time, :smartphone, :name, :company, :is_share, :sex, :chat, :need,
                :fax, :post, :industry, :birthday, :address, :remark, :status, :source,
                :role, :ty,  :tag, :level)",
                params! {
                    "id" => &id,
                    "create_time" => create_time,
                    "smartphone" => table.smartphone.trim(),
                    "name" => table.name.trim(),
                    "company" => table.company.trim(),
                    "is_share" => table.is_share,
                    "sex" => table.sex,
                    "chat" => &table.chat,
                    "need" => &table.need,
                    "fax" => &table.fax,
                    "post" => &table.post,
                    "industry" => &table.industry,
                    "birthday" => &table.birthday,
                    "address" => &table.address,
                    "remark" => &table.remark,
                    "status" => &table.status,
                    "source" => &table.source,
                    "role" => &table.role,
                    "ty" => &table.ty,
                    "tag" => &table.tag,
                    "level" => &table.level
                }
            ) => dup)?;
    conn.exec_drop(
        "INSERT INTO extra_customer_data (id, salesman, last_transaction_time,
        added_date) VALUES (:id, :salesman, :last_transaction_time,
         :added_date) ",
        params! {
            "id" => &id,
            "salesman" => &table.salesman,
            "last_transaction_time" => mysql::Value::NULL,
            "added_date" => time.format(TimeFormat::YYYYMMDD)
        },
    )?;

    crate::pages::func::__insert_custom_fields(conn, &table.custom_fields, 0, &id)?;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct InsertParams {
    smartphone: String,
    name: String,
    company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    sex: i32,
    chat: String,
    need: String,
    fax: String,
    post: String,
    industry: String,
    #[serde(deserialize_with = "deserialize_mm_dd")]
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    level: String,
    role: String,
    ty: String,
    tag: String,
    #[serde(deserialize_with = "deser_empty_to_none")]
    salesman: Option<String>,
    custom_fields: HashMap<String, Vec<Field>>,
}

async fn insert_customer(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: InsertParams = serde_json::from_value(value)?;
    let user = get_user(&id, &mut conn).await?;
    let role = role_to_name(&user.role);
    log!(
        "{}-{} 进行添加客户操作，公司：{}，客户名：{}",
        role,
        user.name,
        params.company,
        params.name
    );
    if !verify_perms!(
        &user.role,
        CustomerGroup::NAME,
        CustomerGroup::ENTER_CUSTOMER_DATA
    ) {
        log!(
            "{}-{} 进行添加客户操作失败, 原因是权限不足",
            role,
            user.name
        );
        return Err(Response::permission_denied());
    }

    commit_or_rollback!(__insert_customer, &mut conn, &params)?;
    CUSTOMER_CACHE.clear();
    log!(
        "{}-{} 成功添加客户({}-{})",
        role,
        user.name,
        params.company,
        params.name
    );
    Ok(Response::empty())
}
#[derive(Debug, FromRow, Deserialize, Serialize)]
struct Colleague {
    id: String,
    name: String,
    phone: String,
}
#[derive(Debug)]
struct CustomerColleagues {
    inner: Vec<Colleague>,
}
impl Serialize for CustomerColleagues {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}
impl From<String> for CustomerColleagues {
    fn from(_value: String) -> Self {
        Self { inner: Vec::new() }
    }
}
impl mysql::prelude::FromValue for CustomerColleagues {
    type Intermediate = String;
}
#[derive(Serialize, Debug, FromRow)]
struct FullCustomerData {
    id: String,
    smartphone: String,
    name: String,
    company: String,
    #[serde(serialize_with = "serialize_i32_to_bool")]
    is_share: i32,
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
    level: String,
    chat: String,
    need: String,
    create_time: String,
    fax: String,
    post: String,
    industry: String,
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    role: String,
    ty: String,
    tag: String,
    pub salesman: Option<String>,
    pub salesman_name: Option<String>,
    pub visited_count: usize,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub next_visit_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_visited_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_transaction_time: Option<String>,
    pub custom_fields: CustomCustomerData,
}
fn __query_full_data(
    conn: &mut PooledConn,
    id: &str,
) -> Result<Option<FullCustomerData>, Response> {
    let time = TIME::now()?;
    let today = time.format(TimeFormat::YYYYMMDD);
    // 会出现重复数据，目前测试数据正确
    let query = format!(
        "SELECT DISTINCT c.*, ex.salesman, ex.last_transaction_time, 
            MIN(app.appointment) as next_visit_time, COUNT(cou.id) as visited_count,
            MAX(cou.appointment) as last_visited_time, 1 as custom_fields,
            uu.name as salesman_name,
            1 as colleagues
            FROM customer c 
            JOIN extra_customer_data ex ON ex.id = c.id 
            JOIN user uu ON uu.id = ex.salesman 
            LEFT JOIN appointment app ON app.customer = c.id AND app.salesman=ex.salesman
                AND app.appointment > '{today}' AND app.finish_time IS NULL
            LEFT JOIN appointment cou ON cou.customer = c.id AND cou.salesman=ex.salesman
                AND cou.finish_time IS NOT NULL
            WHERE c.id = '{id}'
            GROUP BY c.id, app.id, cou.id"
    );

    let mut data: Option<FullCustomerData> = conn.query_first(query)?;
    if let Some(d) = &mut data {
        d.custom_fields = get_custom_fields(conn, &d.id, 0)?;
    }
    Ok(data)
}

async fn query_full_data(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    if let Some(value) = get_cache!(CUSTOMER_CACHE, "full", &id) {
        log!("{user} 成功查询到客户`{}`的信息 缓存", id);
        Ok(Response::ok(value.clone()))
    } else if let Some(data) = __query_full_data(&mut conn, &id)? {
        log!("{user} 成功查询到客户`{}({})`的信息", data.name, id);
        CUSTOMER_CACHE
            .entry("full".to_string())
            .or_default()
            .insert(id, json!(data));
        Ok(Response::ok(json!(data)))
    } else {
        log!("没有找到该客户`{}`信息", id);
        Ok(Response::ok(json!(None::<()>)))
    }
}

#[derive(Deserialize)]
struct QueryParams {
    status: Option<String>,
    ty: Option<String>,
    ap: i32,
    appointment: u64,
    added_days: i32,
    #[allow(unused)]
    is_share: Value,
    salesman: String,
    department: String,
}

macro_rules! __convert {
    ($arg:expr) => {
        match &$arg {
            Some(s) => op::ternary!(s.is_empty() => "IS NOT NULL".into(); format!("='{s}'")),
            None => "=''".into()
        }
    };
    ($time:expr, $days:expr, $local:expr, $name:expr) => {
        if $days < 0 {
            format!("IS NULL OR {} IS NOT NULL", $name)
        } else {
            let t = if let Some(t) = $local.checked_sub_days(Days::new($days as u64)) {
                t
            }else {
                log!("查询客户失败，原因天数错误");
                return Err(Response::invalid_value("天数错误"))
            };
            format!(">= '{}'",TIME::from(t).format(TimeFormat::YYYYMMDD))
        }
    };
    ($param:expr, $time:expr, $local:expr => appointment) => {
        match $param.ap {
            0 => (0 , "".into()),
            1 => {
                let t = op::some!($local.checked_sub_days(Days::new($param.appointment));
                    ret Err(Response::invalid_value("天数错误")));
                (1, format!("a.finish_time >= '{}'", TIME::from(t).format(TimeFormat::YYYYMMDD)))
            }
            2 => {
                let t = op::some!($local.checked_add_days(Days::new($param.appointment));
                    ret Err(Response::invalid_value("天数错误")));
               (1, format!("a.finish_time IS NULL AND (a.appointment >= '{}' AND a.appointment <= '{} 24:00:00')",
                    $time.format(TimeFormat::YYYYMMDD), TIME::from(t).format(TimeFormat::YYYYMMDD)))
            }
            _ => return Err(Response::invalid_value("ap错误"))
        }
    };
    // ()
    ($sales:expr, $depart:expr, $u:expr, $conn:expr; auto) => {
        if $sales.is_empty() {
            if !$depart.is_empty() {
                let (depart, root) = verify_perms!(&$u.role, CustomerGroup::NAME, CustomerGroup::QUERY, None, Some(["all"].as_slice()));
                if root || (($depart.eq(&$u.department) || $depart.eq("my")) && depart) {
                    ("IS NOT NULL".to_owned(), format!("='{}'", op::ternary!($depart.eq("my") => $u.department.as_str(), $depart.as_str())))
                } else {
                    return Err(Response::permission_denied())
                }
            } else {
                if verify_perms!(&$u.role, CustomerGroup::NAME, CustomerGroup::QUERY, Some(["all"].as_slice())) {
                    ("IS NOT NULL".to_owned(), "IS NOT NULL".to_owned())
                } else{
                    return Err(Response::permission_denied())
                }
            }
        } else if $sales.eq("my") {
            (format!("='{}'", $u.id), "IS NOT NULL".to_owned())
        } else {
            let sl = get_user($sales, $conn).await?;

            let (depart, root) = verify_perms!(&$u.role, CustomerGroup::NAME, CustomerGroup::QUERY, None, Some(["all"].as_slice()));
            if root || (sl.department.eq(&$u.department) && depart) {
                (format!("='{}'", $sales), "IS NOT NULL".to_owned())
            } else{
                return Err(Response::permission_denied())
            }
        }
    }

}
async fn __query_customer_list_data<'err>(
    conn: &mut DB<'err>,
    params: &QueryParams,
    u: &User,
) -> Result<Vec<ListData>, Response> {
    let status = __convert!(params.status);
    let ty = __convert!(params.ty);
    let time = TIME::now()?;
    let local = chrono::Local.timestamp_nanos(time.naos() as i64);
    let (ot, appoint) = __convert!(&params, &time, local => appointment);
    let added_time = __convert!(time, params.added_days, local, "ex.added_date");
    let ap = if ot == 0 {
        String::new()
    } else {
        format!("JOIN appointment a ON a.customer=c.id AND a.salesman=ex.salesman AND ({appoint})")
    };

    let (salesman, department) =
        __convert!(params.salesman.as_str(), params.department, u, conn; auto);
    let today = time.format(TimeFormat::YYYYMMDD);

    let query = format!(
        "SELECT c.*,
        ex.salesman, COUNT(cou.id) as visited_count,
        u.name as salesman_name,
        MAX(cou.finish_time) AS last_visited_time,
        ex.last_transaction_time, MIN(app.appointment) as next_visit_time
        FROM customer c JOIN extra_customer_data ex ON ex.id=c.id AND (ex.salesman {salesman}) 
        AND (ex.added_date {added_time})
        JOIN user u ON u.id=ex.salesman AND (u.department {department}) 
        {ap}
        LEFT JOIN appointment app ON app.customer=c.id AND app.salesman=ex.salesman AND app.appointment>'{today}' AND app.finish_time IS NULL
        LEFT JOIN appointment cou ON cou.customer=c.id AND cou.salesman=ex.salesman AND cou.finish_time IS NOT NULL
        WHERE (c.status {status}) AND (c.ty {ty}) AND NOT EXISTS (select 1 from customer_sea cs where cs.id = c.id)
        GROUP BY c.id
        "
    );
    let list = conn.query(query)?;
    Ok(list)
}

async fn query_customer(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let param_str = value.to_string();
    let params: QueryParams = serde_json::from_value(value)?;
    log!("{user} 正在查询客户信息");
    let time1 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let value = if let Some(cache) = get_cache!(CUSTOMER_CACHE, &uid, &param_str) {
        cache.clone()
    } else {
        let list = __query_customer_list_data(&mut conn, &params, &user).await?;
        let value = json!(list);
        CUSTOMER_CACHE
            .entry(uid)
            .or_default()
            .insert(param_str, value.clone());
        value
    };

    let time2 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    println!("{:?}", time2 - time1);
    log!("{user} 成功查询到客户信息");
    Ok(Response::ok(json!(value)))
}

#[derive(Deserialize, Debug)]
struct UpdateParams {
    id: String,
    smartphone: String,
    name: String,
    company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    sex: i32,
    level: String,
    chat: String,
    need: String,
    fax: String,
    post: String,
    industry: String,
    #[serde(deserialize_with = "deserialize_mm_dd")]
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    role: String,
    ty: String,
    tag: String,
    custom_fields: HashMap<String, Vec<Field>>,
}
pub fn check_user_customer(
    id: &str,
    customer: &str,
    conn: &mut PooledConn,
) -> Result<(), Response> {
    let flag: Option<String> = conn.query_first(format!(
        "SELECT 1 FROM customer c 
            JOIN extra_customer_data d ON d.id=c.id AND d.salesman='{id}'
            WHERE c.id='{customer}'"
    ))?;
    if flag.is_some() {
        Ok(())
    } else {
        Err(Response::permission_denied())
    }
}
async fn update_customer(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: UpdateParams = serde_json::from_value(value)?;
    let user = get_user(&id, &mut conn).await?;
    let role = role_to_name(&user.role);
    log!(
        "{}-{} 更新客户 `{}-{}`的信息",
        role,
        user.name,
        params.company,
        params.name
    );
    check_user_customer(&id, &params.id, &mut conn)?;
    if !verify_perms!(
        &user.role,
        CustomerGroup::NAME,
        CustomerGroup::UPDATE_CUSTOMER_DATA
    ) {
        log!(
            "{}-{} 更新客户 `{}-{}`的信息失败，原因权限受阻",
            role,
            user.name,
            params.company,
            params.name
        );
        return Err(Response::permission_denied());
    }
    commit_or_rollback!(__update_customer, &mut conn, &params)?;
    CUSTOMER_CACHE.clear();
    log!(
        "{}-{} 成功更新客户 `{}-{}`的信息",
        role,
        user.name,
        params.company,
        params.name
    );
    Ok(Response::empty())
}
fn __update_customer(conn: &mut PooledConn, params: &UpdateParams) -> Result<(), Response> {
    conn.exec_drop(
        format!(
            "UPDATE customer SET smartphone=:smartphone, name=:name, company=:company,
        is_share=:is_share, sex=:sex, chat=:chat, level=:level,
        need=:need, fax=:fax, post=:post, industry=:industry, birthday=:birthday,
        address=:address, remark=:remark, status=:status, 
        source=:source, role=:role, ty=:ty, tag=:tag WHERE id = '{}' LIMIT 1",
            params.id
        ),
        params! {
                    "smartphone" => &params.smartphone,
                    "name" => &params.name,
                    "company" => &params.company,
                    "is_share" => params.is_share,
                    "sex" => params.sex,
                    "chat" => &params.chat,
                    "need" => &params.need,
                    "level" => &params.level,
                    "fax" => &params.fax,
                    "post" => &params.post,
                    "industry" => &params.industry,
                    "birthday" => &params.birthday,
                    "address" => &params.address,
                    "remark" => &params.remark,
                    "status" => &params.status,
                    "source" => &params.source,
                    "role" => &params.role,
                    "ty" => &params.ty,
                    "tag" => &params.tag
        },
    )?;
    __update_custom_fields(conn, &params.custom_fields, 0, &params.id)?;
    Ok(())
}

async fn upload_excel(_header: HeaderMap, part: Multipart) -> ResponseResult {
    let data = parse_multipart(part).await?;
    for f in &data.files {
        println!("{:?}", f.filename())
    }
    Ok(Response::empty())
}
