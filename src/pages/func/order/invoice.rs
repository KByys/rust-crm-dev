use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};

use crate::{log, Response};

#[derive(Deserialize, Serialize, FromRow, Default, Debug)]
pub struct Invoice {
    pub required: i32,
    pub deadline: String,
    pub title: String,
    pub number: String,
    pub description: String,
}

impl Invoice {
    pub fn gen_number(
        &mut self,
        conn: &mut PooledConn,
        salesman: &str,
        customer: &str,
    ) -> Result<(), Response> {
        self.number = super::gen_number(conn, 1, format!("INV{}{}", salesman, customer))?;
        Ok(())
    }

    pub fn delete(&self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        conn.exec_drop("delete from invoice where order_id=? limit 1", (id,))
    }
    pub fn insert_or_update(
        &mut self,
        id: &str,
        conn: &mut PooledConn,
        salesman: &str,
        customer: &str,
    ) -> Result<(), Response> {
        log!("{:#?}", self);
        if self.number.is_empty() {
            log!("----");
            self.gen_number(conn, salesman, customer)?;
            conn.exec_drop(
                "insert into  invoice (order_id, number, title, deadline, description)
                    values (:id, :num, :title, :dl, :d)",
                params! {
                    "num" => &self.number,
                    "title" => &self.title,
                    "dl" => &self.deadline,
                    "d" => &self.description,
                    "id" => id
                },
            )?;
        } else {
            conn.exec_drop(
                "update invoice set title=:title, deadline=:deadline, description=:description 
                where order_id = :order_id",
                params! {
                    "title" => &self.title,
                    "deadline" => &self.deadline,
                    "description" => &self.description,
                    "order_id" => id
                },
            )?;
        }
        Ok(())
    }
}
