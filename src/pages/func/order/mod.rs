mod commission;
pub mod data;
mod update;
use commission::get_commission;
pub use data::Order;
use std::{fmt::Display, sync::Arc};
mod customer;
mod invoice;
mod payment;
mod product;
mod ship;

use axum::{
    extract::{Multipart, Path},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use payment::Instalment;
use product::Product;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    database::{get_db, DB},
    get_cache,
    libs::{cache::{ORDER_CACHE, ORDER_CACHE_WITH_ID}, gen_file_link, gen_id, parse_multipart, TimeFormat, TIME},
    log,
    pages::account::{get_user, User},
    parse_jwt_macro,
    perm::action::OtherGroup,
    response::BodyFile,
    verify_perms, Response, ResponseResult,
};
fn verify_instalment(product: &[Product], instalment: &[Instalment]) -> Result<(), Response> {
    log!("接收到的产品信息: \n {:#?}", product);
    let sum = product::computed_products_sum(product);

    let instalment_sum = Instalment::computed_instalment(instalment);
    let result = sum - instalment_sum;
    if (-0.001f32..=0.001f32).contains(&result) {
        Ok(())
    } else {
        Err(Response::invalid_value(format!(
            "回款金额错误，误差超过0.001, 预期值：{sum}, 实际值：{instalment_sum} (已包括折扣)"
        )))
    }
}
pub fn order_router() -> Router {
    Router::new()
        .route("/order/add", post(add_order))
        .route("/order/query", post(query_order))
        .route("/order/tran", post(update::order_transaction))
        .route("/order/finish/:id", post(update::complete_order))
        .route("/order/update/order", post(update::update_order))
        .route("/order/finish/repayment", post(finish_repayment))
        .route("/order/upload/image/:id", post(upload_order_file))
        .route("/order/delete/:id", delete(delete_order))
        .route("/order/get/commission", get(get_commission))
        .route("/order/get/img/:url", get(get_order_file))
        .route(
            "/order/set/commission/:value",
            post(commission::set_commission),
        )
}

async fn upload_order_file(
    header: HeaderMap,
    Path(id): Path<String>,
    part: Multipart,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data = parse_multipart(part).await?;
    let Some(f) = data.files.first() else {
        return Err(Response::invalid_value("没有接收到附件信息"));
    };
    let order = query_order_by_id(&mut conn, &id)?;
    if order.salesman.id != uid {
        log!("上传附件失败，该订单不存在或权限不足");
        return Err(Response::permission_denied());
    }
    let time = TIME::now()?;
    let link = gen_file_link(&time, f.filename());
    std::fs::write(format!("resources/order/{link}"), &f.bytes)?;
    conn.query_drop(format!(
        "update order_data set file = '{link}' where id = '{id}' limit 1"
    ))?;
    if let Some(path) = &order.file {
        std::fs::remove_file(path).unwrap_or_default();
    }
    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    log!("添加订单附件成功");
    Ok(Response::ok(json!("添加订单附件成功")))
}

async fn add_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut order: Order = serde_json::from_value(value)?;
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 发起添加订单请求", user.department, user.name);
    commit_or_rollback!(async __add_order, &mut conn, &mut order, &user)?;

    log!(
        "{}-{} 添加订单成功, 订单编号为：{}",
        user.department,
        user.name,
        order.number
    );
    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    Ok(Response::ok(json!({"id": order.id})))
}
pub fn gen_number(conn: &mut PooledConn, ty: i32, name: impl Display) -> Result<String, Response> {
    use rust_pinyin::get_pinyin;
    let pinyin = get_pinyin(&format!("{}", name));
    let number = conn
        .exec_first(
            "select num from order_num where name = ? and ty = ?",
            (&pinyin, ty),
        )?
        .unwrap_or(0)
        + 1;
    conn.exec_drop(
        "insert into order_num 
                (name, ty, num) 
                values (:name, :ty, :num) 
                on duplicate key update num = :new_num",
        params! {
            "name" => &pinyin,
            "ty" => ty,
            "num" => number,
            "new_num" => number
        },
    )?;
    Ok(format!("NO.{}{:0>7}", pinyin, number))
}

async fn __add_order(
    conn: &mut PooledConn,
    order: &mut Order,
    user: &User,
) -> Result<(), Response> {
    let time = TIME::now()?;
    order.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    order.gen_number(conn)?;
    order.id = gen_id(&time, &format!("order{}", user.name));

    match order.status {
        1 | 2 => {
            if order.ship.shipped == 1 && order.ship.date.is_none() {
                order.ship.date = Some(time.format(TimeFormat::YYYYMMDD_HHMMSS));
            }
            order.transaction_date = Some(time.format(TimeFormat::YYYYMMDD_HHMMSS));
            verify_instalment(&order.product, &order.instalment)?;
        }
        _ => {
            order.ship.shipped = 0;
            order.invoice.required = 0;
        }
    }
    
    order.insert(conn)
}

fn query_order_by_id(conn: &mut PooledConn, id: &str) -> Result<Arc<Order>, Response> {
    if let Some(order) = ORDER_CACHE_WITH_ID.get(id) {
        return Ok(Arc::clone(&order));
    }
    let order: Option<Order> = conn.query_first(format!(
        "{QUERY_ORDER}
        where o.id = '{id}' limit 1
    "
    ))?;

    if let Some(mut order) = order {
        order.query_other(conn)?;
        let order = Arc::new(order);
        ORDER_CACHE_WITH_ID
            .insert(id.into(), Arc::clone(&order));
        Ok(order)
    } else {
        log!("订单 {} 不存在", id);
        Err(Response::not_exist("订单不存在"))
    }
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    ty: u8,
    data: String,
    #[serde(default)]
    limit: u32,
    status: i32,
}
static QUERY_ORDER: &str = "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company
        from order_data o
        join user u on u.id = o.salesman
        join customer c on c.id = o.customer";
async fn query_person_order<'err>(
    conn: &mut DB<'err>,
    param: &QueryParams,
    user: &User,
    status: &str,
) -> Result<Vec<Order>, Response> {
    let id = if param.data.eq("my") || user.id == param.data {
        log!("{}-{} 正在查询自己的订单", user.department, user.name);
        &user.id
    } else {
        let u = get_user(&param.data, conn).await?;
        log!(
            "{}-{} 正在查询 {}-{} 的订单",
            user.department,
            user.name,
            u.department,
            u.department
        );
        if u.department == user.department {
            if !verify_perms!(&user.role, OtherGroup::NAME, OtherGroup::QUERY_ORDER) {
                log!(
                    "{}-{} 查询 {}-{} 的订单失败，因为没有查看本部门其他成员订单的权限",
                    user.department,
                    user.name,
                    u.department,
                    u.department
                );
                return Err(Response::permission_denied());
            }
            &param.data
        } else if verify_perms!(
            &user.role,
            OtherGroup::NAME,
            OtherGroup::QUERY_ORDER,
            Some(["all"].as_slice())
        ) {
            log!(
                "{}-{} 查询 {}-{} 的订单失败，因为没有查看其他部门成员订单的权限",
                user.department,
                user.name,
                u.department,
                u.department
            );
            &param.data
        } else {
            log!(
                "{}-{} 查询 {}-{} 的订单失败，因为没有查看其他成员订单的权限",
                user.department,
                user.name,
                u.department,
                u.department
            );
            return Err(Response::permission_denied());
        }
    };
    let query = format!(
        "{QUERY_ORDER}
            where o.salesman = ? and o.status {status}
            order by o.create_time desc
            limit ?
        "
    );
    conn.exec(&query, (&id, &param.limit)).map_err(Into::into)
}

async fn query_department_order(
    conn: &mut PooledConn,
    param: &QueryParams,
    user: &User,
    status: &str,
) -> Result<Vec<Order>, Response> {
    if !verify_perms!(&user.role, OtherGroup::NAME, OtherGroup::QUERY_ORDER) {
        return Err(Response::permission_denied());
    }
    let depart = if param.data.eq("my") || user.department == param.data {
        &user.department
    } else if verify_perms!(
        &user.role,
        OtherGroup::NAME,
        OtherGroup::QUERY_ORDER,
        Some(["all"].as_slice())
    ) {
        &param.data
    } else {
        log!(
            "{}-{} 查询 {} 部门的订单失败，因为没有查看其他部门订单的权限",
            user.department,
            user.name,
            param.data
        );
        return Err(Response::permission_denied());
    };
    log!(
        "{}-{} 正在查询 {depart} 部门的订单",
        user.department,
        user.name
    );
    conn.exec(
        format!(
            "{QUERY_ORDER}
        where o.status {status} and u.department = ?
        order by o.create_time desc
        limit ?
    "
        ),
        (&depart, &param.limit),
    )
    .map_err(Into::into)
}

async fn query_company_order(
    conn: &mut PooledConn,
    user: &User,
    limit: u32,
    status: &str,
) -> Result<Vec<Order>, Response> {
    log!("{}-{} 正在查询全公司的订单", user.department, user.name);
    if !verify_perms!(
        &user.role,
        OtherGroup::NAME,
        OtherGroup::QUERY_ORDER,
        Some(["all"].as_slice())
    ) {
        log!(
            "{}-{} 查询全公司的订单失败，没有该权限",
            user.department,
            user.name
        );
        return Err(Response::permission_denied());
    }
    conn.query(format!(
        "{QUERY_ORDER}
        where o.status {status}
        order by
            o.create_time desc
        limit {limit}
            "
    ))
    .map_err(Into::into)
}

async fn query_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 请求查询订单", user.department, user.name);
    let param_str = value.to_string();
    let mut param: QueryParams = serde_json::from_value(value)?;
    if param.limit == 0 {
        param.limit = 50
    }
    let status = if param.status >= 3 {
        ">= 0".to_string()
    } else {
        format!("={}", param.status)
    };
    let value = if let Some(value) = get_cache!(ORDER_CACHE, &uid, &param_str) {
        log!("缓存命中");
        value
    } else {
        log!("缓存未命中");
        let mut data = match param.ty {
            0 => query_person_order(&mut conn, &param, &user, &status).await?,
            1 => query_department_order(&mut conn, &param, &user, &status).await?,
            2 => query_company_order(&mut conn, &user, param.limit, &status).await?,
            _ => return Ok(Response::empty()),
        };
        for o in &mut data {
            o.query_other(&mut conn)?;
        }
        let value = Arc::new(data);
        ORDER_CACHE
            .entry(uid)
            .or_default()
            .insert(param_str, value.clone());
        value
    };

    log!(
        "{user} 查询订单成功，共查询到{}条记录",
        value.len()
    );
    Ok(Response::ok(json!(value)))
}

#[derive(Deserialize)]
struct PayParam {
    id: String,
    inv_index: i32,
}

async fn finish_repayment(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let param: PayParam = serde_json::from_value(value)?;
    log!(
        "{} 请求完成订单{} 第{}期 的收款",
        user,
        param.id,
        param.inv_index
    );
    let time = TIME::now()?;
    let query = format!(
        "select inv_index from order_instalment where order_id = '{}' and inv_index = {}",
        param.id, param.inv_index
    );
    // println!("{query}");
    let key: Option<i32> = conn.query_first(query)?;
    if key.is_none() {
        log!("收款失败，无法找到第{}期回款", param.inv_index);
        return Err(Response::not_exist("无法找到该分期回款"));
    }
    conn.exec_drop(
        "update order_instalment set finish = 1, date= ? where order_id= ? and inv_index = ? limit 1",
        (
            time.format(TimeFormat::YYYYMMDD_HHMMSS),
            &param.id,
            &param.inv_index,
        ),
    )?;

    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    log!(
        "{} 成功完成订单{} -  第{}期 的收款",
        user,
        param.id,
        param.inv_index
    );
    Ok(Response::ok(json!("收款成功")))
}

async fn delete_order(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{} 请求删除订单{}", user, id);
    let order = query_order_by_id(&mut conn, &id)?;
    if order.status != 0 {
        return Err(Response::dissatisfy("仅意向订单可以删除"));
    } else if order.salesman.id != user.id {
        log!("{user}删除订单{}失败，只能删除自己的订单", order.id);
        return Err(Response::permission_denied());
    }
    order.del(&mut conn)?;
    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    log!("{} 成功删除订单{}", user, id);
    Ok(Response::ok(json!("删除订单成功")))
}

async fn get_order_file(
    Path(url): Path<String>,
) -> Result<BodyFile, (axum::http::StatusCode, String)> {
    BodyFile::new_with_base64_url("resources/order", &url)
}
