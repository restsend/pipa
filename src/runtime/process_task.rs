use std::os::unix::process::ExitStatusExt;

use super::context::JSContext;
use super::extension::MacroTaskExtension;

#[derive(Debug)]
pub struct ProcessResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: i32,
    pub signal: Option<i32>,
    pub error: Option<String>,
}

pub struct ProcessExtension;

impl ProcessExtension {
    pub fn new() -> Self {
        ProcessExtension
    }
}

impl MacroTaskExtension for ProcessExtension {
    fn tick(&mut self, _ctx: &mut JSContext) -> Result<bool, String> {
        
        Ok(false)
    }

    fn has_pending(&self) -> bool {
        
        false
    }
}

pub fn run_command_sync(command: &str, args: &[String]) -> ProcessResult {
    match std::process::Command::new(command)
        .args(args)
        .output()
    {
        Ok(output) => {
            let code = output.status.code().unwrap_or(-1);
            let signal = if output.status.signal().is_some() {
                output.status.signal()
            } else {
                None
            };
            ProcessResult {
                stdout: output.stdout,
                stderr: output.stderr,
                code,
                signal,
                error: None,
            }
        }
        Err(e) => ProcessResult {
            stdout: Vec::new(),
            stderr: Vec::new(),
            code: -1,
            signal: None,
            error: Some(format!("spawn failed: {e}")),
        },
    }
}
