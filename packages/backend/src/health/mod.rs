use crate::store;
use axum::http::StatusCode;

pub async fn report_health() -> (StatusCode,) {
    if store::can_reach_cache() {
        (StatusCode::OK,)
    } else {
        (StatusCode::SERVICE_UNAVAILABLE,)
    }
}
