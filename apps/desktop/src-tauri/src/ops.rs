//! Tauri commands cho Ops Workspace (P2-M1): connections + DB query + SSH exec.
//! Secret (DB password / SSH password) nằm ở keychain, không trong file.

use std::path::PathBuf;

use ipc_types::{Connection, ConnectionKind, DbQueryResult, SshResult};
use tauri::State;

use crate::AppState;

const CONN_KEYCHAIN: &str = "__conn__";

fn ws_root(state: &State<'_, AppState>) -> PathBuf {
    state.workspace_root.lock().unwrap().clone()
}

fn active_ws_id(state: &State<'_, AppState>) -> String {
    state.active_workspace_id.lock().unwrap().clone()
}

fn conn_secret(scope: &str, id: &str) -> Option<String> {
    secrets::get_scoped_or_legacy(scope, CONN_KEYCHAIN, id).ok().flatten()
}

/// Percent-encode userinfo (user/password) cho URL kết nối DB.
fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn db_url(conn: &Connection, password: Option<&str>) -> String {
    let driver = conn.db_driver.as_deref().unwrap_or("postgres");
    let db = conn.database.clone().unwrap_or_default();
    if driver == "sqlite" {
        return format!("sqlite://{db}");
    }
    let userinfo = match password {
        Some(pw) if !pw.is_empty() => format!("{}:{}", enc(&conn.username), enc(pw)),
        _ => enc(&conn.username),
    };
    format!("{driver}://{userinfo}@{}:{}/{}", conn.host, conn.port, db)
}

// ---------------------------------------------------------------------------
// Connections CRUD
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_connections(state: State<'_, AppState>) -> Result<Vec<Connection>, String> {
    let scope = active_ws_id(&state);
    let mut conns = workspace::list_connections(&ws_root(&state)).map_err(|e| e.to_string())?;
    for c in conns.iter_mut() {
        c.has_secret = conn_secret(&scope, &c.id).is_some();
    }
    Ok(conns)
}

#[tauri::command]
pub fn save_connection(
    state: State<'_, AppState>,
    conn: Connection,
    secret: Option<String>,
) -> Result<(), String> {
    let scope = active_ws_id(&state);
    let mut c = conn;
    if let Some(sec) = secret {
        if sec.is_empty() {
            let _ = secrets::delete_scoped(&scope, CONN_KEYCHAIN, &c.id);
            c.has_secret = false;
        } else {
            secrets::set_scoped(&scope, CONN_KEYCHAIN, &c.id, &sec).map_err(|e| e.to_string())?;
            c.has_secret = true;
        }
    }
    workspace::save_connection(&ws_root(&state), &c).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_connection(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let scope = active_ws_id(&state);
    let _ = secrets::delete_scoped(&scope, CONN_KEYCHAIN, &id);
    workspace::delete_connection(&ws_root(&state), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_connection(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let conn = workspace::load_connection(&ws_root(&state), &id).map_err(|e| e.to_string())?;
    let secret = conn_secret(&active_ws_id(&state), &id);
    match conn.kind {
        ConnectionKind::Db => {
            let r = ops_db::query(&db_url(&conn, secret.as_deref()), "SELECT 1").await;
            match r.error {
                None => Ok("OK — kết nối DB thành công".into()),
                Some(e) => Err(e),
            }
        }
        ConnectionKind::Ssh => {
            let r = ops_ssh::exec(&conn, secret.as_deref(), "echo apic-ok").await;
            if let Some(e) = r.error {
                Err(e)
            } else if r.stdout.contains("apic-ok") {
                Ok("OK — SSH chạy lệnh thành công".into())
            } else {
                Err(format!("SSH trả về bất thường (exit {:?}): {}", r.exit_code, r.stderr))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DB query / SSH exec
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn db_query(state: State<'_, AppState>, connection_id: String, sql: String) -> Result<DbQueryResult, String> {
    let conn = workspace::load_connection(&ws_root(&state), &connection_id).map_err(|e| e.to_string())?;
    if conn.kind != ConnectionKind::Db {
        return Err("Connection không phải kiểu Database".into());
    }
    let secret = conn_secret(&active_ws_id(&state), &connection_id);
    Ok(ops_db::query(&db_url(&conn, secret.as_deref()), &sql).await)
}

#[tauri::command]
pub async fn ssh_exec(state: State<'_, AppState>, connection_id: String, command: String) -> Result<SshResult, String> {
    let conn = workspace::load_connection(&ws_root(&state), &connection_id).map_err(|e| e.to_string())?;
    if conn.kind != ConnectionKind::Ssh {
        return Err("Connection không phải kiểu SSH".into());
    }
    let secret = conn_secret(&active_ws_id(&state), &connection_id);
    Ok(ops_ssh::exec(&conn, secret.as_deref(), &command).await)
}
