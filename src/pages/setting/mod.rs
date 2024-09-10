mod custom;
pub mod option;
use axum::{
    routing::{delete, get, post},
    Router,
};
pub use custom::{CustomFields, Field, STATIC_CUSTOM_BOX_OPTIONS, STATIC_CUSTOM_FIELDS};
pub fn setting_router() -> Router {
    Router::new()
        .route("/box/option/infos", get(option::query_option_value))
        .route("/box/option/infos/:ty", get(option::query_specific_info))
        .route("/box/option/insert", post(option::insert_options))
        .route("/box/option/update", post(option::update_option_value))
        .route("/box/option/delete", delete(option::delete_option_value))
        .route("/customize/info/insert", post(custom::insert_custom_field))
        .route(
            "/customize/info/box/insert",
            post(custom::insert_box_option),
        )
        .route("/customize/info/update", post(custom::update_custom_field))
        .route(
            "/customize/info/box/update",
            post(custom::update_box_option),
        )
        .route(
            "/customize/info/delete",
            delete(custom::delete_custom_field),
        )
        .route(
            "/customize/info/box/delete",
            delete(custom::delete_box_option),
        )
        .route("/customize/infos", get(custom::get_custom_info))
        .route("/customize/info/get/:ty", get(custom::get_custom_info_with))
        .route("/custom/fields/:ty/:id", get(custom::query_custom_fields))
        .route("/custom/fields/box/:ty/:display", get(custom::query_box))
}
