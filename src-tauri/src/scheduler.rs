use tokio::time::{sleep, Duration};
use crate::db::{DbState, Agent, log_event};

pub async fn start_scheduler(state: DbState) {
    loop {
        let agents = {
            let conn = state.0.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, name, description, openclaw_task, schedule, status, last_run_at, next_run_at FROM agents WHERE status = 'Active'").unwrap();
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
            }).unwrap();

            let mut list = Vec::new();
            for agent in agent_iter {
                list.push(agent.unwrap());
            }
            list
        };

        for agent in agents {
            {
                let conn = state.0.lock().unwrap();
                let _ = log_event(&conn, Some(agent.id), &format!("Processing agent: {}", agent.name), "Info");
            }
            
            if agent.name.contains("Trending") {
                // Demo 1: Search trends and create approval
                let trend = crate::openclaw::search_trends();
                let conn = state.0.lock().unwrap();
                conn.execute(
                    "INSERT INTO approvals (agent_id, content) VALUES (?, ?)",
                    rusqlite::params![agent.id, trend],
                ).unwrap();
                let _ = log_event(&conn, Some(agent.id), "Created approval for Trending Agent", "Info");
            } else if agent.name.contains("Hashtag") {
                // Demo 2: Run comment script directly
                {
                    let conn = state.0.lock().unwrap();
                    let _ = log_event(&conn, Some(agent.id), "Running Hashtag Agent script...", "Info");
                }
                let result = crate::openclaw::run_script("linkedin_comment.cjs", "#openclaw");
                
                let conn = state.0.lock().unwrap();
                let _ = log_event(&conn, Some(agent.id), &format!("Hashtag agent script finished. Result: {:?}", result), "Info");
                let _ = log_event(&conn, Some(agent.id), "Commented on LinkedIn posts", "Info");
            }
            
            // Mark as run
            let conn = state.0.lock().unwrap();
            conn.execute(
                "UPDATE agents SET last_run_at = CURRENT_TIMESTAMP WHERE id = ?",
                [agent.id],
            ).unwrap();
        }

        sleep(Duration::from_secs(60)).await;
    }
}
