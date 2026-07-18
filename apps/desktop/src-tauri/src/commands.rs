//! Tauri commands — điểm vào IPC từ frontend.
//!
//! Mỗi command mỏng: parse input → gọi crate lõi → trả DTO từ `ipc-types`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ipc_types::{
    AssertionResult, Auth, CodegenTarget, CodegenTargetInfo, Environment, ExchangeRecord,
    HistoryEntry, NodeKind, RequestSpec, ResolvePreview, RunResult, SavedRequest, TreeNode,
    WorkspaceInfo, WorkspaceKind, WorkspaceMeta,
};
use tauri::State;
use tokio_util::sync::CancellationToken;

use crate::AppState;

fn ws_root(state: &State<'_, AppState>) -> PathBuf {
    state.workspace_root.lock().unwrap().clone()
}

/// Id workspace đang active — dùng làm scope cho secret (namespace theo workspace).
fn active_ws_id(state: &State<'_, AppState>) -> String {
    state.active_workspace_id.lock().unwrap().clone()
}

// ---------------------------------------------------------------------------
// Gửi request (resolve biến + inherit + secrets) — M1
// ---------------------------------------------------------------------------

/// Gộp biến của một environment thành map key→value (secret lấy từ keychain).
fn env_values(scope: &str, root: &Path, env_name: Option<&str>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(name) = env_name {
        if let Ok(env) = workspace::load_environment(root, name) {
            for v in &env.variables {
                if v.is_secret {
                    if let Ok(Some(val)) = secrets::get_scoped_or_legacy(scope, name, &v.key) {
                        map.insert(v.key.clone(), val);
                    }
                } else {
                    map.insert(v.key.clone(), v.value.clone());
                }
            }
        }
    }
    map
}

/// Dựng spec đã resolve từ spec gốc + environment + collection (defaults + biến).
fn resolve(
    scope: &str,
    root: &Path,
    spec: &RequestSpec,
    environment: Option<&str>,
    collection_id: Option<&str>,
) -> (RequestSpec, Vec<String>) {
    let global = workspace::global_variables(root).unwrap_or_default();
    let (default_auth, default_headers, coll_vars) = match collection_id {
        Some(cid) => workspace::collection_defaults(root, cid),
        None => (Auth::Inherit, Vec::new(), Vec::new()),
    };
    let env_name = environment
        .map(|s| s.to_string())
        .or_else(|| workspace::active_environment(root).ok().flatten());
    let env_map = env_values(scope, root, env_name.as_deref());
    let merged = workspace::merge_vars(&global, &coll_vars, &env_map);

    let mut prepared = spec.clone();
    workspace::vars::apply_defaults(&mut prepared, &default_auth, &default_headers);
    // Dùng smart-vars: hỗ trợ {{uuid.v7}}, {{today+7}}, {{faker.*}}, {{jwt(x).exp}}, {{otp}}...
    smart_vars::resolve_spec(&prepared, &merged)
}

/// Gửi request (có thể hủy) — resolve biến/inherit theo environment + collection, rồi lưu history.
#[tauri::command]
pub async fn send_request(
    state: State<'_, AppState>,
    spec: RequestSpec,
    request_id: String,
    environment: Option<String>,
    collection_id: Option<String>,
) -> Result<ExchangeRecord, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let (resolved, _unresolved) =
        resolve(&scope, &root, &spec, environment.as_deref(), collection_id.as_deref());

    let token = CancellationToken::new();
    {
        let mut map = state.inflight.lock().map_err(|e| e.to_string())?;
        map.insert(request_id.clone(), token.clone());
    }
    let record = http_engine::send_with_cancel(&resolved, &token).await;
    if let Ok(mut map) = state.inflight.lock() {
        map.remove(&request_id);
    }
    // Lưu history với spec GỐC (giữ template {{var}}), record thực tế.
    if let Ok(conn) = state.db.lock() {
        if let Err(e) = storage::save_exchange(&conn, &spec, &record) {
            eprintln!("[history] lưu thất bại: {e}");
        }
    }
    Ok(record)
}

/// Xem trước kết quả resolve (cho UI highlight biến chưa resolve).
#[tauri::command]
pub fn resolve_preview(
    state: State<'_, AppState>,
    spec: RequestSpec,
    environment: Option<String>,
    collection_id: Option<String>,
) -> ResolvePreview {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let (resolved, unresolved) =
        resolve(&scope, &root, &spec, environment.as_deref(), collection_id.as_deref());
    ResolvePreview {
        resolved_url: resolved.url,
        unresolved,
    }
}

/// Hủy request đang chạy theo `request_id`.
#[tauri::command]
pub fn cancel_request(state: State<'_, AppState>, request_id: String) {
    if let Ok(mut map) = state.inflight.lock() {
        if let Some(token) = map.remove(&request_id) {
            token.cancel();
        }
    }
}

// ---------------------------------------------------------------------------
// Code generation (F) — sinh snippet request cho nhiều ngôn ngữ
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_codegen_targets() -> Vec<CodegenTargetInfo> {
    codegen::targets()
}

#[tauri::command]
pub fn generate_code(spec: RequestSpec, target: CodegenTarget) -> String {
    codegen::generate(&spec, target)
}

// ---------------------------------------------------------------------------
// Assertions runner (M3)
// ---------------------------------------------------------------------------

/// Chạy assertions trên một record đã có (frontend gọi sau khi Send).
#[tauri::command]
pub fn run_assertions(
    record: ExchangeRecord,
    assertions: Vec<ipc_types::Assertion>,
) -> Vec<AssertionResult> {
    assertions::evaluate(&assertions, &record)
}

fn collect_request_ids_all(nodes: &[TreeNode], out: &mut Vec<String>) {
    for n in nodes {
        match n.kind {
            NodeKind::Request => out.push(n.id.clone()),
            _ => collect_request_ids_all(&n.children, out),
        }
    }
}

/// Chạy toàn bộ request dưới một collection/folder (hoặc chính 1 request).
#[tauri::command]
pub async fn run_collection(
    state: State<'_, AppState>,
    id: String,
    environment: Option<String>,
) -> Result<Vec<RunResult>, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let info = workspace::info(&root).map_err(|e| e.to_string())?;
    let mut all = Vec::new();
    collect_request_ids_all(&info.tree, &mut all);
    let targets: Vec<String> = all
        .into_iter()
        .filter(|rid| *rid == id || rid.starts_with(&format!("{id}/")))
        .collect();

    let mut results = Vec::new();
    for rid in targets {
        let Ok(saved) = workspace::load_request(&root, &rid) else { continue };
        let cid = workspace::collection_root_of(&rid);
        let (resolved, _un) =
            resolve(&scope, &root, &saved.spec, environment.as_deref(), cid.as_deref());
        let record = http_engine::send(&resolved).await;
        let ar = assertions::evaluate(&saved.spec.assertions, &record);
        let passed = record.error.is_none() && assertions::all_passed(&ar);
        results.push(RunResult {
            request_id: rid,
            name: saved.name,
            method: record.method.clone(),
            url: record.final_url.clone(),
            status: record.response.as_ref().map(|r| r.status),
            total_ms: record.timings.total_ms,
            error: record.error.as_ref().map(|e| e.message.clone()),
            assertions: ar,
            passed,
        });
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_history(state: State<'_, AppState>, limit: u32) -> Result<Vec<HistoryEntry>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::list_history(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_history_record(state: State<'_, AppState>, id: i64) -> Result<Option<ExchangeRecord>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::load_record(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::clear_history(&conn).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// cURL
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn import_curl(command: String) -> Result<RequestSpec, String> {
    curl_tools::parse(&command).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_curl(spec: RequestSpec) -> String {
    curl_tools::to_curl(&spec)
}

// ---------------------------------------------------------------------------
// Workspace / collections (M1)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn workspace_info(state: State<'_, AppState>) -> Result<WorkspaceInfo, String> {
    workspace::info(&ws_root(&state)).map_err(|e| e.to_string())
}

/// Đổi thư mục workspace (backward-compat của luồng "Mở thư mục khác").
/// Cài lại trên nền registry: upsert theo path rồi activate.
#[tauri::command]
pub fn set_workspace(state: State<'_, AppState>, path: String) -> Result<WorkspaceInfo, String> {
    let p = PathBuf::from(&path);
    workspace::ensure(&p).map_err(|e| format!("Không mở được workspace: {e}"))?;
    let norm = workspace::normalize_root(&p);
    let name = crate::derive_ws_name(&p);
    let id = {
        let mut guard = state.db.lock().map_err(|e| e.to_string())?;
        let meta =
            storage::upsert_workspace_by_path(&guard, &name, &norm, WorkspaceKind::Personal, None)
                .map_err(|e| e.to_string())?;
        storage::set_active_workspace(&mut guard, &meta.id).map_err(|e| e.to_string())?;
        let _ = storage::set_setting(&guard, "workspace.path", &path); // rollback-safe legacy
        meta.id
    };
    *state.workspace_root.lock().map_err(|e| e.to_string())? = p.clone();
    *state.active_workspace_id.lock().map_err(|e| e.to_string())? = id;
    workspace::info(&p).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Registry đa-workspace (v4)
// ---------------------------------------------------------------------------

fn decorate(mut list: Vec<WorkspaceMeta>, active_id: &str) -> Vec<WorkspaceMeta> {
    for w in list.iter_mut() {
        w.is_active = w.id == active_id;
        w.available = Path::new(&w.path).exists();
    }
    list
}

/// Danh sách mọi workspace trong registry (kèm cờ active + available runtime).
#[tauri::command]
pub fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<WorkspaceMeta>, String> {
    let active_id = state.active_workspace_id.lock().map_err(|e| e.to_string())?.clone();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let list = storage::list_workspaces(&conn).map_err(|e| e.to_string())?;
    Ok(decorate(list, &active_id))
}

/// Thêm một thư mục workspace vào registry (KHÔNG activate). Idempotent theo path.
#[tauri::command]
pub fn add_workspace(
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
    kind: Option<WorkspaceKind>,
    color: Option<String>,
) -> Result<WorkspaceMeta, String> {
    let p = PathBuf::from(&path);
    let _ = workspace::ensure(&p); // best-effort: tạo cấu trúc nếu folder tạo được
    let norm = workspace::normalize_root(&p);
    let display_name = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| crate::derive_ws_name(&p));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::upsert_workspace_by_path(
        &conn,
        &display_name,
        &norm,
        kind.unwrap_or(WorkspaceKind::Personal),
        color.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Đặt workspace `id` làm active + trả `WorkspaceInfo` (cùng shape set_workspace).
#[tauri::command]
pub fn set_active_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<WorkspaceInfo, String> {
    let path = {
        let mut guard = state.db.lock().map_err(|e| e.to_string())?;
        let w = storage::get_workspace(&guard, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Workspace không tồn tại".to_string())?;
        let pb = PathBuf::from(&w.path);
        // Kiểm tra tồn tại TRƯỚC ensure để không âm thầm tạo lại folder đã bị xoá.
        if !pb.exists() {
            return Err(format!("Thư mục workspace không còn tồn tại: {}", w.path));
        }
        workspace::ensure(&pb).map_err(|e| format!("Không mở được workspace: {e}"))?;
        storage::set_active_workspace(&mut guard, &id).map_err(|e| e.to_string())?;
        let _ = storage::set_setting(&guard, "workspace.path", &w.path);
        pb
    };
    *state.workspace_root.lock().map_err(|e| e.to_string())? = path.clone();
    *state.active_workspace_id.lock().map_err(|e| e.to_string())? = id;
    workspace::info(&path).map_err(|e| e.to_string())
}

/// Cập nhật nhãn workspace (tên/kind/màu). FE gửi trọn giá trị mong muốn.
#[tauri::command]
pub fn update_workspace(
    state: State<'_, AppState>,
    id: String,
    name: String,
    kind: WorkspaceKind,
    color: Option<String>,
) -> Result<WorkspaceMeta, String> {
    let active_id = state.active_workspace_id.lock().map_err(|e| e.to_string())?.clone();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::update_workspace(&conn, &id, Some(&name), Some(kind), Some(color.as_deref()))
        .map_err(|e| e.to_string())?;
    let mut w = storage::get_workspace(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Workspace không tồn tại".to_string())?;
    w.is_active = w.id == active_id;
    w.available = Path::new(&w.path).exists();
    Ok(w)
}

/// Gỡ workspace khỏi registry (KHÔNG xoá file). Chặn gỡ workspace active / workspace cuối.
#[tauri::command]
pub fn remove_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<WorkspaceMeta>, String> {
    let active_id = state.active_workspace_id.lock().map_err(|e| e.to_string())?.clone();
    if id == active_id {
        return Err("Không thể gỡ workspace đang mở — hãy chuyển sang workspace khác trước.".into());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if storage::count_workspaces(&conn).map_err(|e| e.to_string())? <= 1 {
        return Err("Phải còn ít nhất một workspace.".into());
    }
    storage::remove_workspace(&conn, &id).map_err(|e| e.to_string())?;
    let list = storage::list_workspaces(&conn).map_err(|e| e.to_string())?;
    Ok(decorate(list, &active_id))
}

// ---------------------------------------------------------------------------
// Session tab per-workspace (persist & restore) — lưu trong SQLite settings
// ---------------------------------------------------------------------------

fn tab_session_key(workspace_id: &str) -> String {
    format!("session.tabs.{workspace_id}")
}

#[tauri::command]
pub fn save_tab_session(
    state: State<'_, AppState>,
    workspace_id: String,
    json: String,
) -> Result<(), String> {
    if workspace_id.is_empty() {
        return Ok(()); // chưa có workspace active → bỏ qua
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::set_setting(&conn, &tab_session_key(&workspace_id), &json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_tab_session(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Option<String>, String> {
    if workspace_id.is_empty() {
        return Ok(None);
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    storage::get_setting(&conn, &tab_session_key(&workspace_id)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_collection(state: State<'_, AppState>, name: String) -> Result<String, String> {
    workspace::create_collection(&ws_root(&state), &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_folder(state: State<'_, AppState>, parent_id: String, name: String) -> Result<String, String> {
    workspace::create_folder(&ws_root(&state), &parent_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_request(
    state: State<'_, AppState>,
    target_id: String,
    name: String,
    spec: RequestSpec,
) -> Result<String, String> {
    workspace::save_request(&ws_root(&state), &target_id, &name, &spec).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_request(state: State<'_, AppState>, id: String) -> Result<SavedRequest, String> {
    workspace::load_request(&ws_root(&state), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_node(state: State<'_, AppState>, id: String) -> Result<(), String> {
    workspace::delete_node(&ws_root(&state), &id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Environments (M1) — secret nằm ở keychain, không trong file
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_environments(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    workspace::list_environment_names(&ws_root(&state)).map_err(|e| e.to_string())
}

/// Load environment kèm giá trị secret (đọc từ keychain) để hiển thị/sửa.
#[tauri::command]
pub fn load_environment(state: State<'_, AppState>, name: String) -> Result<Environment, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let mut env = workspace::load_environment(&root, &name).map_err(|e| e.to_string())?;
    for v in env.variables.iter_mut() {
        if v.is_secret {
            if let Ok(Some(val)) = secrets::get_scoped_or_legacy(&scope, &name, &v.key) {
                v.value = val;
            }
        }
    }
    Ok(env)
}

/// Lưu environment: secret → keychain (scoped theo workspace), file chỉ giữ tên (value rỗng).
#[tauri::command]
pub fn save_environment(state: State<'_, AppState>, env: Environment) -> Result<(), String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let mut stripped = env.clone();
    for v in stripped.variables.iter_mut() {
        if v.is_secret {
            secrets::set_scoped(&scope, &env.name, &v.key, &v.value).map_err(|e| e.to_string())?;
            v.value = String::new();
        }
    }
    workspace::save_environment(&root, &stripped).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_environment(state: State<'_, AppState>, name: Option<String>) -> Result<(), String> {
    workspace::set_active_environment(&ws_root(&state), name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_environment(state: State<'_, AppState>, name: String) -> Result<(), String> {
    workspace::delete_environment(&ws_root(&state), &name).map_err(|e| e.to_string())
}

// Postman import: xem module `crate::postman`.
