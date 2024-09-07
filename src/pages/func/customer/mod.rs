mod appointment;
mod colleague;
pub mod index;

use axum::Router;
use crate::libs::cache::CUSTOMER_CACHE;
use self::{appointment::appointment_router, colleague::colleague_router};

pub fn customer_router() -> Router {
    index::customer_router()
        .merge(colleague_router())
        .merge(appointment_router())
}
