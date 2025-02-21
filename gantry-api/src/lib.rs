use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvariant::Type;


#[derive(Debug, Default, Serialize, Deserialize, Type, Clone, Copy)]
pub enum PrinterErrorCode {
    #[default]
    None,
    /// printer is in error state, ignored
    ErrorState,
    /// printer has been shutdown, ignored
    ShutdownState,
    /// printer is starting up, ignored
    StartupState,
    /// failed to authenticate
    AuthFailed,
    /// unauthorised access
    AuthRequired,
    /// auth token is invalid
    AuthTokenInvalid,
    /// auth token timedout, must autherise again
    AuthTokenTimeout,
    /// refresh token invalid
    RefreshTokenInvalid,
    /// error parsing gcode
    GcodeParseError,
    /// print job already running
    PrintJobRunning,
    /// file not found
    FileNotFound
}

#[derive(Debug, Default, Serialize, Deserialize, Type, Clone)]
pub struct PrinterError {
    /// error code
    pub code: PrinterErrorCode,
    /// message
    pub message: String,
}

impl PrinterError {
    pub const NONE: Self = PrinterError {
        code: PrinterErrorCode::None,
        message: String::new(),
    };
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterResult<T: Type>{
    pub error: PrinterError,
    pub result: T
}

impl<T: Type> PrinterResult<T>{
    pub const fn ok(result: T) -> Self{
        Self { error: PrinterError::NONE, result }
    }
}

impl<T: Type> PrinterResult<T> where T: Default{
    pub fn err(error: PrinterError) -> Self{
        Self { error, result: Default::default() }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterLogin {
    /// the temporary token
    pub token: String,
    /// the refresh token
    pub refresh_token: String,
}

/// printer state
#[derive(Debug, Default, Serialize, Deserialize, Type, Clone, Copy)]
pub enum PrinterState{
    /// printer is up and running
    Ready,
    /// printer is at startup phase
    #[default]
    Startup,
    /// printer encoutoured error
    Error,
    /// printer has been shutdown
    Shutdown,
}

/// generic printer information
#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterInfo{
    /// printer state
    pub state: PrinterState,
    /// only used when in error state
    pub error_state_code: PrinterErrorCode,
    /// only used when in error state
    pub error_state_message: String,
    /// path where printer data is stored
    pub printer_path: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterExtension{
    pub name: String,
    pub repo: String,
    pub version: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterEndstopStatus{
    pub x_triggered: bool,
    pub y_triggered: bool,
    pub z_triggered: bool
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterGcodeFile{
    pub path: String,
    pub modified: u64,
    pub size: u64,
    pub permissions: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterGcodeThumbnail{
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub relative_path: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterGcodeFileMetadata{
    pub size: u64,
    pub modified: u64,
    pub uuid: String,
    pub file_processors: Vec<String>,
    /// The name of the slicer software used to slice the file.
    pub slicer: String,
    /// The version of the slicer software.
    pub slicer_version: String,
    /// The byte offset in the file where the first gcode command is detected.
    pub gcode_start_byte: i32,
    /// The byte offset in the file where the last gcode command is detected.
    pub gcode_int_byte: i32,
    pub object_height: f32,
    pub estimated_time: f32,
    pub nozzle_diameter: f32,
    pub layer_height: f32,
    pub first_layer_height: f32,
    pub first_layer_extr_temp: f32,
    pub first_layer_bed_temp: f32,
    pub chamber_temp: f32,
    pub filament_name: String,
    pub filament_type: String,
    pub filament_total: f32,
    pub filament_weight_total: f32,
    pub thumbnails: Vec<PrinterGcodeThumbnail>,
    pub job_id: String,
    pub print_start_time: f64,
    pub filename: String,
}

#[zbus::proxy(
    interface = "org.gantry.Printer",
    default_service = "org.gantry.ThreeD",
    default_path = "/org/gantry/instance0"
)]
pub trait Printer{
    /////////////////////////////////////////////
    ///////////      Authentication    //////////
    /////////////////////////////////////////////

    /// login to the printer
    pub async fn login(&self, pwd: &str) -> PrinterResult<PrinterLogin>;
    /// logout from the printer
    pub async fn logout(&self, token: &str) -> PrinterResult<()>;
    /// reset password
    pub async fn reset_password(&self, token: &str, new_password: &str) -> PrinterResult<()>;
    /// refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> PrinterResult<PrinterLogin>;

    /////////////////////////////////////////////
    ///////////         Status        ///////////
    /////////////////////////////////////////////
    
    /// get printer info
    pub async fn get_info(&self, token: &str) -> PrinterResult<PrinterInfo>;
    /// emergency stop
    pub async fn emergency_stop(&self, token: &str) -> PrinterResult<()>;
    /// restart gantry
    pub async fn restart(&self, token: &str) -> PrinterResult<()>;
    /// list objects loaded
    pub async fn list_objects(&self, token: &str) -> PrinterResult<HashMap<String, String>>;
    /// list extensions loaded
    pub async fn list_extensions(&self, token: &str) -> PrinterResult<HashMap<String, PrinterExtension>>;
    /// install an extension
    pub async fn install_extension(&self, token: &str, repo: String) -> PrinterResult<()>;
    /// remove an extension
    pub async fn remove_extension(&self, token: &str, name: String) -> PrinterResult<()>;
    /// query endstop status
    pub async fn query_endstops(&self, token: &str) -> PrinterResult<PrinterEndstopStatus>;

    /////////////////////////////////////////////
    ///////////       Gcode API       ///////////
    /////////////////////////////////////////////
    
    /// execute a gcode script
    pub async fn run_gcode(&self, token: &str, script: String) -> PrinterResult<()>;
    /// Retrieves a list of registered GCode Command Descriptions.
    pub async fn get_gcode_help(&self, token: &str) -> PrinterResult<HashMap<String, String>>;

    /////////////////////////////////////////////
    ///////////       Print job       ///////////
    /////////////////////////////////////////////
    
    /// start a print job
    pub async fn start_print_job(&self, token: &str, filename: String) -> PrinterResult<()>;
    /// pause the print job
    pub async fn pause_print_job(&self, token: &str) -> PrinterResult<()>;
    /// resume the print job
    pub async fn resume_print_job(&self, token: &str) -> PrinterResult<()>;
    /// cancel the print job
    pub async fn cancel_print_job(&self, token: &str) -> PrinterResult<()>;

    /////////////////////////////////////////////
    ///////////      Gcode files      ///////////
    /////////////////////////////////////////////
    
    /// list avaliable gcode files
    pub async fn list_files(&self, token: &str) -> PrinterResult<Vec<PrinterGcodeFile>>;
    /// get metadata for a specified gcode file
    pub async fn get_file_metadata(&self, token: &str, filename: String) -> PrinterResult<()>;
    /// Initiate a metadata scan for a selected file. If the file has already been scanned the endpoint will force a re-scan.
    pub async fn scan_file_metadata(&self, token: &str, filename: String) -> PrinterResult<()>;
}