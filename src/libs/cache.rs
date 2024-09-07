use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;

use crate::{
    database::__get_conn,
    pages::{func::{store::Storehouse, Order}, User},
};

macro_rules! gen_cache {
    ($(($N:ident, $T:ty, $clear:expr)), +) => {
        lazy_static::lazy_static! {
            $(
                pub static ref $N: Arc<DashMap<String, $T>> = {
                    Arc::new(DashMap::new())
                };
            )+
        }
        pub fn clear_cache() {
            $(
                if $clear {
                    $N.clear();
                }
            )+
            STORE_HOUSE_CACHE.clear();
        }
    };
}

gen_cache! {
    (ORDER_CACHE, DashMap<String, Arc<Vec<Order>>>, true),
    (ORDER_CACHE_WITH_ID, Arc<Order>, true),
    (CUSTOMER_CACHE, DashMap<String, Value>, true),
    (PRODUCT_CACHE, Value, true),
    (OPTION_CACHE, DashMap<String, Value>, true),
    (USER_CACHE, Arc<User>, false),
    (TOKEN_CACHE, String, true),
    (KEY_VALUE_CACHE, Vec<Value>, false)
}
use mysql::prelude::Queryable;
lazy_static::lazy_static! {
    pub static ref STORE_HOUSE_CACHE: Arc<DashMap<String, Storehouse>> = {
        let mut conn = __get_conn().expect("连接数据库失败");
        let map: DashMap<String, Storehouse> = 
        conn.query::<Storehouse, &str>("select * from storehouse")
            .expect("连接数据库失败")
            .into_iter()
            .map(|v| (v.id.clone(), v))
            .collect();
        Arc::new(map)
    };
}
