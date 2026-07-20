//! Tauri commands cho Team Workspace (MySQL) — team tự dựng MySQL server,
//! mọi thành viên nhập thông tin kết nối là dùng chung một workspace.
//!
//! Nội dung workspace mirror vào thư mục cache local (TOML chuẩn — mọi command
//! file-based dùng nguyên vẹn); `workspace-sync` lo đồng bộ hai chiều.
//! Password MySQL nằm trong OS keychain (scope theo workspace id), KHÔNG trong file/DB.

use std::path::PathBuf;
use std::time::Duration;

use ipc_types::{RemoteDbConfig, WorkspaceKind, WorkspaceMeta, WsSyncReport};
use tauri::{Manager, State};

use crate::AppState;

/// "env" giả trong keychain cho password MySQL của team workspace.
const TEAM_KEYCHAIN_ENV: &str = "__team_db__";
const TEAM_KEYCHAIN_KEY: &str = "mysql";

const DEFAULT_DB: &str = "apic_workspace";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const SYNC_TIMEOUT: Duration = Duration::from_secs(90);

fn slug(name: &str) -> String {
    let mut s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "team".to_string()
    } else {
        s
    }
}

fn opt_pw(password: &str) -> Option<&str> {
    if password.is_empty() {
        None
    } else {
        Some(password)
    }
}

/// Thử kết nối tới MySQL server (chưa cần database tồn tại).
#[tauri::command]
pub async fn team_ws_test(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<String, String> {
    let url = workspace_sync::server_url(host.trim(), port, username.trim(), opt_pw(&password));
    tokio::time::timeout(CONNECT_TIMEOUT, workspace_sync::test_server(&url))
        .await
        .map_err(|_| format!("Timeout {}s khi kết nối MySQL", CONNECT_TIMEOUT.as_secs()))?
        .map_err(|e| e.to_string())?;
    Ok("OK — kết nối MySQL thành công".into())
}

/// Tạo/tham gia team workspace: init database riêng trên server (idempotent),
/// tạo thư mục mirror local, đăng ký registry, lưu password vào keychain,
/// rồi sync lần đầu (kéo nội dung sẵn có của team về). KHÔNG activate —
/// frontend gọi `set_active_workspace` sau đó như mọi workspace khác.
#[tauri::command]
pub async fn team_ws_add(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    database: String,
) -> Result<WorkspaceMeta, String> {
    let host = host.trim().to_string();
    let username = username.trim().to_string();
    let db = {
        let d = database.trim();
        if d.is_empty() { DEFAULT_DB.to_string() } else { d.to_string() }
    };
    if host.is_empty() {
        return Err("Host không được để trống".into());
    }
    if !workspace_sync::valid_db_name(&db) {
        return Err(format!(
            "Tên database không hợp lệ: \"{db}\" — chỉ chữ/số/underscore (VD: {DEFAULT_DB})"
        ));
    }

    // Dedupe: đã có workspace trỏ cùng server+database → trả về workspace cũ.
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let list = storage::list_workspaces(&conn).map_err(|e| e.to_string())?;
        if let Some(w) = list.into_iter().find(|w| {
            w.remote
                .as_ref()
                .map(|r| r.host == host && r.port == port && r.database == db)
                .unwrap_or(false)
        }) {
            return Ok(w);
        }
    }

    let display_name = {
        let n = name.trim();
        if n.is_empty() { db.clone() } else { n.to_string() }
    };

    // 1. Init phía server: CREATE DATABASE/TABLE IF NOT EXISTS — không đụng DB khác.
    let url = workspace_sync::server_url(&host, port, &username, opt_pw(&password));
    tokio::time::timeout(CONNECT_TIMEOUT, workspace_sync::init_remote(&url, &db, &display_name))
        .await
        .map_err(|_| format!("Timeout {}s khi kết nối MySQL", CONNECT_TIMEOUT.as_secs()))?
        .map_err(|e| format!("Khởi tạo database thất bại: {e}"))?;

    // 2. Thư mục mirror local (deterministic theo db+host → re-join dùng lại cache cũ).
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = data.join("team-workspaces").join(slug(&format!("{db}-{host}-{port}")));
    workspace::ensure(&dir).map_err(|e| format!("Không tạo được thư mục cache: {e}"))?;
    let norm = workspace::normalize_root(&dir);

    // 3. Đăng ký registry (kind = team, kèm config remote — KHÔNG chứa password).
    let remote_cfg = RemoteDbConfig {
        host: host.clone(),
        port,
        username: username.clone(),
        database: db.clone(),
    };
    let remote_json = serde_json::to_string(&remote_cfg).map_err(|e| e.to_string())?;
    let meta = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        storage::upsert_workspace_full(
            &conn,
            &display_name,
            &norm,
            WorkspaceKind::Team,
            None,
            Some(&remote_json),
        )
        .map_err(|e| e.to_string())?
    };

    // 4. Password → keychain, scope theo workspace id.
    if !password.is_empty() {
        secrets::set_scoped(&meta.id, TEAM_KEYCHAIN_ENV, TEAM_KEYCHAIN_KEY, &password)
            .map_err(|e| format!("Không lưu được password vào keychain: {e}"))?;
    }

    // 5. Sync lần đầu — kéo nội dung team đã có về (hoặc đẩy skeleton nếu DB trống).
    {
        let _guard = state.team_sync.lock().await;
        tokio::time::timeout(SYNC_TIMEOUT, workspace_sync::sync(&dir, &url, &db))
            .await
            .map_err(|_| format!("Sync quá {}s", SYNC_TIMEOUT.as_secs()))?
            .map_err(|e| format!("Sync lần đầu thất bại: {e}"))?;
    }

    Ok(meta)
}

/// Đồng bộ workspace team ĐANG active với MySQL server. Trả báo cáo để UI hiển thị.
#[tauri::command]
pub async fn team_ws_sync(state: State<'_, AppState>) -> Result<WsSyncReport, String> {
    let (root, ws_id, remote) = {
        let id = state.active_workspace_id.lock().map_err(|e| e.to_string())?.clone();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let meta = storage::get_workspace(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Không tìm thấy workspace active".to_string())?;
        let remote = meta
            .remote
            .ok_or_else(|| "Workspace hiện tại không phải team workspace (MySQL)".to_string())?;
        (PathBuf::from(&meta.path), meta.id, remote)
    };

    let pw = secrets::get_scoped(&ws_id, TEAM_KEYCHAIN_ENV, TEAM_KEYCHAIN_KEY)
        .ok()
        .flatten();
    let url =
        workspace_sync::server_url(&remote.host, remote.port, &remote.username, pw.as_deref());

    let _guard = state.team_sync.lock().await;
    tokio::time::timeout(SYNC_TIMEOUT, workspace_sync::sync(&root, &url, &remote.database))
        .await
        .map_err(|_| {
            format!("Sync quá {}s — kiểm tra kết nối tới MySQL server", SYNC_TIMEOUT.as_secs())
        })?
        .map_err(|e| e.to_string())
}
