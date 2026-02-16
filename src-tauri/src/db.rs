use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Agent {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub openclaw_task: String,
    pub schedule: String,
    pub status: String,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub id: i32,
    pub agent_id: i32,
    pub timestamp: String,
    pub message: String,
    pub level: String, // Info, Error
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Approval {
    pub id: i32,
    pub agent_id: i32,
    pub content: String,
    pub status: String, // Pending, Approved, Rejected
}

#[derive(Clone)]
pub struct DbState(pub Arc<Mutex<Connection>>);

pub fn init_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS agents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            openclaw_task TEXT NOT NULL,
            schedule TEXT,
            status TEXT DEFAULT 'Active',
            last_run_at DATETIME,
            next_run_at DATETIME
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id INTEGER,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            message TEXT,
            level TEXT,
            FOREIGN KEY(agent_id) REFERENCES agents(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS approvals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id INTEGER,
            content TEXT,
            status TEXT DEFAULT 'Pending'
        )",
        [],
    )?;
    Ok(conn)
}

pub fn log_event(conn: &Connection, agent_id: Option<i32>, message: &str, level: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO logs (agent_id, message, level) VALUES (?, ?, ?)",
        rusqlite::params![agent_id, message, level],
    )?;
    Ok(())
}
