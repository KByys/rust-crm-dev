// #[forbid(unused)]
// mod table;
use std::{
    fmt::Display,
    sync::Arc,
    time::{Duration, SystemTime},
};

use mysql::{prelude::Queryable, Pool, PooledConn, Result};
use tokio::sync::{Mutex, MutexGuard};

pub struct Database;
impl Database {
    pub const SET_FOREIGN_KEY_0: &str = "SET foreign_key_checks = 0";
    pub const SET_FOREIGN_KEY_1: &str = "SET foreign_key_checks = 1";
    /// 主键已存在
    pub const DUPLICATE_KEY_ERROR_CODE: u16 = 1062;
    /// 外键无法匹配
    pub const FOREIGN_KEY_ERROR_CODE: u16 = 1452;
}
#[macro_export]
macro_rules! catch {
    ($result:expr => dup) => {
        match $result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(match err {
                mysql::Error::MySqlError(e) if e.code == 1062 => {
                    $crate::Response::already_exist("重复添加")
                }
                e => $crate::Response::internal_server_error(e),
            }),
        }
    };
}

pub fn catch_some_mysql_error(code: u16, msg: impl Display, err: mysql::Error) -> Response {
    match err {
        mysql::Error::MySqlError(e) if e.code == code => {
            if code == Database::DUPLICATE_KEY_ERROR_CODE {
                Response::already_exist(msg)
            } else {
                Response::not_exist(msg)
            }
        }
        e => Response::internal_server_error(e),
    }
}
/// 成功提交，失败回滚
pub fn c_or_r<F, T>(
    f: F,
    conn: &mut PooledConn,
    param: T,
    start_check: bool,
) -> Result<(), Response>
where
    F: Fn(&mut PooledConn, T) -> Result<(), Response>,
{
    conn.query_drop("BEGIN")?;
    let result = match f(conn, param) {
        Ok(_) => {
            conn.query_drop("COMMIT")?;
            Ok(())
        }
        Err(e) => {
            conn.query_drop("ROLLBACK")?;
            Err(e)
        }
    };
    if start_check {
        conn.query_drop(Database::SET_FOREIGN_KEY_1)?;
    }
    result
}

#[macro_export]
macro_rules! mysql_stmt {
    ($table:expr, $($idt:ident, )+) => {
        {
            let values = vec![$(stringify!($idt).to_string(), )+];
            // let params = mysql::params!{ $( stringify!($idt) => &$param.$idt, )+};
            let values1: String = values.iter().fold(String::new(),|out, v| {
                if out.is_empty() {
                    v.clone()
                } else {
                    format!("{},{}",out, v)
                }
            });

            let values2: String = values.iter().fold(String::new(),|out, v| {
                if out.is_empty() {
                    format!(":{}", v)
                } else {
                    format!("{},:{}",out, v)
                }
            });
            let stmt = format!("insert into {} ({values1}) values ({values2})", $table);
            stmt
        }
    };



}

#[macro_export]
macro_rules! commit_or_rollback {
    (async $fn:expr, $conn:expr, $params:expr) => {{
        use mysql::prelude::Queryable;
        $conn.query_drop("begin")?;
        match $fn($conn, $params).await {
            Ok(ok) => {
                $conn.query_drop("commit")?;
                Ok(ok)
            }
            Err(e) => {
                $conn.query_drop("rollback")?;
                Err(e)
            }
        }
    }};
    (async $fn:expr, $conn:expr, $($args:expr), +) => {{
        use mysql::prelude::Queryable;
        $conn.query_drop("begin")?;
        match $fn($conn, $($args ,)+).await {
            Ok(ok) => {
                $conn.query_drop("commit")?;
                Ok(ok)
            }
            Err(e) => {
                $conn.query_drop("rollback")?;
                Err(e)
            }
        }
    }};
    ($fn:expr, $conn:expr, $params:expr) => {{
        use mysql::prelude::Queryable;
        $conn.query_drop("begin")?;
        match $fn($conn, $params) {
            Ok(ok) => {
                $conn.query_drop("commit")?;
                Ok(ok)
            }
            Err(e) => {
                $conn.query_drop("rollback")?;
                Err(e)
            }
        }
    }};
    ($fn:expr, $conn:expr, $($args:expr), +) => {{
        use mysql::prelude::Queryable;
        $conn.query_drop("begin")?;
        match $fn($conn, $($args ,)+) {
            Ok(ok) => {
                $conn.query_drop("commit")?;
                Ok(ok)
            }
            Err(e) => {
                $conn.query_drop("rollback")?;
                Err(e)
            }
        }
    }};
}
pub type DB<'err> = MutexGuard<'err, PooledConn>;
lazy_static::lazy_static! {
    pub static ref LAST_LEFT_TIME: Arc<Mutex<Duration>> = {
        Arc::new(Mutex::new(Duration::from_secs(1)))
    };
}
/// 闲置6分钟，数据库连接失效，重新连接
pub async fn get_db() -> Result<Arc<Mutex<PooledConn>>, Response> {
    let mut last_time = LAST_LEFT_TIME.lock().await;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let ten = last_time
        .checked_add(Duration::from_secs(600))
        .unwrap_or_default();
    *last_time = now;
    if ten < now {
        let pool = __get_conn()?;
        log!("更新数据库连接");
        let mut db = DBC.lock().await;
        *db = pool;
    }
    Ok(Arc::clone(&DBC))
}

/// 连接数据库
pub fn __get_conn() -> Result<PooledConn> {
    Pool::new(CONFIG.mysql.uri().as_str())?.get_conn()
}
lazy_static::lazy_static! {
    /// 全局数据库连接池
    pub static ref DBC: Arc<Mutex<PooledConn>> = {
        Arc::new(
            Mutex::new(
                Pool::new( CONFIG.mysql.uri().as_str())
                .expect("数据库连接失败")
                .get_conn()
                .expect("数据库连接失败")
            )
        )
    };
}

use crate::{log, Response, CONFIG};

pub fn create_table() -> Result<()> {
    let mut conn = __get_conn()?;

    let sql = include_str!("./table.sql");
    for s in sql.split(';').filter(|s| !s.trim().is_empty()) {
        conn.query_drop(s)?
    }
    Ok(())
}
