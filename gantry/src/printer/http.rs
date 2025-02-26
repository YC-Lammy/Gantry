use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum_auth::AuthBearer;
use serde::{Deserialize, Serialize};

use gantry_api::*;

use super::Instance;

/// create router for the printer interface
pub fn create_service_router() -> axum::Router {
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
        .route("/temperatures", get(get_temperatures))
        .route("/emergency_stop", post(emergency_stop))
        .route("/restart", post(restart))
        .route("/list_objects", get(list_objects))
        .route("/query_endstops", get(query_endstops))
        .route("/list_extensions", get(list_extensions))
        .route("/install_extension", post(install_extension))
        .route("/remove_extension", post(remove_extension))
        .route("/download_extension_config", get(download_extension_config))
        .route("/upload_extension_config", post(upload_extension_config))
        .route("/run_gcode", post(run_gcode))
        .route("/gcode_help", get(get_gcode_help))
        .route("/start_print_job", post(start_print_job))
        .route("/pause_print_job", post(pause_print_job))
        .route("/resume_print_job", post(resume_print_job))
        .route("/cancel_print_job", post(cancel_print_job))
        .route("/print_job_status", get(get_print_job_status))
        .route("/queue_print_job", post(queue_print_job))
        .route("/delete_queue_print_job", post(delete_queue_print_job))
        .route("/pause_job_queue", post(pause_job_queue))
        .route("/resume_job_queue", post(resume_job_queue))
        .route("/list_job_queue", get(list_job_queue))
        .route("/list_files", get(list_files))
        .route("/file_metadata", get(get_file_metadata))
        .route("/scan_file_metadata", post(scan_file_metadata))
        .route("/download_file", get(download_file))
        .route("/upload_file", post(upload_file))
        .route("/download_printer_config", get(download_printer_config))
        .route("/upload_printer_config", post(upload_printer_config))
        .layer(axum::middleware::from_fn(instance_authenticator));

    without_bearer.merge(with_bearer)
}

/// find the instance by name
async fn find_instance(name: &str) -> Option<Arc<Instance>> {
    let instances = crate::INSTANCES.read().await;

    return instances.get(name).cloned();
}

/// query printer name
#[derive(Deserialize)]
pub struct PrinterNameQuery {
    /// 'name' in query
    name: String,
}

/// extracte instance and verify bearer token
async fn instance_authenticator(
    AuthBearer(bearer_token): AuthBearer,
    query: Query<PrinterNameQuery>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // get the instance request is refering to
    let instance = match find_instance(&query.name).await {
        Some(i) => i,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    if let Err(_) = instance.validate_token(&bearer_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    request.extensions_mut().insert(instance);

    return Ok(next.run(request).await);
}

/// extract instance wothout verifying bearer
async fn instance_extracter(
    query: Query<PrinterNameQuery>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // get the instance request is refering to
    let instance = match find_instance(&query.name).await {
        Some(i) => i,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    request.extensions_mut().insert(instance);

    return Ok(next.run(request).await);
}

/////////////////////////////////////////////
///////////      Authentication    //////////
/////////////////////////////////////////////
#[derive(Deserialize)]
pub struct LoginParams {
    pub password: String,
}
/// login to the printer
pub async fn login(
    Extension(instance): Extension<Arc<Instance>>,
    Json(login): Json<LoginParams>,
) -> Json<PrinterResult<PrinterLogin>> {
    Json(instance.login(&login.password).await)
}
/// logout from the printer
pub async fn logout(
    Extension(instance): Extension<Arc<Instance>>,
    AuthBearer(bearer_token): AuthBearer,
) -> Json<PrinterResult<()>> {
    Json(instance.logout(&bearer_token).await)
}
#[derive(Deserialize)]
pub struct ResetPasswordParams {
    pub new_password: String,
}
/// reset password
pub async fn reset_password(
    Extension(instance): Extension<Arc<Instance>>,
    AuthBearer(bearer_token): AuthBearer,
    Json(reset): Json<ResetPasswordParams>,
) -> Json<PrinterResult<()>> {
    Json(
        instance
            .reset_password(&bearer_token, &reset.new_password)
            .await,
    )
}
#[derive(Deserialize)]
pub struct RefreshTokenParams {
    pub refresh_token: String,
}
/// refresh token
pub async fn refresh_token(
    Extension(instance): Extension<Arc<Instance>>,
    Json(refresh): Json<RefreshTokenParams>,
) -> Json<PrinterResult<PrinterLogin>> {
    Json(instance.refresh_token(&refresh.refresh_token).await)
}

/////////////////////////////////////////////
///////////         Status        ///////////
/////////////////////////////////////////////

/// get printer info
pub async fn get_info(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<PrinterInfo>> {
    Json(instance.get_info().await)
}
/// get printer temperatures
pub async fn get_temperatures(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<Vec<PrinterTemperatureInfo>>> {
    Json(instance.get_temperatures().await)
}
/// emergency stop
pub async fn emergency_stop(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<()>> {
    Json(instance.emergency_stop().await)
}
/// restart gantry
pub async fn restart(Extension(instance): Extension<Arc<Instance>>) -> Json<PrinterResult<()>> {
    Json(instance.restart().await)
}
/// list objects loaded
pub async fn list_objects(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<HashMap<String, String>>> {
    Json(instance.list_objects().await)
}
/// query endstop status
pub async fn query_endstops(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<PrinterEndstopStatus>> {
    Json(instance.query_endstops().await)
}

/////////////////////////////////////////////
///////////       Extensions      ///////////
/////////////////////////////////////////////

/// list extensions loaded
pub async fn list_extensions(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<HashMap<String, PrinterExtension>>> {
    Json(instance.list_extensions().await)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallExtensionParams {
    pub repo: String,
}
/// install an extension
pub async fn install_extension(
    Extension(instance): Extension<Arc<Instance>>,
    Json(install): Json<InstallExtensionParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.install_extension(install.repo).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveExtensionParams {
    pub name: String,
}
/// remove an extension
pub async fn remove_extension(
    Extension(instance): Extension<Arc<Instance>>,
    Json(remove): Json<RemoveExtensionParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.remove_extension(remove.name).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadExtensionConfigParams {
    pub name: String,
}
/// download extension config
pub async fn download_extension_config(
    Extension(instance): Extension<Arc<Instance>>,
    Json(download): Json<DownloadExtensionConfigParams>,
) -> Json<PrinterResult<String>> {
    Json(instance.download_extension_config(&download.name).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadExtensionConfigParams {
    pub name: String,
    pub config: String,
}
/// upload extension config
pub async fn upload_extension_config(
    Extension(instance): Extension<Arc<Instance>>,
    Json(upload): Json<UploadExtensionConfigParams>,
) -> Json<PrinterResult<()>> {
    Json(
        instance
            .upload_extension_config(&upload.name, upload.config)
            .await,
    )
}

/////////////////////////////////////////////
///////////       Gcode API       ///////////
/////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize)]
pub struct RunGcodeParams {
    pub script: String,
}

/// execute a gcode script
pub async fn run_gcode(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<RunGcodeParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.run_gcode(params.script).await)
}
/// Retrieves a list of registered GCode Command Descriptions.
pub async fn get_gcode_help(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<HashMap<String, String>>> {
    Json(instance.get_gcode_help().await)
}

/////////////////////////////////////////////
///////////       Print job       ///////////
/////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize)]
pub struct StartPrintJobParams {
    pub filename: String,
}
/// start a print job
pub async fn start_print_job(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<StartPrintJobParams>,
) -> Json<PrinterResult<StartPrintJobResult>> {
    Json(instance.start_print_job(&params.filename).await)
}
/// pause the print job
pub async fn pause_print_job(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<()>> {
    Json(instance.pause_print_job().await)
}
/// resume the print job
pub async fn resume_print_job(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<()>> {
    Json(instance.resume_print_job().await)
}
/// cancel the print job
pub async fn cancel_print_job(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<()>> {
    Json(instance.cancel_print_job().await)
}
/// get print job status
pub async fn get_print_job_status(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<PrintJobStatus>> {
    Json(instance.get_print_job_status().await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct QueuePrintJobParams {
    pub filename: String,
}
/// queue print job to run after current print job is finished
pub async fn queue_print_job(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<QueuePrintJobParams>,
) -> Json<PrinterResult<PrinterQueuePrintJob>> {
    Json(instance.queue_print_job(&params.filename).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteQueuePrintJobParams {
    pub id: String,
}
//// delete a print job in queue
pub async fn delete_queue_print_job(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<DeleteQueuePrintJobParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.delete_queue_print_job(&params.id).await)
}

/// pause the job queue, next job will not start when current job is finished
pub async fn pause_job_queue(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<()>> {
    Json(instance.pause_job_queue().await)
}

/// resume the job queue
pub async fn resume_job_queue(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<()>> {
    Json(instance.resume_job_queue().await)
}

/// get a list of jobs in job queue
pub async fn list_job_queue(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<Vec<JobQueuePrintJob>>> {
    Json(instance.list_job_queue().await)
}

/////////////////////////////////////////////
///////////      Gcode files      ///////////
/////////////////////////////////////////////

/// list avaliable gcode files
pub async fn list_files(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<Vec<PrinterGcodeFile>>> {
    Json(instance.list_files().await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GetFileMetaParams {
    pub filename: String,
}
/// get metadata for a specified gcode file
pub async fn get_file_metadata(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<GetFileMetaParams>,
) -> Json<PrinterResult<PrinterGcodeFileMetadata>> {
    Json(instance.get_file_metadata(&params.filename).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanFileParams {
    pub filename: String,
}
/// Initiate a metadata scan for a selected file. If the file has already been scanned the endpoint will force a re-scan.
pub async fn scan_file_metadata(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<ScanFileParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.scan_file_metadata(&params.filename).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFileParams {
    pub filename: String,
    pub data: String,
}
/// upload a gcode file
pub async fn upload_file(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<UploadFileParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.upload_file(&params.filename, params.data).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadFileParams {
    pub filename: String,
}
/// download a gcode file
pub async fn download_file(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<DownloadFileParams>,
) -> Json<PrinterResult<String>> {
    Json(instance.download_file(&params.filename).await)
}
/// download the printer config
pub async fn download_printer_config(
    Extension(instance): Extension<Arc<Instance>>,
) -> Json<PrinterResult<String>> {
    Json(instance.download_printer_config().await)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadPrinterConfigParams {
    pub config: String,
}
/// upload the printer config
pub async fn upload_printer_config(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<UploadPrinterConfigParams>,
) -> Json<PrinterResult<()>> {
    Json(instance.upload_printer_config(params.config).await)
}
