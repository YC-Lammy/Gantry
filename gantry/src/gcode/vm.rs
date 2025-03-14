use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ahash::AHashMap;
use tokio::fs::File;

use crate::printer::action::ActionQueue;

use super::parser::GcodeFile;

pub type GcodeHandler = Box<
    dyn for<'a> Fn(
            &'a GcodeVM,
            &'a [String],
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
    pub fn suspend(&self) {
        self.suspended.store(true, Ordering::SeqCst);
    }

    /// resume the vm
    pub fn resume(&self) {
        self.suspended.store(false, Ordering::SeqCst);
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::SeqCst)
    }

    pub async fn run_gcode_file(&self, file: File) -> anyhow::Result<()> {
        let file = GcodeFile::async_parse(file).await?;

        let mut count = 0;

        self.action_queue
            .state
            .gcode_line
            .store(count, Ordering::SeqCst);

        for cmd in file.commands {
            self.run_gcode(&cmd.cmd, &cmd.params).await?;

            count += 1;

            self.action_queue
                .state
                .gcode_line
                .store(count, Ordering::SeqCst);
        }

        return Ok(());
    }

    async fn run_gcode(&self, cmd: &str, params: &[String]) -> anyhow::Result<()> {
        // ignore gcode if suspended
        if self.is_suspended() {
            return Ok(());
        }

        let command = cmd.to_lowercase();

        let handler = self
            .functions
            .get(&command)
            .ok_or(anyhow::Error::msg(format!("Unknown command: {}", cmd)))?;

        (handler)(self, &params).await?;

        return Ok(());
    }

    pub async fn run_gcode_string(&self, input: &str) -> anyhow::Result<()> {
        // split each line
        for line in input.split_terminator('\n') {
            // return immediately when abort
            if self.is_suspended() {
                return Ok(());
            }
            // run a line of gcode
            self.run_single_line_gcode_string(line.trim()).await?;
        }
        // flush the action queue
        self.action_queue.flush().await;
        // return
        return Ok(());
    }

    /// runs a single line of gcode
    async fn run_single_line_gcode_string(&self, mut line: &str) -> anyhow::Result<()> {
        // either it is empty or a comment
        if line == "" || line.starts_with(';') {
            return Ok(());
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
            params.push(p.to_string());
        }

        return self.run_gcode(command, &params).await;
    }
}
