use std::path::PathBuf;
use std::sync::Arc;

use gantry_api::PrinterErrorCode;
use tokio::io::AsyncReadExt;
use tokio::task::JoinHandle;

use crate::config::PrinterConfig;
use crate::gcode::vm::GcodeVM;

use super::action::ActionQueue;

#[derive(Debug, Clone)]
pub enum State {
    Startup,
    Ready,
    Error {
        code: PrinterErrorCode,
        message: String,
    },
    Shutdown,
}

pub struct Printer {
    state: State,
    action_queue: Arc<ActionQueue>,
    vm: GcodeVM,
    event_loop_handle: Option<JoinHandle<()>>,
}

impl Printer {
    pub fn new() -> Self {
        let action_queue = Arc::new(ActionQueue::new());
        let vm = GcodeVM::new(action_queue.clone());

        Self {
            state: State::Startup,
            action_queue,
            vm,
            event_loop_handle: None,
        }
    }

    pub fn state(&self) -> State {
        return self.state.clone();
    }

    /// stops the printer immediately
    pub fn emergency_stop(&mut self) {
        // abort the event loop
        if let Some(handle) = self.event_loop_handle.take() {
            handle.abort();
        }

        // abort the action queue
        self.action_queue.suspend();
        // abort the vm
        self.vm.abort();
        // set state to shutdown
        self.state = State::Shutdown;
    }

    /// restart the printer
    pub async fn restart(&mut self, config_path: PathBuf) {
        // set state to startup
        self.state = State::Startup;

        // buffer for printer config
        let mut printer_config = String::new();

        // open config file
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_path)
            .await;

        // error state if failed to open file
        let mut file = match file {
            Ok(f) => f,
            Err(e) => {
                self.state = State::Error {
                    code: PrinterErrorCode::FileNotFound,
                    message: e.to_string(),
                };

                return;
            }
        };

        // read file
        let re = file.read_to_string(&mut printer_config).await;

        // error state if failed to read file
        if let Err(e) = re {
            self.state = State::Error {
                code: PrinterErrorCode::FileReadError,
                message: e.to_string(),
            };

            return;
        }

        // parse the configuration
        let config = match PrinterConfig::parse(&printer_config) {
            Ok(c) => c,
            Err(e) => {
                self.state = State::Error {
                    code: PrinterErrorCode::FileReadError,
                    message: e.to_string(),
                };

                return;
            }
        };

        // clear the action queue
        self.action_queue.clear().await;
        // resume the action queue
        self.action_queue.resume();
        // resume the gcode vm
        self.vm.resume();

        todo!()
    }

    /// returns endstop triggered xyz
    pub async fn get_endstop_status(&self) -> (bool, bool, bool) {
        todo!()
    }

    pub async fn run_gcodes(&self, script: String) -> anyhow::Result<()> {
        return self.vm.run_gcodes(&script).await;
    }
}
