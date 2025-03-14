use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Query, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum_auth::AuthBearer;
use serde::{Deserialize, Serialize};

use tokio::fs::File;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use gantry_api::*;
use uuid::Uuid;

use super::auth::Auth;
use super::dbus::DBusInstance;
use crate::config::InstanceConfig;
use crate::gcode::GcodeFile;

pub struct PrintJob {
    pub uuid: Uuid,
    pub start_time: u64,
}

/// Instance is the interface exposed to external API
pub struct Instance {
    /// index of instance
    pub index: usize,
    /// name of instance
    pub name: String,
    /// uuid of instance
    pub uuid: u128,
    /// path where printer data is stored
    printer_path: PathBuf,
    /// used to authenticate and store temporary tokens
    auth: Auth,
    /// the printer object, will be none unless state is ready
    printer: Arc<RwLock<super::Printer>>,
    print_jobs: RwLock<Vec<(Uuid, String)>>,
}

impl Instance {
    pub async fn create(
        index: usize,
        name: String,
        config: InstanceConfig,
        gantry_path: PathBuf,
    ) -> Self {
        // printer path
        let printer_path = gantry_path.join(&name);

        if !printer_path.exists() {
            tokio::fs::create_dir(&printer_path)
                .await
                .expect("failed to create printer directory");

            let gcodes = printer_path.join("gcodes");
            tokio::fs::create_dir_all(gcodes.join("build"))
                .await
                .expect("failed to create directory");
            tokio::fs::create_dir(gcodes.join("thumbnails"))
                .await
                .expect("failed to create directory");

            tokio::fs::create_dir(printer_path.join("extensions"))
                .await
                .expect("failed to create directory");
        }

        // create instance
        let inst = Self {
            index,
            name,
            uuid: config.uuid,
            auth: Auth::acquire(config.uuid),
            printer_path,
            printer: Arc::new(RwLock::new(super::Printer::new())),
            print_jobs: RwLock::new(Vec::new()),
        };

        // start the printer
        inst.restart().await;

        return inst;
    }

    pub fn create_dbus_service(self: Arc<Self>) -> DBusInstance {
        DBusInstance { inner: self }
    }

    pub fn path(&self) -> &PathBuf {
        &self.printer_path
    }

    /// get state of printer
    pub async fn state(&self) -> super::printer::State {
        self.printer.read().await.state()
    }

    /// checks authentication
    pub async fn validate_token_state(&self, token: &str) -> Option<PrinterError> {
        // validate token
        if let Err(err) = self.validate_token(token) {
            return Some(err);
        }

        // validate state
        match self.state().await {
            super::printer::State::Error { code, message } => {
                return Some(PrinterError { code, message });
            }
            super::printer::State::Shutdown => {
                return Some(PrinterError {
                    code: PrinterErrorCode::ShutdownState,
                    message: String::new(),
                });
            }
            super::printer::State::Startup => {
                return Some(PrinterError {
                    code: PrinterErrorCode::StartupState,
                    message: String::new(),
                });
            }
            super::printer::State::Ready => {}
        }

        return None;
    }

    /// validates the token
    pub fn validate_token(&self, token: &str) -> Result<(), PrinterError> {
        // validate token
        let (is_valid, is_timeout) = self.auth.validate_token(token);

        // return timeout error
        if is_timeout {
            return Err(PrinterError {
                code: PrinterErrorCode::AuthTokenTimeout,
                message: String::new(),
            });
        }

        // return invalid error
        if !is_valid {
            return Err(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            });
        }

        return Ok(());
    }

    /////////////////////////////////////////////
    ///////////      Authentication    //////////
    /////////////////////////////////////////////

    /// login to the printer
    pub async fn login(&self, pwd: &str) -> PrinterResult<PrinterLogin> {
        match self.auth.login(pwd) {
            Some((token, refresh_token)) => PrinterResult::ok(PrinterLogin {
                token,
                refresh_token,
            }),
            None => PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthFailed,
                message: String::new(),
            }),
        }
    }
    /// logout from the printer
    pub async fn logout(&self, token: &str) -> PrinterResult<()> {
        match self.auth.logout(token) {
            true => PrinterResult::ok(()),
            false => PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            }),
        }
    }
    /// reset password
    pub async fn reset_password(&self, token: &str, new_password: &str) -> PrinterResult<()> {
        if !self.auth.reset_password(token, new_password) {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthFailed,
                message: String::new(),
            });
        }

        return PrinterResult::ok(());
    }
    /// refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> PrinterResult<PrinterLogin> {
        match self.auth.refresh_token(refresh_token) {
            Some((token, refresh_token)) => PrinterResult::ok(PrinterLogin {
                token,
                refresh_token,
            }),
            None => PrinterResult::err(PrinterError {
                code: PrinterErrorCode::RefreshTokenInvalid,
                message: String::new(),
            }),
        }
    }

    /////////////////////////////////////////////
    ///////////         Status        ///////////
    /////////////////////////////////////////////

    /// get printer info
    pub async fn get_info(&self) -> PrinterResult<PrinterInfo> {
        let printer_state = self.state().await;

        let state: PrinterState;
        let mut error_state_code = PrinterErrorCode::None;
        let mut error_state_message = String::new();

        match printer_state {
            super::printer::State::Error { code, message } => {
                state = PrinterState::Error;
                error_state_code = code;
                error_state_message = message;
            }
            super::printer::State::Ready => {
                state = PrinterState::Ready;
            }
            super::printer::State::Shutdown => {
                state = PrinterState::Shutdown;
            }
            super::printer::State::Startup => {
                state = PrinterState::Startup;
            }
        }

        return PrinterResult::ok(PrinterInfo {
            state,
            error_state_code,
            error_state_message,
            printer_path: self.path().to_string_lossy().to_string(),
        });
    }

    pub async fn get_temperatures(&self) -> PrinterResult<Vec<PrinterTemperatureInfo>> {
        todo!()
    }

    /// emergency stop
    pub async fn emergency_stop(&self) -> PrinterResult<()> {
        // block the current thread to stop ASAP
        tokio::task::block_in_place(|| {
            let mut printer = self.printer.blocking_write();
            printer.emergency_stop();
        });

        return PrinterResult::ok(());
    }

    /// restart the printer
    pub async fn restart(&self) -> PrinterResult<()> {
        // acquire write lock
        let mut printer = self.printer.write().await;

        // stop the printer
        printer.emergency_stop();

        let printer = self.printer.clone();
        let printer_config_path = self.path().join("printer.cfg");

        tokio::spawn(async move {
            printer.write().await.restart(printer_config_path).await;
        });

        return PrinterResult::ok(());
    }

    /// list objects loaded
    pub async fn list_objects(&self) -> PrinterResult<HashMap<String, String>> {
        todo!()
    }

    /// returns endstop triggered xyz
    pub async fn query_endstops(&self) -> PrinterResult<PrinterEndstopStatus> {
        let printer = self.printer.read().await;

        let (x, y, z) = printer.get_endstop_status().await;

        return PrinterResult::ok(PrinterEndstopStatus {
            x_triggered: x,
            y_triggered: y,
            z_triggered: z,
        });
    }

    /////////////////////////////////////////////
    ///////////       Extensions      ///////////
    /////////////////////////////////////////////

    /// list extensions loaded
    pub async fn list_extensions(&self) -> PrinterResult<HashMap<String, PrinterExtension>> {
        todo!()
    }
    /// install an extension
    pub async fn install_extension(&self, repo: String) -> PrinterResult<()> {
        todo!()
    }
    /// remove an extension
    pub async fn remove_extension(&self, name: String) -> PrinterResult<()> {
        todo!()
    }
    /// download extension config
    pub async fn download_extension_config(&self, name: &str) -> PrinterResult<String> {
        todo!()
    }
    /// upload extension config
    pub async fn upload_extension_config(&self, name: &str, config: String) -> PrinterResult<()> {
        todo!()
    }

    /////////////////////////////////////////////
    ///////////       Gcode API       ///////////
    /////////////////////////////////////////////

    pub async fn run_gcode(&self, script: String) -> PrinterResult<()> {
        let printer = self.printer.read().await;

        if let Err(e) = printer.run_gcode_string(script).await {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::GcodeError,
                message: e.to_string(),
            });
        }

        return PrinterResult::ok(());
    }

    pub async fn get_gcode_help(&self) -> PrinterResult<HashMap<String, String>> {
        todo!()
    }

    /////////////////////////////////////////////
    ///////////       Print job       ///////////
    /////////////////////////////////////////////

    /// start a print job
    pub async fn start_print_job(
        &self,
        filename: &str,
        exclude_objects: Vec<String>,
    ) -> PrinterResult<StartPrintJobResult> {
        // create path
        let path = self.printer_path.join("gcodes").join(filename);

        let file = match crate::files::open_gcode_file(path).await {
            Ok(f) => f,
            Err(e) => {
                return PrinterResult::err(PrinterError {
                    code: PrinterErrorCode::GcodeParseError,
                    message: e.to_string(),
                });
            }
        };

        let uuid = Uuid::new_v4();

        let printer = self.printer.clone();

        printer
            .read()
            .await
            .spawn_print_job(uuid, file, exclude_objects)
            .await;

        return PrinterResult::ok(StartPrintJobResult {
            job_id: uuid.to_string(),
        });
    }
    /// pause the print job
    pub async fn pause_print_job(&self) -> PrinterResult<()> {
        todo!()
    }
    /// resume the print job
    pub async fn resume_print_job(&self) -> PrinterResult<()> {
        todo!()
    }
    /// cancel the print job
    pub async fn cancel_print_job(&self) -> PrinterResult<()> {
        todo!()
    }

    pub async fn get_print_job_status(&self) -> PrinterResult<PrintJobStatus> {
        todo!()
    }

    /// queue print job to run after current print job is finished
    pub async fn queue_print_job(&self, filename: &str) -> PrinterResult<PrinterQueuePrintJob> {
        todo!()
    }
    //// delete a print job in queue
    pub async fn delete_queue_print_job(&self, id: &str) -> PrinterResult<()> {
        todo!()
    }

    /// pause the job queue, next job will not start when current job is finished
    pub async fn pause_job_queue(&self) -> PrinterResult<()> {
        todo!()
    }

    /// resume the job queue
    pub async fn resume_job_queue(&self) -> PrinterResult<()> {
        todo!()
    }

    /// get a list of jobs in job queue
    pub async fn list_job_queue(&self) -> PrinterResult<Vec<JobQueuePrintJob>> {
        todo!()
    }

    /////////////////////////////////////////////
    ///////////      Gcode files      ///////////
    /////////////////////////////////////////////

    /// list avaliable gcode files
    pub async fn list_files(&self) -> PrinterResult<Vec<PrinterGcodeFile>> {
        todo!()
    }
    /// get metadata for a specified gcode file
    pub async fn get_file_metadata(
        &self,
        filename: &str,
    ) -> PrinterResult<PrinterGcodeFileMetadata> {
        todo!()
    }
    /// Initiate a metadata scan for a selected file. If the file has already been scanned the endpoint will force a re-scan.
    pub async fn scan_file_metadata(&self, filename: &str) -> PrinterResult<()> {
        todo!()
    }
    /// upload a gcode file
    pub async fn upload_file(&self, filename: &str, filedata: String) -> PrinterResult<()> {
        todo!()
    }
    /// download a gcode file
    pub async fn download_file(&self, filename: &str) -> PrinterResult<String> {
        todo!()
    }
    /// download the printer config
    pub async fn download_printer_config(&self) -> PrinterResult<String> {
        todo!()
    }
    /// upload the printer config
    pub async fn upload_printer_config(&self, config: String) -> PrinterResult<()> {
        todo!()
    }
}

/////////////////////////////////////////////
///////////       REST API        ///////////
/////////////////////////////////////////////

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
    pub exclude_objects: Vec<String>,
}
/// start a print job
pub async fn start_print_job(
    Extension(instance): Extension<Arc<Instance>>,
    Json(params): Json<StartPrintJobParams>,
) -> Json<PrinterResult<StartPrintJobResult>> {
    Json(
        instance
            .start_print_job(&params.filename, params.exclude_objects)
            .await,
    )
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
