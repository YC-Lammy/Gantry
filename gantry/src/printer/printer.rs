use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use gantry_api::PrinterErrorCode;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::config::PrinterConfig;
use crate::gcode::GcodeFile;
use crate::gcode::vm::GcodeVM;

use super::action::{ActionQueue, ActionState, PrinterAction};

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

#[derive(Debug)]
pub enum PrinterEvent {
    Action(PrinterAction),
    RunNextPrintJob,
}

#[derive(Debug)]
pub struct PrintJob {
    pub id: Uuid,
    /// filename
    pub file: Arc<GcodeFile>,
    /// linux timestamp
    pub start_timestamp: Option<u64>,
    /// exluded objects
    pub exlude_objects: Vec<String>,
}

pub struct Printer {
    /// generic status of printer
    state: State,
    /// status of physical printer
    action_state: Arc<ActionState>,
    /// queue for kinematic actions, trapezoid generator
    action_queue: Arc<ActionQueue>,
    /// gcode virtual machine
    vm: Arc<GcodeVM>,
    /// job queue
    print_job_queue: RwLock<VecDeque<PrintJob>>,
    /// sender to send events to event loop
    event_sender: UnboundedSender<PrinterEvent>,
    /// join handle for event loop
    event_loop_handle: Option<JoinHandle<()>>,
}

impl Printer {
    pub fn new() -> Self {
        let (event_sender, event_reciever) = unbounded_channel();

        let action_state = Arc::new(ActionState::new());
        let action_queue = Arc::new(ActionQueue::new(action_state.clone(), event_sender.clone()));
        let vm = Arc::new(GcodeVM::new(action_queue.clone()));

        Self {
            state: State::Startup,
            action_state,
            action_queue,
            vm,
            print_job_queue: RwLock::const_new(VecDeque::new()),
            event_sender,
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
        self.vm.suspend();
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

    pub fn is_gcode_running(&self) -> bool {
        self.action_state
            .gcode_running
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// spawns a tokio task to run print jobs
    pub async fn spawn_print_job(
        &self,
        id: Uuid,
        file: Arc<GcodeFile>,
        exlude_objects: Vec<String>,
    ) {
        let mut job_queue = self.print_job_queue.write().await;

        job_queue.push_back(PrintJob {
            id,
            file,
            start_timestamp: None,
            exlude_objects,
        });

        if !self.is_gcode_running() {
            let _ = self.event_sender.send(PrinterEvent::RunNextPrintJob);
        }
    }

    /// runs a gcode string immediately
    pub async fn run_gcode_string(&self, script: String) -> anyhow::Result<()> {
        return self.vm.run_gcode_string(&script).await;
    }
}
