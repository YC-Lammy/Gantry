use std::collections::HashMap;

use gantry_api::PrinterErrorCode;

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

    pub async fn restart(&mut self, config: PrinterConfig) {
        todo!()
    }

    pub fn emergency_stop(&mut self) {}

    /// returns endstop triggered xyz
    pub async fn get_endstop_status(&self) -> (bool, bool, bool) {
        todo!()
    }

    pub async fn run_gcode(&mut self, script: String) -> Result<(), String> {
        todo!()
    }
}
