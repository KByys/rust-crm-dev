use std::collections::HashMap;

pub(super) fn groups() -> HashMap<&'static str, Vec<&'static str>> {
    [
        (RoleGroup::NAME, ROLE.to_vec()),
        (AccountGroup::NAME, ACCOUNT.to_vec()),
        (CustomerGroup::NAME, CUSTOMER.to_vec()),
        (StorehouseGroup::NAME, STOREHOUSE.to_vec()),
        (FinanceGroup::NAME, FINANCE.to_vec()),
        (PurchaseGroup::NAME, PURCHASE.to_vec()),
        (OtherGroup::NAME, OTHER_GROUP.to_vec()),
    ]
    .into_iter()
    .collect()
}

#[forbid(unused)]
pub static ROLE: [&str; 4] = [
    RoleGroup::CREATE,
    RoleGroup::UPDATE,
    RoleGroup::DELETE,
    RoleGroup::CHANGE_ROLE,
];

pub struct RoleGroup;

impl RoleGroup {
    pub const NAME: &str = "role";
    pub const CREATE: &str = "create";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    /// 角色调动
    pub const CHANGE_ROLE: &str = "change_role";
}

pub static ACCOUNT: [&str; 2] = [AccountGroup::CREATE, AccountGroup::DELETE];

pub struct AccountGroup;
impl AccountGroup {
    pub const NAME: &str = "account";
    pub const CREATE: &str = "create";
    pub const DELETE: &str = "delete";
}

#[forbid(unused)]
pub static CUSTOMER: [&str; 10] = [
    CustomerGroup::ACTIVATION,
    CustomerGroup::QUERY,
    CustomerGroup::ENTER_CUSTOMER_DATA,
    CustomerGroup::UPDATE_CUSTOMER_DATA,
    CustomerGroup::DELETE_CUSTOMER_DATA,
    CustomerGroup::QUERY_PUB_SEA,
    CustomerGroup::TRANSFER_CUSTOMER,
    CustomerGroup::EXPORT_DATA,
    CustomerGroup::RELEASE_CUSTOMER,
    CustomerGroup::ADD_APPOINT,
];

pub struct CustomerGroup;
impl CustomerGroup {
    pub const NAME: &str = "customer";
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
    pub const ENTER_CUSTOMER_DATA: &str = "enter_customer_data";
    pub const UPDATE_CUSTOMER_DATA: &str = "update_customer_data";
    pub const DELETE_CUSTOMER_DATA: &str = "delete_customer_data";
    /// 查看任意部门的公海客户信息
    pub const QUERY_PUB_SEA: &str = "query_pub_sea";
    pub const TRANSFER_CUSTOMER: &str = "transfer_customer";
    pub const EXPORT_DATA: &str = "export_data";
    pub const RELEASE_CUSTOMER: &str = "release_customer";
    pub const ADD_APPOINT: &str = "add_appoint";
}
#[forbid(unused)]
pub static STOREHOUSE: [&str; 8] = [
    StorehouseGroup::ACTIVATION,
    StorehouseGroup::ADD_PRODUCT,
    StorehouseGroup::UPDATE_PRODUCT,
    StorehouseGroup::DELETE_PRODUCT,
    StorehouseGroup::ADJUSTING_PRODUCT_INVENTORY,
    StorehouseGroup::ADD_STOREHOUSE,
    StorehouseGroup::DELETE_STOREHOUSE,
    StorehouseGroup::UPDATE_STOREHOUSE,
];

pub struct StorehouseGroup;

impl StorehouseGroup {
    pub const NAME: &str = "storehouse";
    pub const ACTIVATION: &str = "activation";
    pub const ADD_PRODUCT: &str = "add_product";
    pub const UPDATE_PRODUCT: &str = "update_product";
    pub const DELETE_PRODUCT: &str = "delete_product";
    pub const ADJUSTING_PRODUCT_INVENTORY: &str = "adjusting_product_inventory";
    pub const ADD_STOREHOUSE: &str = "add_storehouse";
    pub const DELETE_STOREHOUSE: &str = "delete_storehouse";
    pub const UPDATE_STOREHOUSE: &str = "update_storehouse";
    // TODO:
}

#[forbid(unused)]
pub static PURCHASE: [&str; 2] = [PurchaseGroup::ACTIVATION, PurchaseGroup::QUERY];
pub struct PurchaseGroup;

impl PurchaseGroup {
    pub const NAME: &str = "purchase";
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
}

#[forbid(unused)]
pub static FINANCE: [&str; 2] = [FinanceGroup::ACTIVATION, FinanceGroup::QUERY];
pub struct FinanceGroup;

impl FinanceGroup {
    pub const NAME: &str = "finance";
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
}

#[forbid(unused)]
pub static OTHER_GROUP: [&str; 6] = [
    OtherGroup::QUERY_SIGN_IN,
    OtherGroup::CUSTOM_FIELD,
    OtherGroup::DROP_DOWN_BOX,
    OtherGroup::SEA_RULE,
    OtherGroup::COMPANY_STAFF_DATA,
    OtherGroup::QUERY_ORDER,
];
pub struct OtherGroup;

impl OtherGroup {
    pub const NAME: &str = "other";
    pub const QUERY_SIGN_IN: &str = "query_sign_in";
    pub const CUSTOM_FIELD: &str = "custom_field";
    pub const DROP_DOWN_BOX: &str = "drop_down_box";
    pub const SEA_RULE: &str = "sea_rule";
    pub const COMPANY_STAFF_DATA: &str = "company_staff_data";
    pub const QUERY_ORDER: &str = "query_order";
}
