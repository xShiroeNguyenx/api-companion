//! Import Postman: paste JSON, folder export, nhiều file, hoặc kéo qua Postman API.
//! Tái dùng `postman-import` (thuần) + `workspace`/`secrets`. HTTP qua `http-engine`.

use std::path::{Path, PathBuf};

use ipc_types::{
    Auth, Environment, HttpMethod, ImportSummary, KeyValue, RequestBody, RequestSpec,
};
use postman_import::{ImportedNode, ParsedPostman};
use serde_json::Value;
use tauri::State;

use crate::AppState;

const POSTMAN_API: &str = "https://api.getpostman.com";
const POSTMAN_KEYCHAIN: &str = "__postman__";

fn ws_root(state: &State<'_, AppState>) -> PathBuf {
    state.workspace_root.lock().unwrap().clone()
}

fn active_ws_id(state: &State<'_, AppState>) -> String {
    state.active_workspace_id.lock().unwrap().clone()
}

// ---------------------------------------------------------------------------
// Ghi vào workspace
// ---------------------------------------------------------------------------

fn write_node(root: &Path, parent_id: &str, node: &ImportedNode, summary: &mut ImportSummary) {
    match node {
        ImportedNode::Folder { name, children } => match workspace::create_folder(root, parent_id, name) {
            Ok(fid) => {
                for child in children {
                    write_node(root, &fid, child, summary);
                }
            }
            Err(e) => summary.errors.push(format!("folder '{name}': {e}")),
        },
        ImportedNode::Request { name, spec } => {
            if let Err(e) = workspace::save_request(root, parent_id, name, spec) {
                summary.errors.push(format!("request '{name}': {e}"));
            } else {
                summary.requests += 1;
            }
        }
    }
}

fn import_collection(root: &Path, c: &postman_import::ImportedCollection, summary: &mut ImportSummary) {
    match workspace::create_collection(root, &c.name) {
        Ok(cid) => {
            for node in &c.root {
                write_node(root, &cid, node, summary);
            }
            summary.collections += 1;
        }
        Err(e) => summary.errors.push(format!("collection '{}': {e}", c.name)),
    }
}

fn import_environment(
    scope: &str,
    root: &Path,
    env: &postman_import::ImportedEnvironment,
    summary: &mut ImportSummary,
) {
    let mut e = Environment {
        id: env.name.clone(),
        name: env.name.clone(),
        variables: env.variables.clone(),
    };
    // Secret → keychain (scoped theo workspace), strip khỏi file.
    for v in e.variables.iter_mut() {
        if v.is_secret && !v.value.is_empty() {
            if let Err(err) = secrets::set_scoped(scope, &e.name, &v.key, &v.value) {
                summary.errors.push(format!("secret '{}/{}': {err}", e.name, v.key));
            }
            v.value = String::new();
        }
    }
    if let Err(e2) = workspace::save_environment(root, &e) {
        summary.errors.push(format!("environment '{}': {e2}", e.name));
    } else {
        summary.environments += 1;
    }
}

/// Import một chuỗi JSON: tự nhận diện bundle native → Postman collection → environment.
fn import_json(scope: &str, root: &Path, json: &str, source: &str, summary: &mut ImportSummary) {
    if bundle::detect(json) {
        crate::share::import_bundle_into(root, json, summary);
        return;
    }
    match postman_import::parse_any(json) {
        Ok(ParsedPostman::Collection(c)) => import_collection(root, &c, summary),
        Ok(ParsedPostman::Environment(env)) => import_environment(scope, root, &env, summary),
        Err(e) => summary.errors.push(format!("{source}: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Commands: paste / folder / files
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn import_postman(state: State<'_, AppState>, json: String) -> Result<ImportSummary, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let mut summary = ImportSummary::default();
    import_json(&scope, &root, &json, "paste", &mut summary);
    Ok(summary)
}

#[tauri::command]
pub fn import_postman_files(state: State<'_, AppState>, paths: Vec<String>) -> Result<ImportSummary, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let mut summary = ImportSummary::default();
    for p in &paths {
        match std::fs::read_to_string(p) {
            Ok(json) => import_json(&scope, &root, &json, p, &mut summary),
            Err(e) => summary.errors.push(format!("{p}: {e}")),
        }
    }
    Ok(summary)
}

#[tauri::command]
pub fn import_postman_dir(state: State<'_, AppState>, path: String) -> Result<ImportSummary, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let mut summary = ImportSummary::default();
    let mut files = Vec::new();
    collect_json_files(Path::new(&path), &mut files, 0);
    if files.is_empty() {
        return Err("Không tìm thấy file .json nào trong thư mục".to_string());
    }
    for f in &files {
        match std::fs::read_to_string(f) {
            Ok(json) => import_json(&scope, &root, &json, &f.to_string_lossy(), &mut summary),
            Err(e) => summary.errors.push(format!("{}: {e}", f.display())),
        }
    }
    Ok(summary)
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_json_files(&p, out, depth + 1);
            } else if p.extension().and_then(|e| e.to_str()) == Some("json") {
                out.push(p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Command: Postman API (kéo mọi workspace)
// ---------------------------------------------------------------------------

async fn pm_get(url: &str, api_key: &str) -> Result<Value, String> {
    let spec = RequestSpec {
        method: HttpMethod::new("GET"),
        url: url.to_string(),
        query: Vec::new(),
        headers: vec![KeyValue { key: "X-Api-Key".into(), value: api_key.to_string(), enabled: true }],
        body: RequestBody::None,
        auth: Auth::None,
        timeout_ms: Some(60_000),
        follow_redirects: true,
        max_redirects: 5,
        verify_tls: true,
        assertions: Vec::new(),
    };
    let rec = http_engine::send(&spec).await;
    if let Some(e) = rec.error {
        return Err(e.message);
    }
    let resp = rec.response.ok_or("Không có phản hồi từ Postman API")?;
    let text = resp.body.text.unwrap_or_default();
    if resp.status >= 400 {
        let msg: String = text.chars().take(300).collect();
        return Err(format!("Postman API {}: {}", resp.status, msg));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_postman_api(
    state: State<'_, AppState>,
    api_key: String,
    save_key: bool,
) -> Result<ImportSummary, String> {
    let root = ws_root(&state);
    let scope = active_ws_id(&state);
    let mut summary = ImportSummary::default();

    if save_key {
        // Postman API key: app-global có chủ đích (không theo workspace).
        let _ = secrets::set_secret(POSTMAN_KEYCHAIN, "api_key", &api_key);
    }

    // Collections.
    let list = pm_get(&format!("{POSTMAN_API}/collections"), &api_key).await?;
    if let Some(arr) = list["collections"].as_array() {
        for c in arr {
            if let Some(uid) = c["uid"].as_str() {
                match pm_get(&format!("{POSTMAN_API}/collections/{uid}"), &api_key).await {
                    Ok(full) => {
                        // API bọc trong { "collection": {...} }.
                        let inner = full["collection"].to_string();
                        import_json(&scope, &root, &inner, "postman-api", &mut summary);
                    }
                    Err(e) => summary.errors.push(format!("collection {uid}: {e}")),
                }
            }
        }
    }

    // Environments.
    let envs = pm_get(&format!("{POSTMAN_API}/environments"), &api_key).await?;
    if let Some(arr) = envs["environments"].as_array() {
        for e in arr {
            if let Some(uid) = e["uid"].as_str() {
                match pm_get(&format!("{POSTMAN_API}/environments/{uid}"), &api_key).await {
                    Ok(full) => {
                        let inner = full["environment"].to_string();
                        import_json(&scope, &root, &inner, "postman-api", &mut summary);
                    }
                    Err(e) => summary.errors.push(format!("environment {uid}: {e}")),
                }
            }
        }
    }

    Ok(summary)
}
