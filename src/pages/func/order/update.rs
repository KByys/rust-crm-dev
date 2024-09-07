use axum::{extract::Path, http::HeaderMap, Json};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    database::get_db,
    libs::{cache::{ORDER_CACHE, ORDER_CACHE_WITH_ID}, TimeFormat, TIME},
    log,
    pages::{account::get_user, User},
    parse_jwt_macro, Response, ResponseResult,
};

use super::{
    customer::Customer, data::Order, invoice::Invoice, payment::Instalment, product::Product,
    query_order_by_id, ship::Ship, verify_instalment,
};

#[derive(Deserialize)]
struct TranOrder {
    id: String,
    invoice: Invoice,
    instalment: Vec<Instalment>,
    ship: Ship,
    product: Vec<Product>,
    customer: Customer,
}

pub async fn order_transaction(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let mut param: TranOrder = serde_json::from_value(value)?;
    commit_or_rollback!(__order_transaction, &mut conn, &mut param, &user)?;
    log!("{user} 成功设置订单{} 为成交订单", param.id);
    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    Ok(Response::ok(json!("订单已设为成交订单")))
}

fn __order_transaction(
    conn: &mut PooledConn,
    param: &mut TranOrder,
    user: &User,
) -> Result<(), Response> {
    let time = TIME::now()?;
    let order = query_order_by_id(conn, &param.id)?;
    if user.id != order.salesman.id {
        log!(
            "{} 无法修改 {}-{} 的订单，不可以修改他人的订单",
            user,
            order.salesman.id,
            order.salesman.name()
        );
        return Err(Response::permission_denied());
    }
    verify_instalment(&param.product, &param.instalment)?;
    if order.status == 0 {
        if param.ship.shipped == 1 && param.ship.storehouse.is_none() {
            return Err(Response::dissatisfy("ship的storehouse必须设置"));
        }
        if param.ship.shipped == 1 && param.ship.date.is_none() {
            param.ship.date = Some(time.format(TimeFormat::YYYYMMDD_HHMMSS))
        }
        if param.invoice.required == 1 {
            param.invoice.insert_or_update(
                &param.id,
                conn,
                order.salesman.name(),
                &order.customer.name,
            )?;
        }
        Instalment::insert(conn, &param.id, &param.instalment, false)?;
        Product::insert(&param.product, &param.id, conn, true)?;

        conn.exec_drop(
            "update order_data set transaction_date=:td, 
                        shipped=:sd, shipped_date=:sdd, 
                        status=1, shipped_storehouse=:ssh, 
                        invoice_required=:ir,
                        customer=:customer,
                        purchase_unit=:pu,
                        address=:address
                        where id = :id
                        limit 1",
            params! {
                "sd" => param.ship.shipped,
                "sdd" => &param.ship.date,
                "td" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
                "ssh" => &param.ship.storehouse,
                "ir" => &param.invoice.required,
                "id" => &param.id,
                "customer" => &param.customer.id,
                "address" => &param.customer.address,
                "pu" => &param.customer.purchase_unit
            },
        )?;
        Ok(())
    } else {
        Err(Response::dissatisfy("仅支持意向订单"))
    }
}

pub async fn complete_order(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let order = query_order_by_id(&mut conn, &id)?;
    log!("{:#?}", order);
    if order.status != 1 {
        return Err(Response::dissatisfy("非成交订单不可完成"));
    }
    if order.salesman.id != uid {
        log!("{user} 试图完成 {} 的订单，被系统拒绝", order.salesman.name);
        return Err(Response::permission_denied());
    }
    let flag = order.instalment.iter().any(|inv| inv.finish == 0);
    log!("{flag}");
    if flag {
        return Err(Response::dissatisfy("存在未完成的回款，无法完成订单"));
    }
    conn.exec_drop(
        "update order_data set status = 2 where id = ? limit 1",
        (&id,),
    )?;
    log!("{user}已成功将订单{}的状态设为完成", id);
    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    Ok(Response::ok(json!("订单已完成")))
}

#[derive(Deserialize)]
struct UpdateOrderParam0 {
    id: String,
    ty: String,
    receipt_account: String,
    payment_method: String,
    product: Vec<Product>,
    customer: Customer,
}
#[derive(Deserialize)]
struct UpdateOrderParam1 {
    id: String,
    ty: String,
    receipt_account: String,
    payment_method: String,
    instalment: Vec<Instalment>,
    invoice: Invoice,
    ship: Ship,
}

pub async fn update_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{user} 请求更新订单");
    commit_or_rollback!(async __update_order, &mut conn, value, &user)?;
    log!("{user} 成功更新订单");
    ORDER_CACHE.clear();
    ORDER_CACHE_WITH_ID.clear();
    Ok(Response::ok(json!("更新订单成功")))
}

async fn __update_order(conn: &mut PooledConn, value: Value, user: &User) -> Result<(), Response> {
    let Some(id) = op::catch!(value.as_object()?.get("id")?.as_str()) else {
        return Err(Response::invalid_format("缺少id"));
    };
    let order = query_order_by_id(conn, id)?;
    if order.salesman.id != user.id {
        log!(
            "用户{}无法修改{}-{}的订单",
            user,
            order.salesman.id,
            order.salesman.name
        );
        return Err(Response::permission_denied());
    }
    if order.status == 0 {
        let mut param: UpdateOrderParam0 = serde_json::from_value(value)?;
        update_status0(conn, &mut param)
    } else if order.status == 1 {
        let mut param: UpdateOrderParam1 = serde_json::from_value(value)?;
        update_status1(conn, user, &mut param, &order)
    } else {
        log!(
            "系统拒绝{}修改订单{}的状态，因为该订单处于已完成状态",
            user,
            id
        );
        Err(Response::dissatisfy("该订单处于已完成状态, 不允许被修改"))
    }
}

fn update_status0(conn: &mut PooledConn, param: &mut UpdateOrderParam0) -> Result<(), Response> {
    conn.exec_drop(
        "update order_data set ty=:ty, receipt_account=:ra, 
        payment_method=:pm, 
        customer=:customer, 
        address=:address,
        purchase_unit=:purchase_unit
        where id=:id limit 1
     ",
        params! {
            "ty" => &param.ty,
            "ra" => &param.receipt_account,
            "pm" => &param.payment_method,
            "id" => &param.id,
            "customer" => &param.customer.id,
            "address" => &param.customer.address,
            "purchase_unit" => &param.customer.purchase_unit,
        },
    )?;
    Product::insert(&param.product, &param.id, conn, true)?;

    Ok(())
}

fn update_status1(
    conn: &mut PooledConn,
    user: &User,
    param: &mut UpdateOrderParam1,
    order: &Order,
) -> Result<(), Response> {
    let time = TIME::now()?;
    if param.ship.shipped == 1 && param.ship.storehouse.is_none() {
        log!(
            "系统拒绝{}修改订单{}的状态，当设置成发货状态时，storehouse必须设置",
            user,
            param.id
        );
        return Err(Response::dissatisfy("ship的storehouse必须设置"));
    }
    if param.ship.date.is_none() {
        param.ship.date = Some(time.format(TimeFormat::YYYYMMDD_HHMMSS))
    }
    let already_finish = order.instalment.iter().any(|v| v.finish == 1);
    if !already_finish {
        verify_instalment(&order.product, &param.instalment)?;
        for inv in &mut param.instalment {
            inv.finish = 0;
        }
        Instalment::insert(conn, &order.id, &param.instalment, true)?;
    }
    if param.invoice.required == 1 {
        param
            .invoice
            .insert_or_update(&param.id, conn, &order.salesman.id, &order.customer.id)?;
    } else {
        param.invoice.delete(&param.id, conn)?;
    }
    conn.exec_drop(
        "update order_data set ty=:ty, receipt_account=:ra, payment_method=:pm, invoice_required=:ir, 
            shipped=:ship, shipped_date=:date, shipped_storehouse=:storehouse, transaction_date=:trdate
            where id=:id limit 1
     ",
        params! {
            "ty" => &param.ty,
            "ra" => &param.receipt_account,
            "pm" => &param.payment_method,
            "trdate" => TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS),
            "id" => &param.id,
            "ir" => &param.invoice.required,
            "ship" => &param.ship.shipped,
            "storehouse" => &param.ship.storehouse,
            "date" => &param.ship.date
        },
    )?;

    Ok(())
}
