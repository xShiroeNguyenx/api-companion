//! Tauri shell của API Companion.
//!
//! Nguyên tắc: crate này CHỈ wiring — nhận lệnh từ frontend, gọi các crate lõi,
//! trả kết quả. Không chứa logic nghiệp vụ.

mod ai;
mod commands;
mod ops;
mod postman;
mod share;
mod teamws;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ipc_types::WorkspaceKind;
use tauri::Manager;
use tokio_util::sync::CancellationToken;

/// State dùng chung toàn app.
pub struct AppState {
    /// Kết nối SQLite cho history + registry workspace.
    pub db: Mutex<storage::SqliteConnection>,
    /// Các request đang chạy, map theo request_id → token hủy.
    pub inflight: Mutex<HashMap<String, CancellationToken>>,
    /// Thư mục gốc workspace ĐANG active (cache; nội dung là file TOML).
    pub workspace_root: Mutex<PathBuf>,
    /// Id workspace đang active trong registry (dùng cho secret scope + FE match).
    pub active_workspace_id: Mutex<String>,
    /// Khoá tuần tự hoá sync team workspace (tokio Mutex — giữ được qua await).
    pub team_sync: tokio::sync::Mutex<()>,
}

/// Tên hiển thị từ đường dẫn (segment cuối), fallback khi không có tên khác.
pub(crate) fn folder_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Workspace".to_string())
}

/// Suy ra tên workspace: ưu tiên `workspace.toml.name` (nếu có ý nghĩa), else tên thư mục.
pub(crate) fn derive_ws_name(root: &Path) -> String {
    match workspace::info(root).ok().map(|i| i.name) {
        Some(n) if !n.trim().is_empty() && n != "My Workspace" => n,
        _ => folder_name(root),
    }
}

/// Xác định workspace để mở lúc boot; đảm bảo registry có ≥1 workspace + active.
///
/// Tách riêng để unit-test được với `storage::init_in_memory`. Trả `(root, active_id)`.
pub(crate) fn resolve_boot_workspace(
    conn: &mut storage::SqliteConnection,
    default_root: &Path,
) -> (PathBuf, String) {
    // 1. Đã có active? dùng nó. Chưa có → seed từ legacy "workspace.path" hoặc default.
    let chosen = match storage::get_active_workspace(conn).ok().flatten() {
        Some(w) => w,
        None => {
            let legacy = storage::get_setting(conn, "workspace.path").ok().flatten();
            let (name, norm) = match legacy {
                Some(p) if !p.is_empty() => {
                    let pb = PathBuf::from(&p);
                    (derive_ws_name(&pb), workspace::normalize_root(&pb))
                }
                _ => ("Personal".to_string(), workspace::normalize_root(default_root)),
            };
            match storage::upsert_workspace_by_path(conn, &name, &norm, WorkspaceKind::Personal, None)
            {
                Ok(w) => {
                    let _ = storage::set_active_workspace(conn, &w.id);
                    w
                }
                // DB lỗi bất thường → chạy không registry (id rỗng), vẫn mở được app.
                Err(_) => return (default_root.to_path_buf(), String::new()),
            }
        }
    };

    // 2. Ensure path đã chọn; nếu offline/lỗi → fallback default (GIỮ metadata cũ).
    let chosen_path = PathBuf::from(&chosen.path);
    if workspace::ensure(&chosen_path).is_ok() {
        return (chosen_path, chosen.id);
    }
    let _ = workspace::ensure(default_root);
    let norm = workspace::normalize_root(default_root);
    match storage::upsert_workspace_by_path(conn, "Personal", &norm, WorkspaceKind::Personal, None) {
        Ok(fallback) => {
            let _ = storage::set_active_workspace(conn, &fallback.id);
            (default_root.to_path_buf(), fallback.id)
        }
        Err(_) => (default_root.to_path_buf(), chosen.id),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Auto-update (desktop-only): check/download qua GitHub Releases latest.json,
            // artifact được ký minisign — xem docs/RELEASE.md §4.
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;

            // History DB + registry workspace.
            let db_path = data.join("history.sqlite");
            let mut conn =
                storage::init(&db_path).map_err(|e| format!("Không mở được database: {e}"))?;

            // Workspace: seed/khôi phục từ registry (migrate legacy "workspace.path" lần đầu).
            let default_root = data.join("workspace");
            let (ws_root, active_id) = resolve_boot_workspace(&mut conn, &default_root);

            app.manage(AppState {
                db: Mutex::new(conn),
                inflight: Mutex::new(HashMap::new()),
                workspace_root: Mutex::new(ws_root),
                active_workspace_id: Mutex::new(active_id),
                team_sync: tokio::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_request,
            commands::resolve_preview,
            commands::cancel_request,
            commands::list_history,
            commands::load_history_record,
            commands::clear_history,
            commands::import_curl,
            commands::export_curl,
            commands::workspace_info,
            commands::set_workspace,
            commands::list_workspaces,
            commands::add_workspace,
            commands::set_active_workspace,
            commands::update_workspace,
            commands::remove_workspace,
            commands::save_tab_session,
            commands::load_tab_session,
            commands::create_collection,
            commands::create_folder,
            commands::duplicate_node,
            commands::add_request,
            commands::save_request,
            commands::load_request,
            commands::delete_node,
            commands::list_environments,
            commands::load_environment,
            commands::save_environment,
            commands::set_active_environment,
            commands::delete_environment,
            postman::import_postman,
            postman::import_postman_files,
            postman::import_postman_dir,
            postman::import_postman_api,
            ai::ai_get_settings,
            ai::ai_set_provider,
            ai::ai_set_model,
            ai::ai_set_key,
            ai::ai_test_connection,
            ai::ai_generate_request,
            ai::ai_explain,
            ai::ai_diagnose,
            ai::ai_generate_tests,
            commands::run_assertions,
            commands::run_collection,
            commands::list_codegen_targets,
            commands::generate_code,
            ops::list_connections,
            ops::save_connection,
            ops::delete_connection,
            ops::test_connection,
            ops::db_query,
            ops::ssh_exec,
            share::export_bundle,
            share::export_postman,
            teamws::team_ws_test,
            teamws::team_ws_add,
            teamws::team_ws_sync,
        ])
        .run(tauri::generate_context!())
        .expect("lỗi khi chạy API Companion");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_seeds_default_when_empty() {
        let mut conn = storage::init_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let default_root = tmp.path().join("workspace");
        let (root, id) = resolve_boot_workspace(&mut conn, &default_root);
        assert_eq!(root, default_root);
        assert!(!id.is_empty());
        let active = storage::get_active_workspace(&conn).unwrap().unwrap();
        assert_eq!(active.id, id);
        assert_eq!(storage::count_workspaces(&conn).unwrap(), 1);
    }

    #[test]
    fn boot_seeds_from_legacy_setting() {
        let mut conn = storage::init_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let legacy = tmp.path().join("my-legacy-ws");
        std::fs::create_dir_all(&legacy).unwrap();
        storage::set_setting(&conn, "workspace.path", legacy.to_str().unwrap()).unwrap();
        let default_root = tmp.path().join("workspace");
        let (root, id) = resolve_boot_workspace(&mut conn, &default_root);
        assert_eq!(root, legacy);
        let active = storage::get_active_workspace(&conn).unwrap().unwrap();
        assert_eq!(active.id, id);
        assert_eq!(active.path, workspace::normalize_root(&legacy));
    }

    #[test]
    fn boot_reuses_existing_active() {
        let mut conn = storage::init_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let default_root = tmp.path().join("workspace");
        let (_r1, id1) = resolve_boot_workspace(&mut conn, &default_root);
        let (_r2, id2) = resolve_boot_workspace(&mut conn, &default_root);
        assert_eq!(id1, id2);
        assert_eq!(storage::count_workspaces(&conn).unwrap(), 1);
    }
}
