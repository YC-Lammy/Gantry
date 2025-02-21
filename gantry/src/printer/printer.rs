use std::collections::HashMap;

use crate::config::PrinterConfig;

pub struct Printer {}

impl Printer {
    pub fn new() -> Self{
        todo!()
    }
    
    pub async fn create(config: PrinterConfig) -> Self {
        todo!()
    }

    pub fn emergency_stop(&mut self){

    }

    /// returns endstop triggered xyz
    pub async fn get_endstop_status(&self) -> (bool, bool, bool){
        todo!()
    }

    pub async fn run_gcode(&mut self, script: String) -> Result<(), String>{
        todo!()
    }

    pub async fn get_gcode_help(&self) -> HashMap<String, String>{
        todo!()
    }

    pub async fn start_print_job(&mut self, filename: String) -> bool{
        todo!()
    }

    pub async fn pause_print_job(&mut self){

    }

    pub async fn resume_print_job(&mut self){

    }

    pub async fn cancel_print_job(&mut self){
        
    }
}
