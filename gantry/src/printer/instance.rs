

use std::{path::PathBuf, sync::Arc};

use tokio::sync::RwLock;

use gantry_api::{PrinterError, PrinterState, PrinterErrorCode};

use crate::config::InstanceConfig;
use super::dbus::DBusInstance;

pub struct Instance {
    /// index of instance
    pub index: usize,
    /// name of instance
    pub name: String,
    /// uuid of instance
    pub uuid: u128,
    /// path where printer data is stored
    pub printer_path: PathBuf,

    /// used to authenticate and store temporary tokens
    pub auth: super::auth::Auth,
    /// state of the printer
    state: PrinterState,
    /// If error state is true, client must restart the pinter.
    /// It may be caused by error formatting printer config or temperature status etc.
    error_state: PrinterError,
    
    /// the printer object, will be none unless state is ready
    pub printer: RwLock<super::Printer>
}

impl Instance {
    pub fn create(
        index: usize,
        name: String,
        config: InstanceConfig,
        gantry_path: PathBuf,
    ) -> Self {
        // printer path
        let printer_path = gantry_path.join(&name);

        Self {
            index,
            name,
            uuid: config.uuid,
            state: PrinterState::Startup,
            error_state: PrinterError::NONE,
            auth: super::auth::Auth::acquire(config.uuid),
            printer_path,
            printer: RwLock::new(super::Printer::new())
        }
    }

    pub fn create_dbus_service(self: Arc<Self>) -> DBusInstance{
        DBusInstance{
            inner: self
        }
    }

    pub fn create_axum_router(self: Arc<Self>) -> axum::Router{
        axum::Router::new()
        .with_state(self)
        .route("login", axum::routing::post(||async {}))
    }

    pub fn state(&self) -> PrinterState{
        self.state
    }

    pub fn set_state(&self, state: PrinterState){
        todo!()
    }

    pub async fn error_state(&self) -> PrinterError{
        self.error_state.clone()
    }

    pub async fn check_auth(&self, token: &str) -> Option<PrinterError> {
        // check state of printer
        match self.state() {
            PrinterState::Error => {
                return Some(PrinterError {
                    code: PrinterErrorCode::ErrorState,
                    message: self.error_state.message.clone(),
                });
            }
            PrinterState::Shutdown => {
                return Some(PrinterError {
                    code: PrinterErrorCode::ShutdownState,
                    message: String::new(),
                });
            }
            PrinterState::Startup => {
                return Some(PrinterError {
                    code: PrinterErrorCode::StartupState,
                    message: String::new(),
                });
            }
            PrinterState::Ready => {}
        }

        let (is_valid, is_timeout) = self.auth.validate_token(token);

        if is_timeout {
            return Some(PrinterError {
                code: PrinterErrorCode::AuthTokenTimeout,
                message: String::new(),
            });
        }

        if !is_valid {
            return Some(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            });
        }

        return None;
    }

    pub async fn restart(&self){
        todo!()
    }
}