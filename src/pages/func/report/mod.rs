mod index;
use axum::Router;

pub fn report_router() -> Router {
    index::index_router()
}
