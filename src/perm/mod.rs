pub mod inner;
pub mod roles;
use std::collections::HashMap;

use crate::{
    bearer,
    database::get_db,
    parse_jwt_macro,
    perm::{action::groups, roles::ROLE_TABLES},
    Response, ResponseResult,
};
use axum::{http::HeaderMap, routing::post, Router};
use mysql::{prelude::Queryable, PooledConn};
use serde_json::json;
use tokio::sync::Mutex;



pub type PermissionGroupMap = HashMap<String, HashMap<String, Vec<String>>>;
pub type PermissionMap = HashMap<String, Vec<String>>;
#[allow(elided_lifetimes_in_associated_constant)]
#[forbid(unused)]
pub(crate) mod action;
lazy_static::lazy_static! {
    pub static ref PERMISSION_GROUPS: HashMap<&'static str, Vec<&'static str>> = {
        groups()
    };
    pub static ref ROLES_GROUP_MAP: Mutex<HashMap<String, PermissionGroupMap>> = {
        let map = if let Ok(bytes) = std::fs::read("data/perm") {
            serde_json::from_slice(&bytes).expect("权限文件结构遭到破坏，请联系开发人员进行修复")
        } else {
            let mut map = HashMap::new();
            map.insert("salesman".to_owned(), role_salesman());
            map.insert("admin".to_owned(), unsafe { role_adm() });
            map.insert("manager".to_owned(), unsafe { role_manager() });

            std::fs::write("data/perm", json!(map.clone()).to_string().as_bytes()).expect("写入权限文件失败");
            map
        };
        // let mut dash_map = DashMap::new();
        // for (key, value) in map {
        //     dash_map.insert(key, value);
        // }
        // dash_map
        Mutex::new(map)
    };
}
pub async fn update_role_map(role: &str, perms: PermissionGroupMap) -> Result<(), Response> {
    use std::fs::write;
    let mut map = ROLES_GROUP_MAP.lock().await;
    if let Some(v) = map.get_mut(role) {
        *v = perms;
        write("data/perm", json!(map.clone()).to_string().as_bytes())?;
    }
    Ok(())
}
fn role_salesman() -> PermissionGroupMap {
    use action::*;
    [(
        CustomerGroup::NAME,
        [
            (CustomerGroup::ACTIVATION, vec![]),
            (CustomerGroup::ENTER_CUSTOMER_DATA, vec![]),
            (CustomerGroup::UPDATE_CUSTOMER_DATA, vec![]),
            (CustomerGroup::ACTIVATION, vec![]),
        ]
        .into_iter()
        .map(|(name, key)| (name.to_owned(), key))
        .collect(),
    )]
    .into_iter()
    .map(|(name, key)| (name.to_owned(), key))
    .collect()
}

unsafe fn role_manager() -> PermissionGroupMap {
    use action::*;
    [
        (
            RoleGroup::NAME,
            [
                (RoleGroup::CREATE, vec![]),
                (RoleGroup::UPDATE, vec!["all".to_owned()]),
                (RoleGroup::DELETE, vec!["all".to_owned()]),
                (RoleGroup::CHANGE_ROLE, vec!["all".to_owned()]),
            ]
            .into_iter()
            .map(|(name, key)| (name.to_owned(), key))
            .collect(),
        ),
        (
            CustomerGroup::NAME,
            CUSTOMER
                .iter()
                .map(|x| {
                    if matches!(
                        *x,
                        CustomerGroup::EXPORT_DATA
                            | CustomerGroup::QUERY_PUB_SEA
                            | CustomerGroup::QUERY
                    ) {
                        (x.to_string(), vec!["all".to_owned()])
                    } else {
                        (x.to_string(), vec![])
                    }
                })
                .collect(),
        ),
        (AccountGroup::NAME, {
            [
                (
                    AccountGroup::CREATE,
                    vec!["all_department".to_owned(), "all".to_owned()],
                ),
                (
                    AccountGroup::DELETE,
                    vec!["all_department".to_owned(), "all".to_owned()],
                ),
            ]
            .into_iter()
            .map(|(name, key)| (name.to_owned(), key))
            .collect()
        }),
        (StorehouseGroup::NAME, {
            // [
            //     (StorehouseGroup::ACTIVATION, vec![]),
            //     (StorehouseGroup::ADD_PRODUCT, vec![]),
            //     (StorehouseGroup::ADJUSTING_PRODUCT_INVENTORY, vec![]),
            //     (StorehouseGroup::DELETE_PRODUCT, vec![]),
            //     (StorehouseGroup::UPDATE_PRODUCT, vec![]),
            // ]
            STOREHOUSE
            .iter()
            .map(|v| (v.to_string(), Vec::new()))
            .collect()
        }),
        (OtherGroup::NAME, {
            [
                (OtherGroup::QUERY_SIGN_IN, vec!["all".to_owned()]),
                (OtherGroup::SEA_RULE, vec!["all".to_owned()]),
                (OtherGroup::COMPANY_STAFF_DATA, vec!["all".to_owned()]),
                (OtherGroup::QUERY_ORDER, vec!["all".to_owned()]),
                (OtherGroup::CUSTOM_FIELD, Vec::new()),
                (OtherGroup::DROP_DOWN_BOX, Vec::new()),
            ]
            .into_iter()
            .map(|(name, key)| (name.to_owned(), key))
            .collect()
        }),
    ]
    .into_iter()
    .map(|(perm, key)| (perm.to_owned(), key))
    .collect()
}

unsafe fn role_adm() -> PermissionGroupMap {
    use action::*;
    [
        (
            CustomerGroup::NAME,
            CUSTOMER
                .iter()
                .map(|x| {
                    if *x == CustomerGroup::EXPORT_DATA {
                        (x.to_string(), vec!["department".to_owned()])
                    } else {
                        (x.to_string(), vec![])
                    }
                })
                .collect(),
        ),
        (AccountGroup::NAME, {
            [
                (
                    AccountGroup::CREATE,
                    vec![ROLE_TABLES.get_name_uncheck("salesman")],
                ),
                (
                    AccountGroup::DELETE,
                    vec![ROLE_TABLES.get_name_uncheck("salesman")],
                ),
            ]
            .into_iter()
            .map(|(name, key)| (name.to_owned(), key))
            .collect()
        }),
        (StorehouseGroup::NAME, {
            [
                (StorehouseGroup::ACTIVATION, vec![]),
                (StorehouseGroup::ADD_PRODUCT, vec![]),
                (StorehouseGroup::ADJUSTING_PRODUCT_INVENTORY, vec![]),
                (StorehouseGroup::UPDATE_PRODUCT, vec![]),
            ]
            .into_iter()
            .map(|(name, key)| (name.to_owned(), key))
            .collect()
        }),
        (OtherGroup::NAME, {
            [
                (OtherGroup::QUERY_SIGN_IN, vec![]),
                (OtherGroup::SEA_RULE, vec![]),
                (OtherGroup::QUERY_ORDER, vec![]),
            ]
            .into_iter()
            .map(|(name, key)| (name.to_owned(), key))
            .collect()
        }),
    ]
    .into_iter()
    .map(|(perm, key)| (perm.to_owned(), key))
    .collect()
}

pub fn perm_router() -> Router {
    Router::new().route("/get/perm", post(get_perm))
}

pub async fn verify_permissions(
    role: &str,
    perm: &str,
    action: &str,
    data: Option<&[&str]>,
) -> bool {
    if role.eq("root") {
        return true;
    }
    let role_perm_maps = ROLES_GROUP_MAP.lock().await;
    let role_perms = op::some!(role_perm_maps.get(role); ret false);

    op::some!(role_perms.get(perm); ret false)
        .get(action)
        .map_or(false, |v| {
            data.map_or(true, |d| d.iter().all(|k| v.contains(&k.to_string())))
        })
}
#[macro_export]
macro_rules! verify_perms {
    ($role:expr, $perm:expr, $(($action:expr, $data:expr)), +) => {
        if !$role.eq("root") {
            let role_perm_maps = $crate::perm::ROLES_GROUP_MAP.lock().await;
            $(
                match op::catch!(role_perm_maps.get($role)?.get($perm)?.get($action)) {
                    Some(v) => {
                            if let Some::<&[&str]>(d) = $data {
                                d.iter().all(|k| v.contains(&k.to_string()))
                            } else {
                                true
                            }
                    }
                    _ => {if let Some::<&[&str]>(_) = $data { } false}
                }
            ,)+
        } else {
            ($({
                    if let Some<&[&str]>(_) = $data {} true
            },)+)
        }
    };

    ($role:expr, $perm:expr, $(($action:expr, $($data:expr), +)), +) => {
        if !$role.eq("root") {
            let role_perm_maps = $crate::perm::ROLES_GROUP_MAP.lock().await;
            $(
                match op::catch!(role_perm_maps.get($role)?.get($perm)?.get($action)) {
                    Some(v) => {
                        vec![$(
                            if let Some::<&[&str]>(d) = $data {
                                d.iter().all(|k| v.contains(&k.to_string()))
                            } else {
                                true
                            },
                        )+]
                    }
                    _ => vec![$({if let Some::<&[&str]>(_) = $data { } false}, )+]
                }

            ,)+
        } else {
            ($({
                    let _ = $action;
                    vec![$({if let Some<&[&str]>(_) = $data {} true},)+]
            },)+)
        }
    };



    ($role:expr, $perm:expr, $action:expr) => {
        $crate::verify_perms!($role, $perm, $action, None)
    };

    // 普通验证
    ($role:expr, $perm:expr, $action:expr, $data:expr) => {
        $role.eq("root") || {
            let role_perm_maps = $crate::perm::ROLES_GROUP_MAP.lock().await;
            op::catch!{ role_perm_maps.get($role)?.get($perm)?.get($action)}
            .map_or(false, |v| {
                $data.map_or(true, |d: &[&str]| d.iter().all(|k| v.contains(&k.to_string())))
            })

        }
    };



    // 同时验证多个
    ($role:expr, $perm:expr, $action:expr, $($data:expr), +) => {
        if !$role.eq("root") {
            let role_perm_maps = $crate::perm::ROLES_GROUP_MAP.lock().await;
                match op::catch!(role_perm_maps.get($role)?.get($perm)?.get($action)) {
                    Some(v) => {
                        ($(
                            if let Some::<&[&str]>(d) = $data {
                                d.iter().all(|k| v.contains(&k.to_string()))
                            } else {
                                true
                            },
                        )+)
                    }
                    _ => ($({if let Some::<&[&str]>(_) = $data { } false}, )+)
                }
        } else {
            ($( { if let Some::<&[&str]>(_) = $data{} true }, )+)
        }
    };

}

async fn get_perm(headers: HeaderMap) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let role = get_role(&id, &mut conn)?;
    let perm_map = ROLES_GROUP_MAP.lock().await;
    if let Some(perms) = perm_map.get(&role) {
        Ok(Response::ok(json!(perms[&role])))
    } else {
        Ok(Response::ok(json!(PermissionGroupMap::new())))
    }
}
#[inline(always)]
pub fn get_role(id: &str, conn: &mut PooledConn) -> Result<String, Response> {
    let role = op::some!(conn.query_first(format!("SELECT role FROM user WHERE id = '{id}'"))?; ret Err(Response::not_exist("用户不存在")));
    Ok(role)
}
