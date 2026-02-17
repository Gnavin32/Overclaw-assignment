mod db;
use rusqlite::OptionalExtension;
mod openclaw;
mod scheduler;
mod commands;

use commands::{detect_env, install_openclaw, check_ollama, ensure_phi3};

use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use db::{DbState, Agent, Log, Approval};
use scheduler::start_scheduler;

#[tauri::command]
fn get_agents(state: State<DbState>) -> std::result::Result<Vec<Agent>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, description, openclaw_task, schedule, status, last_run_at, next_run_at FROM agents").map_err(|e| e.to_string())?;
    let agent_iter = stmt.query_map([], |row| {
        Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2).unwrap_or_default(),
            openclaw_task: row.get(3)?,
            schedule: row.get(4).unwrap_or_default(),
            status: row.get(5)?,
            last_run_at: row.get(6).ok(),
            next_run_at: row.get(7).ok(),
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for agent in agent_iter {
        list.push(agent.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn create_agent(state: State<DbState>, name: String, task: String, schedule: String) -> std::result::Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO agents (name, openclaw_task, schedule) VALUES (?, ?, ?)",
        [name, task, schedule],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_logs(state: State<DbState>) -> std::result::Result<Vec<Log>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, agent_id, timestamp, message, level FROM logs ORDER BY timestamp DESC LIMIT 50").map_err(|e| e.to_string())?;
    let log_iter = stmt.query_map([], |row| {
        Ok(Log {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            timestamp: row.get(2)?,
            message: row.get(3)?,
            level: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for log in log_iter {
        list.push(log.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn get_approvals(state: State<DbState>) -> std::result::Result<Vec<Approval>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, agent_id, content, status FROM approvals WHERE status = 'Pending'").map_err(|e| e.to_string())?;
    let app_iter = stmt.query_map([], |row| {
        Ok(Approval {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            content: row.get(2)?,
            status: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for app in app_iter {
        list.push(app.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn approve_request(state: State<DbState>, id: i32, approved: bool) -> std::result::Result<(), String> {
    let (content, current_status) = {
        let conn = state.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT content, status FROM approvals WHERE id = ?").map_err(|e| e.to_string())?;
        stmt.query_row([id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(|e| e.to_string())?
    };

    if approved && current_status == "Pending" {
        // Run the script
        {
            let conn = state.0.lock().unwrap();
            let _ = db::log_event(&conn, None, &format!("Triggering LinkedIn post for content: {}", content), "Info");
        }
        crate::openclaw::run_script("linkedin_post.cjs", &content);
    }

    let status_set = if approved { "Approved" } else { "Rejected" };
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE approvals SET status = ? WHERE id = ?",
        rusqlite::params![status_set, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn log_event_cmd(state: State<DbState>, agent_id: Option<i32>, message: String, level: String) -> std::result::Result<(), String> {
    let conn = state.0.lock().unwrap();
    db::log_event(&conn, agent_id, &message, &level).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_llm_settings(state: State<DbState>) -> std::result::Result<Option<String>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'llm_api_key'").map_err(|e| e.to_string())?;
    let res = stmt.query_row([], |row| row.get::<_, String>(0)).optional().map_err(|e: rusqlite::Error| e.to_string())?;
    Ok(res)
}

#[tauri::command]
fn update_llm_settings(state: State<DbState>, key: String) -> std::result::Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('llm_api_key', ?)",
        [key],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let data_dir = app_handle.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&data_dir).expect("failed to create app data dir");
            let db_path = data_dir.join("personaliz.db");
            
            let conn = db::init_db(db_path.to_str().expect("invalid path")).expect("failed to init db");
            let db_state = DbState(Arc::new(Mutex::new(conn)));
            app.manage(db_state.clone());

            // Start scheduler
            let scheduler_state = db_state.clone();
            tauri::async_runtime::spawn(async move {
                start_scheduler(scheduler_state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_agents,
            create_agent,
            get_logs,
            get_approvals,
            approve_request,
            log_event_cmd,
            get_llm_settings,
            update_llm_settings,
            detect_env,
            install_openclaw,
            check_ollama,
            ensure_phi3
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
