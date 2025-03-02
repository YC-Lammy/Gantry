use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ahash::AHashMap;

use crate::printer::action::ActionQueue;

pub type GcodeHandler = Box<
    dyn for<'a> Fn(
            &'a GcodeVM,
            Vec<&'a str>,
        )
            -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + Sync + 'a>>
        + Send
        + Sync,
>;

pub struct GcodeVM {
    suspended: AtomicBool,
    pub(super) action_queue: Arc<ActionQueue>,
    functions: AHashMap<String, GcodeHandler>,
}

impl GcodeVM {
    pub fn new(action_queue: Arc<ActionQueue>) -> Self {
        let mut functions: AHashMap<String, GcodeHandler> = AHashMap::new();

        functions.insert("g0".into(), Box::new(super::g1::handler));
        functions.insert("g1".into(), Box::new(super::g1::handler));

        Self {
            suspended: AtomicBool::new(false),
            action_queue,
            functions,
        }
    }

    /// abort the vm, abort any running gcodes
    pub fn abort(&self) {
        self.suspended.store(true, Ordering::SeqCst);
    }

    /// resume the vm
    pub fn resume(&self) {
        self.suspended.store(false, Ordering::SeqCst);
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::SeqCst)
    }

    pub async fn run_gcodes(&self, file: &str) -> anyhow::Result<()> {
        // split each line
        for line in file.split_terminator('\n') {
            // return immediately when abort
            if self.is_suspended() {
                return Ok(());
            }
            // run a line of gcode
            self.run_gcode_line(line.trim()).await?;
        }
        // flush the action queue
        self.action_queue.flush().await;
        // return
        return Ok(());
    }

    /// runs a single line of gcode
    pub async fn run_gcode_line(&self, mut line: &str) -> anyhow::Result<String> {
        // either it is empty or a comment
        if line == "" || line.starts_with(';') {
            return Ok(String::new());
        }
        // remove comment at line end
        if let Some((l, _)) = line.split_once(';') {
            line = l;
        }

        // params are split by spaces
        let mut iter = line.split(' ');

        // get the command
        let command = iter.next().unwrap();

        let mut params = Vec::new();

        for p in iter {
            // multiple whitespace will result in empty string
            if p == "" {
                continue;
            }
            // push param
            params.push(p);
        }

        let handler = self
            .functions
            .get(&command.to_lowercase())
            .ok_or(anyhow::Error::msg(format!("Unknown command: {}", command)))?;

        return (handler)(self, params).await;
    }
}
