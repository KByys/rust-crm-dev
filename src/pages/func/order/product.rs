use crate::libs::dser::serialize_f32_to_string;
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::data::Order;

pub fn deserialize_f32_max_1<'de, D>(de: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(de)?;
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(f as f32)
            } else if let Some(i) = n.as_i64() {
                Ok(i as f32)
            } else if let Some(u) = n.as_u64() {
                Ok(u as f32)
            } else {
                Err(serde::de::Error::custom("discount不是浮点数格式"))
            }
        }
        Value::String(value) => {
            if let Ok(f) = value.parse::<f32>() {
                op::ternary!(f <= 1.0 => Ok(f), Err(serde::de::Error::custom("discount最大值为1")))
            } else {
                Err(serde::de::Error::custom("discount不是浮点数格式"))
            }
        }
        _ => Err(serde::de::Error::custom("discount不是浮点数格式")),
    }
}
#[derive(Deserialize, Serialize, Debug, FromRow)]
pub struct Product {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "deserialize_f32_max_1")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub discount: f32,
    #[serde(deserialize_with = "crate::libs::dser::deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub price: f32,
    pub model: String,
    #[serde(skip_deserializing)]
    pub cover: String,
    pub amount: usize,
    #[serde(skip_deserializing)]
    pub unit: String,
}

// pub fn f32_is_eq(v1: f32, v2: f32) -> bool {
//     (-0.001..0.001f32).contains(&(v1 - v2))
// }
pub fn computed_products_sum(products: &[Product]) -> f32 {
    let mut sum = 0.0f32;
    for product in products {
        let prsd = product.price_sum_with_discount();
        sum += prsd;
    }
    sum
}
// pub fn to_fixed(size: usize, value: f32) -> f32 {
//     let value = value.to_string();
//     let sp = value.splitn(2, '.').collect::<Vec<_>>();
//     let digit = if let Some(dig) = sp.get(1) {
//         if dig.len() >= size {
//             if dig.len() > size {
//                 let flag = match dig.as_bytes()[size + 1] {
//                     b'5'..=b'9' => true,
//                     _ => false
//                 };
//                 let value: i32 = (&dig[..size]).parse().unwrap();
//                 if value <= 99 {

//                 }
//             } else {
//                 dig.to_string()
//             }
//         } else {
//             dig.chars().next().expect("Invalid").to_string()
//         }
//     } else {
//         "0".to_string()
//     }
//     format!("{}.", sp[0]).parse().unwrap_or(0.0)
// }

impl Product {
    pub fn price_sum(&self) -> f32 {
        self.amount as f32 * self.price
    }

    pub fn price_sum_with_discount(&self) -> f32 {
        self.price_sum() * (1.0 - self.discount)
    }
    pub fn insert(
        products: &[Product],
        id: &str,
        conn: &mut PooledConn,
        del: bool,
    ) -> mysql::Result<()> {
        if del {
            conn.exec_drop("delete from order_product where order_id = ?", (id,))?;
        }
        conn.exec_batch(
            "insert into order_product (order_id, id, price, discount, amount) values (:order_id, :id, :price, :discount, :amount)",
            products.iter().map(|product| {
                params! {
                    "order_id" => id,
                    "id" => &product.id,
                    "price" => product.price,
                    "discount" => product.discount,
                    "amount" => product.amount
                }
            }),
        )
    }
    pub fn query(order: &mut Order, conn: &mut PooledConn) -> mysql::Result<()> {
        order.product = conn.exec(
            "select op.*, p.model, p.unit, p.cover, p.name
                from order_product op 
                left join product p on p.id=op.id 
                where op.order_id=? 
                order by p.name",
            (&order.id, )
        )?;
        Ok(())
    }
}
