pub mod supper;
pub mod store;
mod order;
pub use order::Order;
mod product;
pub use product::DEFAULT_PRODUCT_COVER;
mod report;
use std::collections::HashMap;

use axum::Router;
use mysql::prelude::Queryable;

use crate::{log, Response};

use self::customer::index::CustomCustomerData;

mod customer;

pub fn func_router() -> Router {
    customer::customer_router()
        .merge(product::product_router())
        .merge(report::report_router())
        .merge(order::order_router())
        .merge(store::store_router())
        .merge(supper::router())
}

pub fn verify_custom_fields(ver: &[&str], data: &[crate::Field]) -> bool {
    ver.len() == data.len() && {
        data.iter()
            .all(|info| ver.iter().any(|v| info.display.eq(v)))
    }
}

pub fn get_custom_fields(
    conn: &mut mysql::PooledConn,
    id: &str,
    fields: u8,
) -> Result<CustomCustomerData, Response> {
    let data: Vec<(String, String, String)> = conn.query(format!(
        "SELECT ty, display, value FROM custom_field_data WHERE
    fields={fields} AND id = '{id}'"
    ))?;
    let mut fields = CustomCustomerData::default();
    for (ty, display, value) in data {
        let text = match ty.as_str() {
            "0" => "texts",
            "1" => "times",
            "2" => "boxes",
            _ => return Err(Response::unknown_err("意外错误，不可到达")),
        };
        fields
            .inner
            .entry(text.to_owned())
            .or_default()
            .push(crate::Field { display, value })
    }
    for t in ["texts", "times", "boxes"] {
        fields.inner.entry(t.to_owned()).or_default();
    }
    Ok(fields)
}

pub fn __update_custom_fields(
    conn: &mut mysql::PooledConn,
    fields: &HashMap<String, Vec<crate::Field>>,
    field: u8,
    id: &str,
) -> Result<(), Response> {
    for (k, v) in fields {
        let ty = match k.as_str() {
            "texts" => 0,
            "times" => 1,
            "boxes" => 2,
            _ => {
                log!("更新{}信息失败，自定义字段错误", op::ternary!(field == 0 => "客户", "产品"));
                return Err(Response::invalid_value("自定义字段错误"))
            }
        };
        for f in v {
            let state = format!(
                "UPDATE custom_field_data SET value='{}' 
                    WHERE fields={field} AND ty={ty} AND display='{}' AND id='{id}' LIMIT 1",
                f.value, f.display
            );
            // println!("{}", state);
            conn.query_drop(state)?;
        }
    }
    Ok(())
}

pub fn __insert_custom_fields(
    conn: &mut mysql::PooledConn,
    fields: &HashMap<String, Vec<crate::Field>>,
    ty: u8,
    id: &str,
) -> Result<(), crate::Response> {
    let (texts, times, boxes) = unsafe {
        crate::pages::STATIC_CUSTOM_FIELDS.get_fields(ty as _)
    };

    let map: HashMap<&str, Vec<&str>> = [("texts", texts), ("times", times), ("boxes", boxes)]
        .into_iter()
        .collect();
    for (k, v) in &map {
        if let Some(d) = fields.get(*k) {
            if !verify_custom_fields(v, d) {
                log!("录入{}信息失败，原因存在自定义字段不匹配情况", op::ternary!(ty == 0 => "客户", "产品"));
                return Err(crate::Response::dissatisfy("自定义字段存在不匹配情况"));
            }
        } else {
            log!("录入{}信息失败，原因存在自定义字段不匹配情况", op::ternary!(ty == 0 => "客户", "产品"));
            return Err(crate::Response::dissatisfy("自定义字段存在不匹配情况"));
        }
    }
    let mut values = String::new();
    for (k, v) in fields {
        let s = op::some!(get_ty(k); continue);
        for field in v {
            values.push_str(&format!(
                "({ty}, {s}, '{id}', '{}', '{}'),",
                field.display, field.value
            ));
        }
    }
    values.pop();
    if !values.is_empty() {
        conn.query_drop(format!(
            "INSERT INTO custom_field_data (fields, ty, id, display, value) VALUES {values}"
        ))?;
    }

    Ok(())
}

fn get_ty(s: &str) -> Option<i32> {
    match s {
        "texts" => Some(0),
        "times" => Some(1),
        "boxes" => Some(2),
        _ => None,
    }
}
