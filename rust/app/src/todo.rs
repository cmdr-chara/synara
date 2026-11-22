//! Native implementation of the `todo.*` WS-RPC domain
//! (`apps/server/src/todo/Layers/TodoService.ts` +
//! `persistence/Layers/TodoRepository.ts` + `packages/shared/src/todo.ts`).
//!
//! Rows live in `<state_dir>/native.sqlite` — a file of our own because the
//! original keeps `state.sqlite` under `PRAGMA locking_mode = EXCLUSIVE` for
//! its whole lifetime. The todo domain is self-contained, so a separate store
//! is indistinguishable over the wire.

use std::sync::Mutex;

use rusqlite::{Connection, params};
use serde_json::{Map, Value, json};
use tokio::sync::broadcast;

const PRIORITIES: &[&str] = &["none", "low", "medium", "high", "urgent"];
const TITLE_MAX: usize = 500;
const NOTES_MAX: usize = 10_000;

pub struct TodoStore {
    conn: Mutex<Connection>,
    events: broadcast::Sender<Value>,
}

pub struct WsErr(pub String, pub Option<&'static str>);

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

/// `server_env` maps its errors through the same wire shape.
pub fn err_pub(message: impl Into<String>) -> WsErr {
    err(message)
}

fn tasks_unavailable() -> WsErr {
    WsErr(
        "Tasks is unavailable on this server.".into(),
        Some("TASKS_UNAVAILABLE"),
    )
}

fn tasks_enabled() -> bool {
    // Stable desktop flavor only gates Beta-only features; "tasks" is not in
    // BETA_ONLY_FEATURES, so it is on everywhere.
    true
}

fn iso_now_ms() -> String {
    // JS `new Date().toISOString()` — millis precision.
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    iso_from_ms(ms)
}

fn iso_from_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let rem = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{rem:03}Z",
        day_secs / 3600,
        day_secs % 3600 / 60,
        day_secs % 60
    )
}

/// Howard Hinnant's civil_from_days.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn iso_to_ms(s: &str) -> Option<i64> {
    // YYYY-MM-DDTHH:MM:SS(.sss)?Z — the only shape this server writes.
    let b = s.as_bytes();
    if b.len() < 20 || s.ends_with('Z') == false {
        return None;
    }
    let g = |a: usize, b: usize| s[a..b].parse::<i64>().ok();
    let (y, m, d) = (g(0, 4)?, g(5, 7)?, g(8, 10)?);
    let (hh, mm, ss) = (g(11, 13)?, g(14, 16)?, g(17, 19)?);
    let days = days_from_civil(y, m as u32, d as u32);
    let mut ms = (days * 86_400 + hh * 3600 + mm * 60 + ss) * 1000;
    if b.len() > 20 && b[19] == b'.' {
        let frac: String = s[20..s.len() - 1].chars().take(3).collect();
        let padded = format!("{frac:0<3}");
        ms += padded.parse::<i64>().unwrap_or(0);
    }
    Some(ms)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn is_calendar_day(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let Ok(y) = s[0..4].parse::<i64>() else {
        return false;
    };
    let Ok(m) = s[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(d) = s[8..10].parse::<u32>() else {
        return false;
    };
    days_from_civil(y, m, d) > i64::MIN && days_from_civil(y, m, d).checked_add(0).is_some() && {
        // Round-trip: the date must be real (not 2026-02-31).
        let (ry, rm, rd) = civil_from_days(days_from_civil(y, m, d));
        (ry, rm, rd) == (y, m, d)
    }
}

fn next_todo_updated_at(previous: &str) -> String {
    let candidate = iso_now_ms();
    match iso_to_ms(previous) {
        Some(prev_ms) => match iso_to_ms(&candidate) {
            Some(cand_ms) if cand_ms <= prev_ms => iso_from_ms(prev_ms + 1),
            _ => candidate,
        },
        None => candidate,
    }
}

fn opt_str(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn validate_create(
    input: &Map<String, Value>,
) -> Result<(String, String, String, String, Value, Value), String> {
    let id = opt_str(input.get("id")).filter(|s| !s.is_empty());
    let title = opt_str(input.get("title")).map(|t| t.trim().to_string());
    let Some(id) = id else {
        return Err("Task id is required.".into());
    };
    let Some(title) = title.filter(|t| !t.is_empty()) else {
        return Err("Task title must be a non-empty string.".into());
    };
    if title.chars().count() > TITLE_MAX {
        return Err(format!(
            "Task title must be at most {TITLE_MAX} characters."
        ));
    }
    let notes = opt_str(input.get("notes")).unwrap_or_default();
    if notes.chars().count() > NOTES_MAX {
        return Err(format!(
            "Task notes must be at most {NOTES_MAX} characters."
        ));
    }
    let priority = opt_str(input.get("priority")).unwrap_or_else(|| "none".into());
    if !PRIORITIES.contains(&priority.as_str()) {
        return Err("Invalid task priority.".into());
    }
    let project_id = input.get("projectId").cloned().unwrap_or(Value::Null);
    let due_date = input.get("dueDate").cloned().unwrap_or(Value::Null);
    if let Value::String(d) = &due_date {
        if !is_calendar_day(d) {
            return Err("Task due date must be a calendar day.".into());
        }
    }
    Ok((id, title, notes, priority, project_id, due_date))
}

fn row_to_todo(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let g = |i: usize| r.get::<_, Option<String>>(i);
    Ok(json!({
        "id": g(0)?,
        "title": g(1)?,
        "notes": g(2)?,
        "priority": g(3)?,
        "projectId": g(4)?,
        "dueDate": g(5)?,
        "threadId": g(6)?,
        "delegationBaseTurnId": g(7)?,
        "linkedAt": g(8)?,
        "completedAt": g(9)?,
        "createdAt": g(10)?,
        "updatedAt": g(11)?,
    }))
}

const SELECT_ALL: &str = "SELECT todo_id, title, notes, priority, project_id, due_date,
    thread_id, delegation_base_turn_id, linked_at, completed_at, created_at, updated_at
    FROM todos ORDER BY created_at";

const SELECT_BY_ID: &str = "SELECT todo_id, title, notes, priority, project_id, due_date,
    thread_id, delegation_base_turn_id, linked_at, completed_at, created_at, updated_at
    FROM todos WHERE todo_id = ?1";

impl TodoStore {
    pub fn open(state_dir: &str) -> std::sync::Arc<Self> {
        let path = std::path::Path::new(state_dir);
        let _ = std::fs::create_dir_all(path);
        let conn = Connection::open(path.join("native.sqlite")).expect("opening the native store");
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS todos (
                todo_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                notes TEXT NOT NULL DEFAULT '',
                priority TEXT NOT NULL DEFAULT 'none'
                  CHECK (priority IN ('none', 'low', 'medium', 'high', 'urgent')),
                project_id TEXT,
                due_date TEXT,
                thread_id TEXT,
                delegation_base_turn_id TEXT,
                linked_at TEXT,
                completed_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
             );",
        )
        .expect("migrating the native store");
        let (events, _) = broadcast::channel(256);
        std::sync::Arc::new(Self {
            conn: Mutex::new(conn),
            events,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }

    fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Value>, String> {
        conn.query_row(SELECT_BY_ID, params![id], row_to_todo)
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })
    }

    fn list_all(conn: &Connection) -> Result<Vec<Value>, String> {
        let mut stmt = conn.prepare(SELECT_ALL).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_todo)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    fn publish(&self, event: Value) {
        let _ = self.events.send(event);
    }

    pub fn handle(&self, tag: &str, payload: Value) -> Result<Value, WsErr> {
        if !tasks_enabled() {
            return Err(tasks_unavailable());
        }
        match tag {
            "todo.list" => self.list(),
            "todo.create" => self.create(payload),
            "todo.update" => self.update(payload),
            "todo.delete" => self.delete(payload),
            _ => Err(err(format!("unknown todo method {tag}"))),
        }
    }

    fn list(&self) -> Result<Value, WsErr> {
        let conn = self.conn.lock().map_err(|_| err("store poisoned"))?;
        let todos = Self::list_all(&conn).map_err(|e| err(format!("Failed to list tasks. {e}")))?;
        Ok(json!({"todos": todos}))
    }

    fn create(&self, payload: Value) -> Result<Value, WsErr> {
        let Some(obj) = payload.as_object() else {
            return Err(err("Invalid task create payload."));
        };
        let (id, title, notes, priority, project_id, due_date) =
            validate_create(obj).map_err(err)?;
        let now = iso_now_ms();
        let conn = self.conn.lock().map_err(|_| err("store poisoned"))?;
        let inserted = conn.execute(
            "INSERT INTO todos (todo_id, title, notes, priority, project_id, due_date,
                thread_id, delegation_base_turn_id, linked_at, completed_at, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,NULL,NULL,NULL,NULL,?7,?7)
             ON CONFLICT (todo_id) DO NOTHING",
            params![
                id,
                title,
                notes,
                priority,
                project_id.as_str().map(|s| s.to_string()),
                due_date.as_str().map(|s| s.to_string()),
                now
            ],
        );
        match inserted {
            Ok(1) => {
                let todo = Self::get_by_id(&conn, &id)
                    .map_err(|e| err(format!("Failed to create the task. {e}")))?
                    .unwrap_or(Value::Null);
                self.publish(json!({"type": "todo-upserted", "todo": todo}));
                Ok(todo)
            }
            Ok(_) => Self::get_by_id(&conn, &id)
                .map_err(|e| err(format!("Failed to create the task. {e}")))?
                .ok_or_else(|| err("The task was not saved.")),
            Err(e) => Err(err(format!("Failed to create the task. {e}"))),
        }
    }

    fn delete(&self, payload: Value) -> Result<Value, WsErr> {
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| err("Task id is required."))?
            .to_string();
        let conn = self.conn.lock().map_err(|_| err("store poisoned"))?;
        conn.execute("DELETE FROM todos WHERE todo_id = ?1", params![id])
            .map_err(|e| err(format!("Failed to delete the task. {e}")))?;
        self.publish(json!({"type": "todo-deleted", "todoId": id}));
        Ok(Value::Null)
    }

    fn update(&self, payload: Value) -> Result<Value, WsErr> {
        let Some(obj) = payload.as_object() else {
            return Err(err("Invalid task update payload."));
        };
        let id = opt_str(obj.get("id")).ok_or_else(|| err("Task id is required."))?;
        let conn = self.conn.lock().map_err(|_| err("store poisoned"))?;
        for _ in 0..3 {
            let Some(current) = Self::get_by_id(&conn, &id)
                .map_err(|e| err(format!("Failed to update the task. {e}")))?
            else {
                return Err(err("This task no longer exists."));
            };
            if let Some(expected) = obj.get("expectedThreadId") {
                if current.get("threadId") != Some(expected) {
                    return Err(err(
                        "This task was delegated somewhere else in the meantime.",
                    ));
                }
            }
            let updated_at = next_todo_updated_at(
                current
                    .get("updatedAt")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            );
            // applyTodoPatch (packages/shared/src/todo.ts)
            let mut next = current.clone();
            let m = next.as_object_mut().unwrap();
            let has = |k: &str| obj.get(k).is_some();
            if has("title") {
                if let Some(t) = opt_str(obj.get("title"))
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                {
                    m.insert("title".into(), Value::String(t));
                }
            }
            for k in [
                "notes",
                "priority",
                "projectId",
                "dueDate",
                "threadId",
                "delegationBaseTurnId",
            ] {
                if has(k) {
                    m.insert(k.into(), obj[k].clone());
                }
            }
            let old_thread = current.get("threadId").cloned().unwrap_or(Value::Null);
            let new_thread = m.get("threadId").cloned().unwrap_or(Value::Null);
            if has("threadId") && new_thread != old_thread {
                m.insert(
                    "linkedAt".into(),
                    if new_thread.is_null() {
                        Value::Null
                    } else {
                        json!(updated_at)
                    },
                );
                if !has("delegationBaseTurnId") {
                    m.insert("delegationBaseTurnId".into(), Value::Null);
                }
            }
            let completed_at = match obj.get("completed").and_then(|v| v.as_bool()) {
                None => current.get("completedAt").cloned().unwrap_or(Value::Null),
                Some(true) => {
                    if current.get("completedAt").is_some_and(|v| !v.is_null()) {
                        current["completedAt"].clone()
                    } else {
                        json!(updated_at)
                    }
                }
                Some(false) => Value::Null,
            };
            m.insert("completedAt".into(), completed_at);
            m.insert("updatedAt".into(), json!(updated_at));
            if let Some(t) = m.get_mut("threadId") {
                if t.is_string() == false && !t.is_null() {
                    *t = Value::Null;
                }
            }

            let links_newly = new_thread != old_thread;
            let reopens = !current["completedAt"].is_null() && m["completedAt"].is_null();
            if !m["threadId"].is_null() && m["completedAt"].is_null() && (links_newly || reopens) {
                let owners = Self::list_all(&conn).map_err(|e| err(e))?;
                let owned_elsewhere = owners.iter().any(|t| {
                    t["id"] != json!(id)
                        && t["threadId"] == m["threadId"]
                        && t["completedAt"].is_null()
                });
                if owned_elsewhere && links_newly {
                    return Err(err("That chat is already working on another task."));
                }
                if owned_elsewhere {
                    m.insert("threadId".into(), Value::Null);
                    m.insert("delegationBaseTurnId".into(), Value::Null);
                    m.insert("linkedAt".into(), Value::Null);
                }
            }
            let affected = conn.execute(
                "UPDATE todos SET title=?2, notes=?3, priority=?4, project_id=?5, due_date=?6,
                    thread_id=?7, delegation_base_turn_id=?8, linked_at=?9, completed_at=?10,
                    created_at=?11, updated_at=?12
                 WHERE todo_id=?1 AND updated_at=?13",
                params![
                    id,
                    m["title"].as_str().unwrap_or("").to_string(),
                    m["notes"].as_str().unwrap_or("").to_string(),
                    m["priority"].as_str().unwrap_or("none").to_string(),
                    m["projectId"].as_str().map(|s| s.to_string()),
                    m["dueDate"].as_str().map(|s| s.to_string()),
                    m["threadId"].as_str().map(|s| s.to_string()),
                    m["delegationBaseTurnId"].as_str().map(|s| s.to_string()),
                    m["linkedAt"].as_str().map(|s| s.to_string()),
                    m["completedAt"].as_str().map(|s| s.to_string()),
                    m["createdAt"].as_str().map(|s| s.to_string()),
                    m["updatedAt"].as_str().map(|s| s.to_string()),
                    current["updatedAt"].as_str().unwrap_or("").to_string(),
                ],
            );
            match affected {
                Ok(1) => {
                    let todo = Self::get_by_id(&conn, &id)
                        .map_err(|e| err(format!("Failed to update the task. {e}")))?
                        .unwrap_or(Value::Null);
                    self.publish(json!({"type": "todo-upserted", "todo": todo}));
                    return Ok(todo);
                }
                Ok(_) => continue,
                Err(e)
                    if e.to_string().contains("idx_todos_open_thread")
                        || e.to_string().contains("todos.thread_id") =>
                {
                    if links_newly {
                        return Err(err("That chat is already working on another task."));
                    }
                    continue;
                }
                Err(e) => return Err(err(format!("Failed to update the task. {e}"))),
            }
        }
        Err(err("The task changed while it was being saved. Try again."))
    }
}

/// State dir mirroring `deriveServerPaths`: `<home>/userdata`. The front door
/// resolves `--home-dir`/`SYNARA_HOME` the same way the child gets it
/// (`.rec-home` is the replay default baked into `original::RUN`).
pub fn state_dir() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut home: Option<String> = std::env::var("SYNARA_HOME").ok();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--home-dir" && i + 1 < args.len() {
            home = Some(args[i + 1].clone());
            i += 1;
        } else if let Some(v) = args[i].strip_prefix("--home-dir=") {
            home = Some(v.to_string());
        }
        i += 1;
    }
    let home = home.unwrap_or_else(|| "./.rec-home".to_string());
    // `--home-dir` is relative to the repo root, same as the child's cwd.
    let home = if std::path::Path::new(&home).is_relative() {
        format!("{}/{home}", concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    } else {
        home
    };
    format!("{home}/userdata")
}
