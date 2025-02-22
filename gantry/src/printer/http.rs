use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum_auth::AuthBearer;
use serde::Deserialize;

use gantry_api::*;

use super::Instance;

/// create router for the printer interface
pub fn create_service_router() -> axum::Router{
    
    // login and refresh_token does not have bearer token
    let without_bearer = axum::Router::new()
    .route("/login", post(login))
    .route("/refresh_token", post(refresh_token))
    .layer(axum::middleware::from_fn(instance_extracter));

    // all other methods requires bearer token
    let with_bearer = axum::Router::new()
    .route("/logout", post(logout))
    .route("/reset_password", post(reset_password))
    .route("/info", get(get_info))
    .route("/emergency_stop", post(emergency_stop))
    .route("/restart", post(restart))
    .route("/list_objects", get(list_objects))
    .route("/query_endstops", get(query_endstops))
    .layer(axum::middleware::from_fn(instance_authenticator));

    without_bearer.merge(with_bearer)
}

/// find the instance by name
async fn find_instance(name: &str) -> Option<Arc<Instance>>{
    let instances = crate::INSTANCES.read().await;

    return instances.get(name).cloned()
}

/// query printer name
#[derive(Deserialize)]
pub struct PrinterNameQuery{
    /// 'name' in query
    name: String
}

/// extracte instance and verify bearer token
async fn instance_authenticator(
    AuthBearer(bearer_token): AuthBearer,
    query: Query<PrinterNameQuery>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode>{
    // get the instance request is refering to
    let instance = match find_instance(&query.name).await{
        Some(i) => i,
        None => return Err(StatusCode::BAD_REQUEST)
    };

    if let Err(_) = instance.validate_token(&bearer_token){
        return Err(StatusCode::UNAUTHORIZED)
    }

    request.extensions_mut().insert(instance);

    return Ok(next.run(request).await)
}

/// extract instance wothout verifying bearer
async fn instance_extracter(
    query: Query<PrinterNameQuery>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode>{
    // get the instance request is refering to
    let instance = match find_instance(&query.name).await{
        Some(i) => i,
        None => return Err(StatusCode::BAD_REQUEST)
    };

    request.extensions_mut().insert(instance);

    return Ok(next.run(request).await)
}

/////////////////////////////////////////////
///////////      Authentication    //////////
/////////////////////////////////////////////
#[derive(Deserialize)]
pub struct Login{
    password: String
}
/// login to the printer
pub async fn login(Extension(instance): Extension<Arc<Instance>>, Json(login): Json<Login>) -> Json<PrinterResult<PrinterLogin>>{
    Json(instance.login(&login.password).await)
}
/// logout from the printer
pub async fn logout(Extension(instance): Extension<Arc<Instance>>, AuthBearer(bearer_token): AuthBearer,) -> Json<PrinterResult<()>>{
    Json(instance.logout(&bearer_token).await)
}
#[derive(Deserialize)]
pub struct ResetPassword{
    new_password: String
}
/// reset password
pub async fn reset_password(Extension(instance): Extension<Arc<Instance>>, AuthBearer(bearer_token): AuthBearer, Json(reset): Json<ResetPassword>) -> Json<PrinterResult<()>>{
    Json(instance.reset_password(&bearer_token, &reset.new_password).await)
}
#[derive(Deserialize)]
pub struct RefreshToken{
    refresh_token: String
}
/// refresh token
pub async fn refresh_token(Extension(instance): Extension<Arc<Instance>>, Json(refresh): Json<RefreshToken>) -> Json<PrinterResult<PrinterLogin>>{
    Json(instance.refresh_token(&refresh.refresh_token).await)
}

/////////////////////////////////////////////
///////////         Status        ///////////
/////////////////////////////////////////////

/// get printer info
pub async fn get_info(Extension(instance): Extension<Arc<Instance>>) -> Json<PrinterResult<PrinterInfo>>{
    Json(instance.get_info().await)
}
/// emergency stop
pub async fn emergency_stop(Extension(instance): Extension<Arc<Instance>>) -> Json<PrinterResult<()>>{
    Json(instance.emergency_stop())
}
/// restart gantry
pub async fn restart(Extension(instance): Extension<Arc<Instance>>) -> Json<PrinterResult<()>>{
    Json(instance.restart().await)
}
/// list objects loaded
pub async fn list_objects(Extension(instance): Extension<Arc<Instance>>) -> Json<PrinterResult<HashMap<String, String>>>{
    Json(instance.list_objects().await)
}
/// query endstop status
pub async fn query_endstops(Extension(instance): Extension<Arc<Instance>>) -> Json<PrinterResult<PrinterEndstopStatus>>{
    Json(instance.query_endstops().await)
}