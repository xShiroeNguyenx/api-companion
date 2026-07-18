//! Export/Import để chia sẻ collection/workspace giữa người dùng tool.
//! - Native bundle (đầy đủ, gồm assertions) — file .apic.json
//! - Postman v2.1 (interop) — file .postman_collection.json
//! Secret KHÔNG đi kèm (để trống); người nhận tự nhập lại.

use std::path::{Path, PathBuf};

use bundle::{Bundle, BundleCollection, BundleRequest};
use ipc_types::ImportSummary;
use tauri::State;

use crate::AppState;

fn ws_root(state: &State<'_, AppState>) -> PathBuf {
    state.workspace_root.lock().unwrap().clone()
}

/// Export ra native bundle. `collection_id` None = cả workspace.
#[tauri::command]
pub fn export_bundle(
    state: State<'_, AppState>,
    collection_id: Option<String>,
    path: String,
) -> Result<String, String> {
    let root = ws_root(&state);
    let info = workspace::info(&root).map_err(|e| e.to_string())?;

    let mut b = Bundle::new(&info.name);
    let mut req_total = 0u32;

    let nodes: Vec<_> = match &collection_id {
        Some(cid) => info.tree.iter().filter(|n| &n.id == cid).collect(),
        None => info.tree.iter().collect(),
    };
    for node in nodes {
        let (name, description, auth, headers, variables) = workspace::collection_meta(&root, &node.id);
        let reqs = workspace::collection_requests(&root, &node.id).map_err(|e| e.to_string())?;
        req_total += reqs.len() as u32;
        b.collections.push(BundleCollection {
            name,
            description,
            auth,
            headers,
            variables,
            requests: reqs
                .into_iter()
                .map(|(folder, sr)| BundleRequest { path: folder, name: sr.name, spec: sr.spec })
                .collect(),
        });
    }
    if let Some(cid) = &collection_id {
        b.name = b.collections.first().map(|c| c.name.clone()).unwrap_or_else(|| cid.clone());
    }
    // Environment (secret để trống — đọc từ file, không lấy keychain).
    for env_name in &info.environments {
        if let Ok(e) = workspace::load_environment(&root, env_name) {
            b.environments.push(e);
        }
    }

    let json = bundle::to_json(&b)?;
    write_file(&path, &json)?;
    Ok(format!(
        "Đã export {} collection, {} request, {} environment.",
        b.collections.len(),
        req_total,
        b.environments.len()
    ))
}

/// Export một collection ra Postman v2.1 JSON.
#[tauri::command]
pub fn export_postman(
    state: State<'_, AppState>,
    collection_id: String,
    path: String,
) -> Result<String, String> {
    let root = ws_root(&state);
    let (name, _d, _a, _h, _v) = workspace::collection_meta(&root, &collection_id);
    let reqs = workspace::collection_requests(&root, &collection_id).map_err(|e| e.to_string())?;
    let n = reqs.len();
    let triples: Vec<_> = reqs.into_iter().map(|(folder, sr)| (folder, sr.name, sr.spec)).collect();
    let value = postman_import::to_postman_collection(&name, &triples);
    let json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    write_file(&path, &json)?;
    Ok(format!("Đã export {n} request ra Postman v2.1 (assertions không kèm)."))
}

/// Ghi bundle native vào workspace (dùng khi import auto-detect thấy bundle).
pub fn import_bundle_into(root: &Path, json: &str, summary: &mut ImportSummary) {
    let b = match bundle::parse(json) {
        Ok(b) => b,
        Err(e) => {
            summary.errors.push(e);
            return;
        }
    };
    for bc in b.collections {
        match workspace::create_collection(root, &bc.name) {
            Ok(cid) => {
                let _ = workspace::save_collection_meta(
                    root, &cid, &bc.name, bc.description, bc.auth, bc.headers, bc.variables,
                );
                for br in bc.requests {
                    let target = if br.path.is_empty() {
                        cid.clone()
                    } else {
                        format!("{cid}/{}", br.path)
                    };
                    if let Err(e) = workspace::save_request(root, &target, &br.name, &br.spec) {
                        summary.errors.push(format!("request '{}': {e}", br.name));
                    } else {
                        summary.requests += 1;
                    }
                }
                summary.collections += 1;
            }
            Err(e) => summary.errors.push(format!("collection '{}': {e}", bc.name)),
        }
    }
    for env in b.environments {
        if let Err(e) = workspace::save_environment(root, &env) {
            summary.errors.push(format!("environment '{}': {e}", env.name));
        } else {
            summary.environments += 1;
        }
    }
}

fn write_file(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| format!("Ghi file lỗi: {e}"))
}
