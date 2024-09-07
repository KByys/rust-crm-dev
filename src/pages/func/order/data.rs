use std::collections::HashMap;

use mysql::{
    params,
    prelude::{FromRow, Queryable},
    FromRowError, PooledConn,
};
use serde::{Deserialize, Serialize};

use crate::{common::Person, mysql_stmt, Response};

use super::{
    customer::Customer, invoice::Invoice, payment::Instalment, product::Product, ship::Ship,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct Order {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub create_time: String,
    pub number: String,
    pub status: i32,
    pub ty: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub transaction_date: Option<String>,
    pub receipt_account: String,
    pub salesman: Person,
    pub payment_method: String,
    pub instalment: Vec<Instalment>,
    pub product: Vec<Product>,
    pub customer: Customer,
    pub invoice: Invoice,
    pub ship: Ship,
    pub comment: String,
}
impl Order {
    pub fn gen_number(&mut self, conn: &mut PooledConn) -> Result<(), Response> {
        if self.number.is_empty() {
            let name = format!(
                "{}{}{}",
                self.salesman.name, self.product[0].name, self.customer.name
            );
            self.number = super::gen_number(conn, 0, &name)?;
        }
        Ok(())
    }
    pub fn del(&self, conn: &mut PooledConn) -> Result<(), Response> {
        conn.exec_drop("delete from order_product where order_id = ?", (&self.id,))?;
        conn.exec_drop(
            "delete from order_instalment where order_id = ? ",
            (&self.id,),
        )?;
        conn.exec_drop("delete from invoice where order_id = ? ", (&self.id,))?;
        conn.exec_drop("delete from order_data where id = ? limit 1", (&self.id,))?;
        if let Some(f) = &self.file {
            let _ = std::fs::remove_file(format!("resources/order/{}", f));
        }
        Ok(())
    }

    pub fn query_other(&mut self, conn: &mut PooledConn) -> Result<(), Response> {
        self.query_insalment(conn)?;
        self.query_invoice(conn)?;
        self.query_product(conn)?;
        Ok(())
    }
    pub fn query_insalment(&mut self, conn: &mut PooledConn) -> mysql::Result<()> {
        self.instalment = Instalment::query(conn, &self.id)?;
        Ok(())
    }

    pub fn query_product(&mut self, conn: &mut PooledConn) -> mysql::Result<()> {
        Product::query(self, conn)
    }
    pub fn query_invoice(&mut self, conn: &mut PooledConn) -> Result<(), Response> {
        if self.invoice.required == 1 {
            let Some(invoice) = conn.exec_first(
                "select *, 1 as required from invoice where order_id = ? limit 1 ",
                (&self.id,),
            )?
            else {
                return Err(Response::not_exist("发票不存在"));
            };
            self.invoice = invoice
        }
        Ok(())
    }
    pub fn insert(&mut self, conn: &mut PooledConn) -> Result<(), Response> {
        let order = self;
        let stmt = mysql_stmt!(
            "order_data",
            id,
            number,
            create_time,
            status,
            ty,
            receipt_account,
            salesman,
            payment_method,
            transaction_date,
            customer,
            address,
            purchase_unit,
            invoice_required,
            shipped,
            shipped_date,
            shipped_storehouse,
            comment,
        );
        conn.exec_drop(
            stmt,
            params! {
                "id" => &order.id,
                "number" =>  &order.number,
                "create_time" => &order.create_time,
                "status" => &order.status,
                "ty" => &order.ty,
                "receipt_account" => &order.receipt_account,
                "salesman" => &order.salesman.id,
                "payment_method" => &order.payment_method,
                "customer" => &order.customer.id,
                "transaction_date" => &order.transaction_date,
                "address" => &order.customer.address,
                "purchase_unit" => &order.customer.purchase_unit,
                "invoice_required" => &order.invoice.required,
                "shipped" => &order.ship.shipped,
                "shipped_date" => &order.ship.date,
                "shipped_storehouse" => &order.ship.storehouse,
                "comment" => &order.comment
            },
        )?;
        Product::insert(&order.product, &order.id, conn, false)?;
        if order.status > 0 {
            for inv in &mut order.instalment {
                inv.finish = if order.status == 2 { 1 } else { 0 };
            }
            Instalment::insert(conn, &order.id, &order.instalment, false)?;
        }
        if order.invoice.required == 1 {
            order.invoice.insert_or_update(
                &order.id,
                conn,
                &order.salesman.name,
                &order.customer.name,
            )?;
        }
        Ok(())
    }
}
macro_rules! get {
    ($map:expr, $name:expr) => {{
        mysql::prelude::FromValue::from_value($map.get($name)?.clone())
    }};
}

impl FromRow for Order {
    fn from_row_opt(row: mysql::Row) -> Result<Self, mysql::FromRowError>
    where
        Self: Sized,
    {
        let columns = row.columns();
        let _row = row.clone();
        let values = row.unwrap();
        let map: HashMap<String, _> = values
            .into_iter()
            .enumerate()
            .map(|(i, item)| (columns[i].name_str().to_string(), item))
            .collect();
        let result: Option<Order> = op::catch!(Some(Self {
            id: get!(map, "id"),
            create_time: get!(map, "create_time"),
            number: get!(map, "number"),
            status: get!(map, "status"),
            ty: get!(map, "ty"),
            file: get!(map, "file"),
            transaction_date: get!(map, "transaction_date"),
            receipt_account: get!(map, "receipt_account"),
            salesman: Person {
                name: get!(map, "salesman_name"),
                id: get!(map, "salesman")
            },
            payment_method: get!(map, "payment_method"),
            instalment: Vec::new(),
            product: Vec::new(),
            customer: Customer {
                id: get!(map, "customer"),
                address: get!(map, "address"),
                name: get!(map, "customer_name"),
                company: get!(map, "company"),
                purchase_unit: get!(map, "purchase_unit")
            },
            invoice: Invoice {
                required: get!(map, "invoice_required"),
                ..Default::default()
            },
            ship: Ship {
                shipped: get!(map, "shipped"),
                date: get!(map, "shipped_date"),
                storehouse: get!(map, "shipped_storehouse")
            },
            comment: get!(map, "comment"),
        }));
        if let Some(order) = result {
            Ok(order)
        } else {
            Err(FromRowError(_row))
        }
    }
}
