// Use anyhow implicitly or use std::result::Result explicitly
use chrono::{DateTime, Local, Utc};
use cron::Schedule;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;
use which::which;

#[derive(Serialize, Deserialize, Debug)]
pub struct EnvInfo {
    pub os: String,
    pub has_node: bool,
    pub has_pnpm: bool,
    pub has_openclaw: bool,
    pub has_ollama: bool,
    pub has_playwright: bool,
}

fn db_path() -> std::path::PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push("personaliz-desktop");
    std::fs::create_dir_all(&p).ok();
    p.push("personaliz.db");
    p
}

fn get_conn() -> anyhow::Result<Connection> {
    let path = db_path();
    let conn = Connection::open(path)?;
    Ok(conn)
}

#[tauri::command]
pub fn detect_env() -> std::result::Result<EnvInfo, String> {
    let os = std::env::consts::OS.to_string();
    let has_node = which("node").is_ok();
    let has_pnpm = which("pnpm").is_ok();
    let has_openclaw = which("openclaw").is_ok();
    let has_ollama = which("ollama").is_ok();
    // detect playwright by checking for `npx playwright --version` availability via pnpm
    let has_playwright = if has_pnpm {
        // quick check: attempt `pnpm --version` to confirm pnpm works
        std::process::Command::new("pnpm")
            .arg("-v")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };

    Ok(EnvInfo {
        os,
        has_node,
        has_pnpm,
        has_openclaw,
        has_ollama,
        has_playwright,
    })
}

#[tauri::command]
pub fn install_openclaw() -> std::result::Result<String, String> {
    let pnpm = which("pnpm").is_ok();
    let cmd = if pnpm { "pnpm" } else { "npm" };
    
    let mut c = Command::new(cmd);
    if pnpm {
        c.args(["add", "-g", "openclaw"]);
    } else {
        c.args(["install", "-g", "openclaw"]);
    }

    match c.output() {
        Ok(out) => {
            if out.status.success() {
                Ok("OpenClaw installed successfully".into())
            } else {
                Err(format!("Failed to install OpenClaw: {}", String::from_utf8_lossy(&out.stderr)))
            }
        }
        Err(e) => Err(format!("Failed to run install command: {}", e)),
    }
}

#[tauri::command]
pub async fn check_ollama() -> std::result::Result<bool, String> {
    let client = reqwest::Client::new();
    let res = client.get("http://localhost:11434/api/tags").send().await;
    Ok(res.is_ok())
}

#[tauri::command]
pub fn ensure_phi3() -> std::result::Result<String, String> {
    let mut c = Command::new("ollama");
    c.args(["pull", "phi3"]);

    match c.output() {
        Ok(out) => {
            if out.status.success() {
                Ok("Phi-3 model pulled successfully".into())
            } else {
                Err(format!("Failed to pull Phi-3: {}", String::from_utf8_lossy(&out.stderr)))
            }
        }
        Err(e) => Err(format!("Failed to run ollama pull: {}", e)),
    }
}

#[tauri::command]
pub fn run_shell_command(cmd: String) -> Result<String, String> {
    // Run via shell so users can pass full commands. Keep simple for demo.
    #[cfg(target_os = "windows")]
    let mut c = Command::new("cmd");
    #[cfg(target_os = "windows")]
    c.args(["/C", &cmd]);

    #[cfg(not(target_os = "windows"))]
    let mut c = Command::new("sh");
    #[cfg(not(target_os = "windows"))]
    c.args(["-lc", &cmd]);

    match c.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
            Ok(combined)
        }
        Err(e) => Err(format!("failed to run command: {}", e)),
    }
}

#[tauri::command]
pub fn db_init() -> Result<String, String> {
    match get_conn() {
        Ok(conn) => {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS agents (
                    id TEXT PRIMARY KEY,
                    name TEXT,
                    description TEXT,
                    config_json TEXT,
                    created_at TEXT
                );
                CREATE TABLE IF NOT EXISTS schedules (
                    id TEXT PRIMARY KEY,
                    agent_id TEXT,
                    cron_expr TEXT,
                    next_run TEXT
                );
                CREATE TABLE IF NOT EXISTS logs (
                    id TEXT PRIMARY KEY,
                    agent_id TEXT,
                    level TEXT,
                    message TEXT,
                    ts TEXT
                );
                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT
                );",
            )
            .map_err(|e| e.to_string())?;
            Ok("ok".into())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn create_demo_agents() -> Result<String, String> {
    let conn = get_conn().map_err(|e| e.to_string())?;

    // Trending LinkedIn Agent
    let id1 = Uuid::new_v4().to_string();
    let config1 = serde_json::json!({
        "role": "trending-researcher",
        "goal": "Search OpenClaw trending topics and draft a LinkedIn post; request approval before posting.",
        "automation": {"script":"linkedin_post"},
        "sandbox": false
    });

    conn.execute(
        "INSERT OR REPLACE INTO agents (id, name, description, config_json, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![
            &id1,
            "Trending LinkedIn Agent",
            "Find trending OpenClaw topics and prepare LinkedIn post (requires approval)",
            config1.to_string(),
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| e.to_string())?;

    // schedule daily at 09:00
    let sid1 = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO schedules (id, agent_id, cron_expr, next_run) VALUES (?1,?2,?3,?4)",
        params![&sid1, &id1, "0 9 * * *", ""],
    )
    .map_err(|e| e.to_string())?;

    // Hashtag Comment Agent
    let id2 = Uuid::new_v4().to_string();
    let config2 = serde_json::json!({
        "role": "hashtag-promoter",
        "goal": "Every hour search LinkedIn for #openclaw and comment promoting the repo; do not post personal data.",
        "automation": {"script":"linkedin_comment"},
        "sandbox": false
    });

    conn.execute(
        "INSERT OR REPLACE INTO agents (id, name, description, config_json, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![&id2, "Hashtag Comment Agent", "Hourly search for #openclaw and comment to promote repo", config2.to_string(), chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;

    let sid2 = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO schedules (id, agent_id, cron_expr, next_run) VALUES (?1,?2,?3,?4)",
        params![&sid2, &id2, "0 * * * *", ""],
    )
    .map_err(|e| e.to_string())?;

    Ok("demo agents created".into())
}

#[derive(Serialize)]
pub struct AgentSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: serde_json::Value,
    pub created_at: String,
}

#[tauri::command]
pub fn list_agents() -> Result<Vec<AgentSummary>, String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, description, config_json, created_at FROM agents ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let cfg_str: String = r.get(3)?;
            let cfg: serde_json::Value = serde_json::from_str(&cfg_str).unwrap_or(serde_json::json!({}));
            Ok(AgentSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                config: cfg,
                created_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn run_agent_now(agent_id: String, sandbox: bool) -> Result<String, String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT config_json FROM agents WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let cfg_str: String = stmt
        .query_row([&agent_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let cfg: serde_json::Value = serde_json::from_str(&cfg_str).map_err(|e| e.to_string())?;
    let automation = cfg
        .get("automation")
        .and_then(|a| a.get("script"))
        .and_then(|s| s.as_str())
        .unwrap_or("");

    // For demo, automation.script can be "linkedin_post" or "linkedin_comment"
    let script_cmd = match automation {
        "linkedin_post" => vec!["run", "playwright:post"],
        "linkedin_comment" => vec!["run", "playwright:comment"],
        _ => vec![],
    };

    if script_cmd.is_empty() {
        return Err("No automation script configured for this agent".into());
    }

    // If sandbox, do not actually execute external script; instead simulate output
    if sandbox {
        // record a log
        let _ = conn.execute(
            "INSERT INTO logs (id, agent_id, level, message, ts) VALUES (?1,?2,?3,?4,?5)",
            params![Uuid::new_v4().to_string(), &agent_id, "INFO", "Sandbox run - simulated actions", chrono::Utc::now().to_rfc3339()],
        );
        return Ok("sandbox: simulated run completed".into());
    }

    // Attempt to call pnpm to run the script (requires pnpm available in PATH)
    let pnpm = if which("pnpm").is_ok() { "pnpm" } else { "npm" };
    let mut cmd = Command::new(pnpm);
    cmd.args(&script_cmd);
    // run in project workspace
    cmd.current_dir(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let msg = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
            let _ = conn.execute(
                "INSERT INTO logs (id, agent_id, level, message, ts) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), &agent_id, "INFO", &msg, chrono::Utc::now().to_rfc3339()],
            );
            Ok(msg)
        }
        Err(e) => Err(format!("failed to spawn automation script: {}", e)),
    }
}

#[tauri::command]
pub fn get_logs_legacy(limit: Option<u32>) -> std::result::Result<Vec<HashMap<String, String>>, String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(200) as i64;
    let mut stmt = conn
        .prepare("SELECT id, agent_id, level, message, ts FROM logs ORDER BY ts DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([lim], |r| {
            let mut m = HashMap::new();
            m.insert("id".to_string(), r.get::<_, String>(0)?);
            m.insert("agent_id".to_string(), r.get::<_, String>(1)?);
            m.insert("level".to_string(), r.get::<_, String>(2)?);
            m.insert("message".to_string(), r.get::<_, String>(3)?);
            m.insert("ts".to_string(), r.get::<_, String>(4)?);
            Ok(m)
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn start_scheduler_thread() {
    // Spawn a background thread that wakes every 30s and checks schedules
    thread::spawn(|| loop {
        if let Ok(conn) = get_conn() {
            if let Ok(mut stmt) = conn.prepare("SELECT id, agent_id, cron_expr FROM schedules") {
                let rows = stmt.query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
                });
                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        let (sid, agent_id, cron_expr) = row;
                        if let Ok(schedule) = Schedule::from_str(&cron_expr) {
                            let now: DateTime<Utc> = Utc::now();
                            if let Some(next) = schedule.upcoming(Utc).next() {
                                let next_ts = next;
                                // If next occurrence is within the next 30s, run it
                                let diff = next_ts.signed_duration_since(now).num_seconds();
                                if diff <= 30 && diff >= 0 {
                                    // run agent (best-effort)
                                    let _ = run_agent_now(agent_id.clone(), false);
                                    // log next_run update
                                    let _ = conn.execute(
                                        "UPDATE schedules SET next_run = ?1 WHERE id = ?2",
                                        params![next_ts.to_rfc3339(), sid],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_secs(30));
    });
}
