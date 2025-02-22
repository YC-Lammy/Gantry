use axum::Router;
use axum::response::Html;
use axum::routing::get;

pub fn create_service_router() -> Router {
    Router::new().route("/server_info", get(get_server_info))
}

pub async fn get_server_info() -> String {
    "hello world".to_string()
}
