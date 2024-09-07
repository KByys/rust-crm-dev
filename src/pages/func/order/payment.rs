use crate::libs::{
    dser::{deser_f32, serialize_f32_to_string},
    TIME,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, FromRow, Serialize, PartialEq, Debug)]
pub struct Instalment {
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub interest: f32,
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub original_amount: f32,
    #[serde(skip_deserializing)]
    pub date: Option<String>,
    #[serde(default)]
    pub inv_index: i32,
    #[serde(skip_deserializing)]
    pub finish: i32,
}
impl Instalment {
    pub fn computed_instalment(instalment: &[Instalment]) -> f32 {
        instalment
            .iter()
            .fold(0.0f32, |output, inv| output + inv.original_amount)
    }

    pub fn query(conn: &mut PooledConn, id: &str) -> mysql::Result<Vec<Instalment>> {
        conn.exec(
            "select * from order_instalment where order_id = ? order by inv_index",
            (id,),
        )
    }
    pub fn insert(
        conn: &mut PooledConn,
        id: &str,
        instalment: &[Instalment],
        del: bool,
    ) -> mysql::Result<()> {
        let time = TIME::now().unwrap_or_default();
        if del {
            conn.exec_drop("delete from order_instalment where order_id = ?", (&id,))?;
        }
        for (i, v) in instalment.iter().enumerate() {
            conn.exec_drop(
                "insert into order_instalment 
                (order_id, interest, original_amount, date, finish, inv_index) 
                values 
                (:order_id, :interest, :original_amount, :date, :finish, :inv_index) 
                ",
                params! {
                        "order_id" => id,
                        "interest" => v.interest,
                        "original_amount" => v.original_amount,
                        "finish" => v.finish,
                        "date" => if v.finish == 1 {
                           time.format(crate::libs::TimeFormat::YYYYMMDD_HHMMSS)
                        } else {
                            "".to_string()
                        },
                        "inv_index" => i + 1
                },
            )?;
        }
        Ok(())
    }
}
