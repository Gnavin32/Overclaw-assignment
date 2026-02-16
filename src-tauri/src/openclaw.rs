use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenClawResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub fn search_trends() -> String {
    // Mocking OpenClaw trend search for demo
    "OpenClaw v2.0 is trending! New features include multi-modal agentic workflows and enhanced Tauri integration. #OpenClaw #AI"
    .to_string()
}

pub fn run_script(script_name: &str, arg: &str) -> OpenClawResult {
    let output = Command::new("node")
        .arg(format!("scripts/{}", script_name))
        .arg(arg)
        .output();

    match output {
        Ok(out) => {
            OpenClawResult {
                success: out.status.success(),
                output: String::from_utf8_lossy(&out.stdout).to_string(),
                error: if out.stderr.is_empty() { None } else { Some(String::from_utf8_lossy(&out.stderr).to_string()) },
            }
        }
        Err(e) => OpenClawResult {
            success: false,
            output: "".to_string(),
            error: Some(e.to_string()),
        },
    }
}
