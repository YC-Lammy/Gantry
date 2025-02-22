
use url::Url;

use serde::{Deserialize, Serialize};
use zvariant::Type;


#[derive(Debug, Default, Serialize, Deserialize, Type, Clone, Copy)]
pub enum PrinterErrorCode {
    #[default]
    None,
    /// can be any error
    GenericError,
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
    /// error parsing config
    PrinterConfigParseError,
    /// error parsing gcode
    GcodeParseError,
    /// print job already running
    PrintJobRunning,
    /// no print job running
    PrintJobNotRunning,
    /// file not found
    FileNotFound,
    /// file system has full capacity
    FileCapacityFull
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
    pub result: Option<T>
}

impl<T: Type> PrinterResult<T>{
    pub const fn ok(result: T) -> Self{
        Self { error: PrinterError::NONE, result: Some(result) }
    }
}

impl<T: Type> PrinterResult<T> where T: Default{
    pub fn err(error: PrinterError) -> Self{
        Self { error, result: Default::default() }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginParams{
    pub password: String
}

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterLogin {
    /// the temporary token
    pub token: String,
    /// the refresh token
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetPasswordParams{
    pub new_password: String
}

#[derive(Deserialize)]
pub struct RefreshTokenParams{
    refresh_token: String
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

#[derive(Debug, Default, Serialize, Deserialize, Type)]
pub struct PrinterQueuePrintJob{
    pub id: u64
}

/// zbus proxy
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
    pub async fn login(&self, password: &str) -> PrinterResult<PrinterLogin>;
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
    /// query endstop status
    pub async fn query_endstops(&self, token: &str) -> PrinterResult<PrinterEndstopStatus>;

    /////////////////////////////////////////////
    ///////////       Extensions      ///////////
    /////////////////////////////////////////////
    
    /// list extensions loaded
    pub async fn list_extensions(&self, token: &str) -> PrinterResult<HashMap<String, PrinterExtension>>;
    /// install an extension
    pub async fn install_extension(&self, token: &str, repo: String) -> PrinterResult<()>;
    /// remove an extension
    pub async fn remove_extension(&self, token: &str, name: String) -> PrinterResult<()>;
    /// download extension config
    pub async fn download_extension_config(&self, token: &str, name: &str) -> PrinterResult<String>;
    /// upload extension config
    pub async fn upload_extension_config(&self, token: &str, name: &str,  config: String) -> PrinterResult<()>;

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
    pub async fn start_print_job(&self, token: &str, filename: &str) -> PrinterResult<()>;
    /// pause the print job
    pub async fn pause_print_job(&self, token: &str) -> PrinterResult<()>;
    /// resume the print job
    pub async fn resume_print_job(&self, token: &str) -> PrinterResult<()>;
    /// cancel the print job
    pub async fn cancel_print_job(&self, token: &str) -> PrinterResult<()>;
    /// queue print job to run after current print job is finished
    pub async fn queue_print_job(&self, token: &str, filename: &str) -> PrinterResult<PrinterQueuePrintJob>;
    //// delete a print job in queue
    pub async fn delete_queue_print_job(&self, token: &str, id: u64) -> PrinterResult<()>;

    /////////////////////////////////////////////
    ///////////      Gcode files      ///////////
    /////////////////////////////////////////////
    
    /// list avaliable gcode files
    pub async fn list_files(&self, token: &str) -> PrinterResult<Vec<PrinterGcodeFile>>;
    /// get metadata for a specified gcode file
    pub async fn get_file_metadata(&self, token: &str, filename: &str) -> PrinterResult<()>;
    /// Initiate a metadata scan for a selected file. If the file has already been scanned the endpoint will force a re-scan.
    pub async fn scan_file_metadata(&self, token: &str, filename: &str) -> PrinterResult<()>;
    /// upload a gcode file
    pub async fn upload_file(&self, token: &str, filename: &str, filedata: String) -> PrinterResult<()>;
    /// download a gcode file
    pub async fn download_file(&self, token: &str, filename: &str) -> PrinterResult<String>;
    /// download the printer config
    pub async fn download_printer_config(&self, token: &str) -> PrinterResult<String>;
    /// upload the printer config
    pub async fn upload_printer_config(&self, token: &str, config: String) -> PrinterResult<()>;
}

#[derive(Debug)]
pub enum PrinterRestError{
    UrlError(url::ParseError),
    HttpError(reqwest::Error),
    PrinterError(PrinterError)
}

type PrinterRestResult<T> = Result<T, PrinterRestError>;

/// printer REST API client
pub struct PrinterRestClient{
    client: reqwest::Client,
    url: Url,
    printer_name: String,
    bearer: String,
    refresh_token: String,
}

impl PrinterRestClient{
    pub async fn new(url: &str, printer_name: &str) -> Result<Self, PrinterRestError>{
        let client = reqwest::Client::builder()
        //.add_root_certificate(cert)()
        .build().unwrap();

        let url = match Url::parse(url){
            Ok(u) => u.join("printer").unwrap(),
            Err(e) => return Err(PrinterRestError::UrlError(e))
        };

        Ok(Self { 
            client,
            url,
            printer_name: printer_name.to_string(),
            bearer: String::new() ,
            refresh_token: String::new()
        })
    }
    
    pub fn handle_json_response<T>(&self, re: Result<reqwest::Response, reqwest::Error>) -> Result<T, PrinterRestError>{
        todo!()
    }

    /////////////////////////////////////////////
    ///////////      Authentication    //////////
    /////////////////////////////////////////////

    /// login to the printer
    pub async fn login(&mut self, password: &str) -> PrinterRestResult<()>{
        let re = self.client.post(self.url.join("login").unwrap()).query(&[("name", &self.printer_name)]).json(&LoginParams{
            password: password.to_string()
        })
        .send()
        .await;
        
        let tokens = self.handle_json_response::<PrinterLogin>(re)?;
        self.bearer = tokens.token;
        self.refresh_token = tokens.refresh_token;

        return Ok(())
    }
    /// logout from the printer
    pub async fn logout(&self) -> PrinterResult<()>{
        todo!()
    }
    /// reset password
    pub async fn reset_password(&self, new_password: &str) -> PrinterResult<()>{
        todo!()
    }
    /// refresh token
    pub async fn refresh_token(&self) -> PrinterResult<PrinterLogin>{
        todo!()
    }
}