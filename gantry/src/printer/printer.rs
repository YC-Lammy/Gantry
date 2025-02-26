use std::collections::HashMap;
use std::path::PathBuf;

use gantry_api::PrinterErrorCode;
use tokio::io::AsyncReadExt;

use crate::config::PrinterConfig;

pub enum State {
    Startup,
    Ready,
    Error {
        code: PrinterErrorCode,
        message: String,
    },
    Shutdown,
}

pub struct Printer {}

impl Printer {
    pub fn new() -> Self {
        todo!()
    }

    pub fn state(&self) -> State {
        todo!()
    }

    pub fn set_error_state(&mut self, code: PrinterErrorCode, message: String) {}

    pub async fn restart(&mut self, config_path: PathBuf) {
        let mut printer_config = Vec::new();

        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_path)
            .await;

        let mut file = match file {
            Ok(f) => f,
            Err(e) => todo!(),
        };

        let re = file.read_to_end(&mut printer_config).await;

        if let Err(e) = re {
            todo!()
        }
        todo!()
    }

    pub fn emergency_stop(&mut self) {}

    /// returns endstop triggered xyz
    pub async fn get_endstop_status(&self) -> (bool, bool, bool) {
        todo!()
    }

    pub async fn run_gcode(&self, script: String) -> Result<(), String> {
        todo!()
    }
}
