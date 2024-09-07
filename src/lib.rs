pub mod common;

pub mod perm;
pub mod response;
pub mod token;

#[allow(elided_lifetimes_in_associated_constant)]
pub mod database;
pub mod libs;
pub mod pages;

use std::fmt::Arguments;

use libs::time::TIME;
pub use libs::{base64_decode, base64_encode};
use mysql_common::prelude::FromRow;
pub use response::Response;
use serde_json::json;
pub type ResponseResult = Result<Response, Response>;

#[macro_export]
macro_rules! get_cache {
    ($map:expr, $arg1:expr, $arg2:expr) => {
        match $map.get($arg1) {
            Some(rw) => rw.get($arg2).map(|c| c.clone()),
            _ => None,
        }
    };
}


#[macro_export]
macro_rules! log {
    ($($args:tt)+) => {
        $crate::log(format_args!($($args)+))
    };
}

pub fn log(args: Arguments) {
    println!("{}", "*".repeat(10));
    let time = TIME::now().unwrap_or_default();
    println!("{} ---", time.format(libs::TimeFormat::YYYYMMDD_HHMMSS));
    println!("    {}", args);
}

pub fn get_value<'a>(value: &'a serde_json::Value, index: &str) -> Option<&'a str> {
    value.get(index).and_then(|s| s.as_str())
}

#[derive(serde::Serialize, FromRow, Debug, serde::Deserialize, Clone)]
pub struct Field {
    pub display: String,
    pub value: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ID {
    pub id: String,
    #[serde(default)]
    pub public: bool,
}

pub static mut MYSQL_URI: String = String::new();
pub static mut SEA_MAX_DAY: u64 = 3;
pub static mut SEA_MIN_DAY: u64 = 3;
/// 提成
pub static mut COMMISSION: i32 = -1;
pub fn get_commission() -> std::io::Result<i32> {
    unsafe {
        if COMMISSION == -1 {
            match std::fs::read_to_string("data/commission") {
                Ok(v) => {
                    COMMISSION = v.parse().unwrap_or(0);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    COMMISSION = 10;
                    set_commission(10)?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(COMMISSION)
    }
}
pub fn set_commission(value: i32) -> std::io::Result<()> {
    unsafe {
        COMMISSION = value;
    }
    std::fs::write("data/commission", value.to_string().as_bytes())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct MYSQL {
    user: String,
    password: String,
    host: String,
    port: u32,
    database: String,
}
impl MYSQL {
    pub fn uri(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}
lazy_static::lazy_static! {
    pub static ref CONFIG: Config = {
        Config::read()
    };
}
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Config {
    port: u16,
    mysql: MYSQL,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 80,
            mysql: MYSQL {
                user: "root".to_owned(),
                password: "密码".to_owned(),
                host: "localhost".to_owned(),
                port: 3306,
                database: "crm".to_owned(),
            },
        }
    }
}
impl Config {
    pub fn read() -> Self {
        let config = match std::fs::read_to_string("config/config.json") {
            Ok(value) => value,
            Err(e) => {
                if let std::io::ErrorKind::NotFound = e.kind() {
                    std::fs::write("config/config.json", json!(Config::default()).to_string())
                        .expect("创建config/config.json文件失败，请手动创建");
                    panic!(
                        "该设置文件'config/config.json'不存在，已在当前目录自动创建，请根据实际情况修改里面的配置!",
                    )
                } else {
                    panic!("读取设置文件时发送错误，具体信息为：{:#?}", e)
                }
            }
        };
        match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(e) => panic!("config/config.json格式错误，具体错误信息为: {:#?}", e),
        }
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn mysql_addr(&self) -> String {
        self.mysql.uri()
    }
}
pub fn read_data() {
    use std::fs::read_to_string;
    if let Ok(sea) = read_to_string("data/sea") {
        let v: Vec<&str> = sea.splitn(2, '-').collect();
        let max_day = v.first().and_then(|s| s.parse().ok()).unwrap_or(3);
        let min_day = v.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
        unsafe {
            SEA_MAX_DAY = max_day;
            SEA_MIN_DAY = min_day;
        }
    }
}
