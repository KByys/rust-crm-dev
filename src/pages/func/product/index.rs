use crate::{
    bearer, commit_or_rollback,
    database::get_db,
    libs::{
        cache::PRODUCT_CACHE,
        dser::{deser_f32, serialize_f32_to_string},
        gen_file_link, gen_id, parse_multipart, FilePart, TimeFormat, TIME,
    },
    log,
    pages::{
        account::get_user,
        func::{
            __insert_custom_fields, __update_custom_fields, customer::index::CustomCustomerData,
            get_custom_fields,
        },
        DROP_DOWN_BOX,
    },
    parse_jwt_macro,
    perm::action::StorehouseGroup,
    response::BodyFile,
    verify_perms, Response, ResponseResult,
};
use axum::{
    extract::{Multipart, Path},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub static DEFAULT: (&str, &[u8]) = ("default_product_cover", include_bytes!("default.png"));
pub fn product_router() -> Router {
    Router::new()
        .route("/product/add", post(add_product))
        .route("/product/add/json", post(add_product_json))
        .route("/product/update", post(update_product))
        .route("/product/update/store/:id", post(update_product_store))
        .route("/product/update/json", post(update_product_json))
        .route("/product/delete/:id", delete(delete_product))
        .route("/product/delete/store/:id", delete(delete_storehouse))
        .route("/product/app/list/data", post(query_product))
        .route("/product/query/:id", get(query_by))
        .route("/product/cover/:cover", get(get_cover))
}
use crate::libs::dser::{deserialize_inventory, deserialize_storehouse};
#[derive(Debug, Serialize, Deserialize, mysql_common::prelude::FromRow)]
struct Inventory {
    #[serde(deserialize_with = "deserialize_storehouse")]
    storehouse: String,
    #[serde(deserialize_with = "deserialize_inventory")]
    amount: i32,
}
#[derive(Default)]
pub struct WrapperInventory {
    inner: Vec<Inventory>,
}

impl std::fmt::Debug for WrapperInventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}
impl From<String> for WrapperInventory {
    fn from(_: String) -> Self {
        Self { inner: Vec::new() }
    }
}

impl mysql::prelude::FromValue for WrapperInventory {
    type Intermediate = String;
}
impl<'de> Deserialize<'de> for WrapperInventory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            inner: Deserialize::deserialize(deserializer)?,
        })
    }
}

impl Serialize for WrapperInventory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Serialize::serialize(&self.inner, serializer)
    }
}

#[derive(Debug, Deserialize, Serialize, mysql_common::prelude::FromRow)]
struct ProductParams {
    #[serde(default)]
    id: String,
    #[serde(skip_deserializing)]
    create_time: String,
    #[serde(default)]
    cover: String,
    /// 编号
    num: String,
    name: String,
    /// 规格
    specification: String,
    /// 型号
    model: String,
    /// 单位
    unit: String,
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    purchase_price: f32,
    product_type: String,
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    price: f32,
    /// 条形码
    barcode: String,
    explanation: String,
    custom_fields: CustomCustomerData,
    #[serde(default)]
    inventory: WrapperInventory,
}

async fn add_product(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn).await?;
    if !verify_perms!(
        &user.role,
        StorehouseGroup::NAME,
        StorehouseGroup::ADD_PRODUCT
    ) {
        log!("系统拒绝 {user} 添加产品的请求，原因是没有添加产品的权限");
        return Err(Response::permission_denied());
    }

    let part = parse_multipart(part).await?;
    let data: ProductParams = serde_json::from_str(&part.json)?;
    let name = data.name.clone();
    log!("{user} 请求添加产品 {} -- 带封面", name);
    let file = op::some!(part.files.first(); ret Err(Response::dissatisfy("缺少封面")));
    commit_or_rollback!(async __insert, &mut conn, data, Some(file), &user.role)?;
    PRODUCT_CACHE.clear();
    log!("{user} 成功添加产品 {} -- 带封面", name);
    Ok(Response::empty())
}

async fn add_product_json(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn).await?;
    if !verify_perms!(
        &user.role,
        StorehouseGroup::NAME,
        StorehouseGroup::ADD_PRODUCT
    ) {
        log!("系统拒绝 {user} 添加产品的请求，原因是没有添加产品的权限");
        return Err(Response::permission_denied());
    }

    let data: ProductParams = serde_json::from_value(value)?;
    let name = data.name.clone();
    log!("{user} 请求添加产品 {} -- 默认封面", name);
    commit_or_rollback!(async __insert, &mut conn, data, None, &user.role)?;
    log!("{user} 成功添加产品 {} -- 默认封面", name);
    PRODUCT_CACHE.clear();
    Ok(Response::empty())
}

async fn __insert(
    conn: &mut PooledConn,
    mut data: ProductParams,
    part: Option<&FilePart>,
    role: &str,
) -> Result<(), Response> {
    let time = TIME::now()?;
    data.id = gen_id(&time, &data.name);
    let pinyin = rust_pinyin::get_pinyin(&data.name);
    let n: Option<i32> = conn.query_first(format!(
        "select num from product_num where name='{}'",
        pinyin
    ))?;

    let n = n.unwrap_or(0) + 1;
    if data.num.is_empty() {
        data.num = format!("NO.{}{:0>7}", pinyin, n)
    }
    conn.query_drop(format!(
        "INSERT INTO product_num (name, num) VALUES ('{pinyin}', {n})
    ON DUPLICATE KEY UPDATE num = {n}"
    ))?;
    data.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    let link = if let Some(part) = part {
        gen_file_link(&time, part.filename())
    } else {
        DEFAULT.0.to_owned()
    };
    conn.exec_drop(
        "INSERT INTO product (id, num, name, 
                specification, cover, model, unit,
                product_type, price, create_time, 
                barcode, explanation, purchase_price) VALUES (
                :id, :num, :name, :specification, :cover, :model, :unit,
                :product_type, :price, :create_time, :barcode, :explanation, :purchase_price
        )",
        params! {
            "id" => &data.id,
            "num" => data.num,
            "name" => data.name,
            "specification" => data.specification,
            "cover" => &link,
            "model" => data.model,
            "unit" => data.unit,
            "product_type" => data.product_type,
            "price" => data.price,
            "explanation" => data.explanation,
            "create_time" => data.create_time,
            "barcode" => data.barcode,
            "purchase_price" => data.purchase_price

        },
    )?;
    first_update_store(conn, &data.id, &data.inventory.inner, role).await?;
    __insert_custom_fields(conn, &data.custom_fields.inner, 1, &data.id)?;
    if let Some(part) = part {
        std::fs::write(format!("resources/product/cover/{link}"), &part.bytes)?;
    }
    Ok(())
}

async fn first_update_store(
    conn: &mut PooledConn,
    id: &str,
    store: &[Inventory],
    role: &str,
) -> Result<(), Response> {
    if !store.is_empty()
        && !verify_perms!(
            role,
            StorehouseGroup::NAME,
            StorehouseGroup::ADJUSTING_PRODUCT_INVENTORY
        )
    {
        return Err(Response::permission_denied());
    }
    unsafe {
        let map = DROP_DOWN_BOX.get("storehouse");
        for s in store {
            if map.contains(&s.storehouse.as_str()) {
                conn.query_drop(format!(
                    "insert into product_store (product, storehouse, amount) 
                        values ('{id}', '{}', {})",
                    s.storehouse, s.amount
                ))?;
            }
        }

        Ok(())
    }
}

async fn update_store(
    conn: &mut PooledConn,
    id: &str,
    store: &[Inventory],
    role: &str,
) -> Result<(), Response> {
    if !verify_perms!(
        role,
        StorehouseGroup::NAME,
        StorehouseGroup::ADJUSTING_PRODUCT_INVENTORY
    ) {
        return Err(Response::permission_denied());
    }
    for s in store {
        conn.query_drop(format!(
            "insert into product_store (product, storehouse, amount) 
                values ('{}', '{}', {}) 
                on duplicate key update amount = {}",
            id, s.storehouse, s.amount, s.amount
        ))?;
    }

    Ok(())
}

async fn update_product_store(
    header: HeaderMap,
    Path(id): Path<String>,
    Json(value): Json<Value>,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{user} 请求更新产品 {} 的库存", id);
    let inventory: Vec<Inventory> = serde_json::from_value(value)?;
    update_store(&mut conn, &id, &inventory, &user.role).await?;

    log!("{user} 成功更新产品 {} 的库存", id);
    PRODUCT_CACHE.clear();
    Ok(Response::empty())
}

async fn update_product(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn).await?;
    if !verify_perms!(
        &user.role,
        StorehouseGroup::NAME,
        StorehouseGroup::UPDATE_PRODUCT
    ) {
        log!("{user} 因权限不足而被系统拒绝更新产品信息 -- 带封面");
        return Err(Response::permission_denied());
    }
    let part = parse_multipart(part).await?;
    let data: ProductParams = serde_json::from_str(&part.json)?;
    log!("{user} 请求更新产品 {} 信息 -- 带封面", data.id);
    let file = op::some!(part.files.first(); ret Err(Response::dissatisfy("缺少封面")));
    commit_or_rollback!(__update, &mut conn, data, Some(file))?;

    log!("{user} 成功更新产品信息 -- 带封面");
    PRODUCT_CACHE.clear();
    Ok(Response::empty())
}

async fn update_product_json(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn).await?;
    if !verify_perms!(
        &user.role,
        StorehouseGroup::NAME,
        StorehouseGroup::UPDATE_PRODUCT,
        None
    ) {
        log!("{user} 因权限不足而被系统拒绝更新产品信息 -- 无封面");
        return Err(Response::permission_denied());
    }
    let data: ProductParams = serde_json::from_value(value)?;
    log!("{user} 请求更新产品 {} 信息 -- 无封面", data.id);
    commit_or_rollback!(__update, &mut conn, data, None)?;
    log!("{user} 成功更新产品信息 -- 无封面");
    PRODUCT_CACHE.clear();
    Ok(Response::empty())
}

fn __update(
    conn: &mut PooledConn,
    data: ProductParams,
    part: Option<&FilePart>,
) -> Result<(), Response> {
    let cover: Option<String> = conn.query_first(format!(
        "SELECT cover FROM product WHERE id = '{}' LIMIT 1",
        data.id
    ))?;
    let cover = op::some!(cover; ret Err(Response::not_exist("code: 180909")));
    let time = TIME::now()?;

    let link = if let Some(f) = part {
        let link = gen_file_link(&time, f.filename());
        link
    } else {
        cover.clone()
    };
    conn.exec_drop(
        format!(
            "UPDATE product 
                SET num=:num, 
                name=:name, 
                specification=:specification,
                cover=:cover, 
                model=:model, 
                unit=:unit,  
                product_type=:product_type, 
                price=:price,
                barcode=:barcode, 
                explanation=:explanation,
                purchase_price=:purchase_price
                WHERE id = '{}' LIMIT 1",
            data.id
        ),
        params! {
            "num" => data.num, "name" => data.name,
            "specification" => data.specification, "cover" => &link,
            "model" => data.model, "unit" => data.unit,
            "product_type" => data.product_type, "price" => data.price,
            "explanation" => &data.explanation,
            "barcode" => data.barcode,
            "purchase_price" => data.purchase_price,
        },
    )?;

    __update_custom_fields(conn, &data.custom_fields.inner, 1, &data.id)?;
    if let Some(f) = part {
        std::fs::write(format!("resources/product/cover/{link}"), &f.bytes)?;
        println!("remove -- {}", cover);
        if !cover.eq(DEFAULT.0) {
            std::fs::remove_file(format!("resources/product/cover/{cover}")).unwrap_or(());
        }
    }
    Ok(())
}
#[derive(Debug, Deserialize)]
struct QueryParams {
    stock: usize,
    ty: String,
    storehouse: String,
}

async fn query_product(Json(value): Json<Value>) -> ResponseResult {
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let param_str = value.to_string();
    if let Some(data) = PRODUCT_CACHE.get(&param_str) {
        log!("查询产品信息，缓存命中");
        return Ok(Response::ok(json!(data.clone())));
    }
    log!("查询产品信息，缓存未命中, 查询中....");
    let data: QueryParams = serde_json::from_value(value)?;
    let ty = op::ternary!(data.ty.is_empty() => "IS NOT NULL".into(); format!("= '{}'", data.ty));
    let stock = match data.stock {
        1 => "> 0",
        2 => "= 0",
        _ => "is not null",
    };

    let store = if data.storehouse.is_empty() {
        "is not null".to_owned()
    } else {
        format!("= '{}'", &data.storehouse)
    };
    let query = if data.stock == 0 && data.storehouse.is_empty() {
        format!(
            "select pr.*, 1 as custom_fields, 1 as inventory from product pr 
            where pr.product_type {ty} order by pr.create_time"
        )
    } else if data.storehouse.eq("null") {
        format!(
            "select pr.*, 1 as custom_fields, 1 as inventory from product pr 
            where pr.product_type {ty} and 
            not exists (select 1 from product_store ps where ps.product = pr.id) order by pr.create_time"
        )
    } else {
        format!(
            "select pr.*, 1 as custom_fields, 1 as inventory from product pr 
            where pr.product_type {ty} and 
            exists (select 1 from product_store ps 
                where ps.product = pr.id and ps.storehouse {store} and ps.amount {stock}) order by pr.create_time"
        )
    };

    let tmp: Vec<ProductParams> = conn.query(query)?;
    let mut products = Vec::new();
    for mut product in tmp {
        product.inventory.inner = conn.query(format!(
            "select storehouse, amount 
                from product_store 
                where product = '{}' and amount {stock} 
                and storehouse {store} order by storehouse",
            product.id
        ))?;
        products.push(product);
    }

    log!("共查询到 {} 条产品信息", products.len());
    let value = json!(products);
    PRODUCT_CACHE.insert(param_str, value.clone());
    Ok(Response::ok(value))
}

async fn query_by(Path(id): Path<String>) -> ResponseResult {
    if let Some(data) = PRODUCT_CACHE.get(&id).map(|v| v.clone()) {
        log!("产品--缓存命中");
        return Ok(Response::ok(data));
    }
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let mut data: Option<ProductParams> = conn.query_first(format!(
        "SELECT *, 1 as custom_fields, 1 as inventory FROM product WHERE id = '{id}' ORDER BY create_time"
    ))?;
    if let Some(d) = &mut data {
        d.inventory.inner = conn.query(format!(
            "select storehouse, amount 
                from product_store 
                where product = '{}' order by storehouse",
            d.id
        ))?;
        d.custom_fields = get_custom_fields(&mut conn, &d.id, 1)?;
    }
    let value = json!(data);
    PRODUCT_CACHE.insert(id, value.clone());
    Ok(Response::ok(value))
}
async fn delete_storehouse(
    header: HeaderMap,
    Path(id): Path<String>,
    Json(value): Json<Vec<String>>,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let user = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&user, &mut conn).await?;
    if !verify_perms!(
        &user.role,
        StorehouseGroup::NAME,
        StorehouseGroup::ADJUSTING_PRODUCT_INVENTORY
    ) {
        return Err(Response::permission_denied());
    }
    conn.exec_batch(
        "delete from product_store where product = ? and storehouse = ?",
        value.iter().map(|v| (&id, v)),
    )?;

    PRODUCT_CACHE.clear();
    Ok(Response::empty())
}
async fn delete_product(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let db = get_db().await?;
    let mut conn = db.lock().await;
    let user = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&user, &mut conn).await?;
    if !verify_perms!(
        &user.role,
        StorehouseGroup::NAME,
        StorehouseGroup::DELETE_PRODUCT
    ) {
        return Err(Response::permission_denied());
    }
    commit_or_rollback!(__delete_product, &mut conn, &id)?;
    PRODUCT_CACHE.clear();
    Ok(Response::empty())
}
fn __delete_product(conn: &mut PooledConn, id: &str) -> Result<(), Response> {
    let cover: Option<String> =
        conn.query_first(format!("select cover from product where id = '{id}'"))?;
    conn.query_drop(format!("DELETE FROM custom_field_data WHERE id = '{id}'"))?;
    conn.query_drop(format!("DELETE FROM product WHERE id = '{id}' LIMIT 1"))?;
    conn.query_drop(format!("DELETE FROM product_store WHERE product = '{id}'"))?;

    if let Some(cover) = cover {
        if !cover.eq(DEFAULT.0) {
            std::fs::remove_file(format!("resources/product/cover/{cover}"))?;
        }
    }
    Ok(())
}

async fn get_cover(Path(cover): Path<String>) -> Result<BodyFile, (StatusCode, String)> {
    BodyFile::new_with_base64_url("resources/product/cover", &cover)
}
