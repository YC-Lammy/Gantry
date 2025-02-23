use std::{collections::HashMap, path::PathBuf, sync::Arc};

use tokio::sync::RwLock;

use gantry_api::*;

use super::dbus::DBusInstance;
use crate::config::{InstanceConfig, PrinterConfig};

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
    auth: super::auth::Auth,
    /// the printer object, will be none unless state is ready
    printer: RwLock<super::Printer>,
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
            auth: super::auth::Auth::acquire(config.uuid),
            printer_path,
            printer: RwLock::new(super::Printer::new()),
        };

        // start the printer
        inst.restart().await;

        return inst;
    }

    pub fn create_dbus_service(self: Arc<Self>) -> DBusInstance {
        DBusInstance { inner: self }
    }

    pub fn create_axum_router(self: Arc<Self>) -> axum::Router {
        axum::Router::new()
            .with_state(self)
            .route("login", axum::routing::post(|| async {}))
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

    pub async fn get_temperatures(&self) -> PrinterResult<Vec<PrinterTemperatureInfo>>{
        todo!()
    }

    /// emergency stop
    pub fn emergency_stop(&self) -> PrinterResult<()> {
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

        let printer_config_path = self.path().join("printer.cfg");

        let printer_config = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&printer_config_path)
            .await
            .expect("failed to open printer.cfg");

        match PrinterConfig::parse(printer_config).await {
            Ok(config) => printer.restart(config).await,
            Err(msg) => {
                // set printer to error state
                printer.set_error_state(PrinterErrorCode::PrinterConfigParseError, msg);
            }
        };

        return PrinterResult::ok(());
    }

    /// list objects loaded
    pub async fn list_objects(&self) -> PrinterResult<HashMap<String, String>> {
        todo!()
    }

    /// returns endstop triggered xyz
    pub async fn query_endstops(&self) -> PrinterResult<PrinterEndstopStatus> {
        todo!()
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
        todo!()
    }

    pub async fn get_gcode_help(&self) -> PrinterResult<HashMap<String, String>> {
        todo!()
    }

    /////////////////////////////////////////////
    ///////////       Print job       ///////////
    /////////////////////////////////////////////

    /// start a print job
    pub async fn start_print_job(&self, filename: &str) -> PrinterResult<StartPrintJobResult> {
        todo!()
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

    pub async fn get_print_job_status(&self) -> PrinterResult<PrintJobStatus>{
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
    pub async fn get_file_metadata(&self, filename: &str) -> PrinterResult<PrinterGcodeFileMetadata> {
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
