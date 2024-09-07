mod index;
use axum::Router;
pub use index::DEFAULT as DEFAULT_PRODUCT_COVER;
pub fn product_router() -> Router {
    Router::new().merge(index::product_router())
}
