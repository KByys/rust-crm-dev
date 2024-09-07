use axum::Router;

mod address_book;

pub fn message_router() -> Router {
    Router::new()
}
