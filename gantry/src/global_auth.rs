use axum::extract::{Query, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_auth::AuthBearer;

use serde::Deserialize;

/// query printer name
#[derive(Deserialize)]
pub struct PrinterNameQuery {
    /// 'name' in query
    name: String,
}

pub async fn auth_middleware(
    AuthBearer(bearer_token): AuthBearer,
    query: Query<PrinterNameQuery>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    todo!()
}

/// login a user, returns bearer and refresh token
pub fn login(username: &str, password: &str) -> Option<(String, String)> {
    todo!()
}

pub fn logout(token: &str) -> bool {
    todo!()
}

pub fn reset_password(token: &str, password: &str) -> bool {
    todo!()
}

/// refresh bearer token using refresh token
pub fn refresh_token(refresh_token: &str) -> Option<(String, String)> {
    todo!()
}

pub fn validate_token(token: &str) -> (bool, bool) {
    todo!()
}
