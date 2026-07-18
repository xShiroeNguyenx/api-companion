//! # workspace — Collections/Environments trên đĩa (TOML git-friendly)
//!
//! Files là source-of-truth cho những gì cần git; một request = một file `.toml`.
//! Xem docs/adr/0003. Crate này thuần I/O + parse, không phụ thuộc keychain/engine.

pub mod format;
pub mod vars;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ipc_types::{
    Auth, Connection, Environment, KeyValue, NodeKind, RequestSpec, SavedRequest, TreeNode,
    WorkspaceInfo,
};

use format::{CollectionFile, ConnectionFile, EnvironmentFile, RequestFile, WorkspaceFile};

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml write: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("không tìm thấy: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}

type Result<T> = std::result::Result<T, WorkspaceError>;

const COLLECTIONS: &str = "collections";
const ENVIRONMENTS: &str = "environments";
const CONNECTIONS: &str = "connections";
const WORKSPACE_TOML: &str = "workspace.toml";
const COLLECTION_TOML: &str = "collection.toml";

/// Đảm bảo workspace tồn tại tại `root` (tạo dirs + workspace.toml nếu chưa có).
pub fn ensure(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root.join(COLLECTIONS))?;
    std::fs::create_dir_all(root.join(ENVIRONMENTS))?;
    std::fs::create_dir_all(root.join(CONNECTIONS))?;
    let ws = root.join(WORKSPACE_TOML);
    if !ws.exists() {
        write_toml(&ws, &WorkspaceFile::default())?;
    }
    Ok(())
}

/// Chuẩn hoá đường dẫn gốc workspace về một chuỗi ổn định để dedup/so sánh trong registry.
///
/// Chỉ chuẩn hoá **từ vựng** (lexical) — KHÔNG `canonicalize` để không fail khi drive
/// offline hoặc path chưa tồn tại. Windows: thống nhất separator `\`, bỏ verbatim prefix
/// `\\?\`, bỏ separator cuối. Giữ nguyên hoa/thường để path hiển thị đẹp — mọi workspace
/// đều được thêm qua dialog directory (trả casing nhất quán trên đĩa) nên dedup vẫn ổn.
pub fn normalize_root(p: &Path) -> String {
    // Absolutize từ vựng (không resolve symlink, không fail khi path chưa tồn tại).
    let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let mut s = abs.to_string_lossy().to_string();

    if let Some(rest) = s.strip_prefix(r"\\?\") {
        s = rest.to_string();
    }

    if cfg!(windows) {
        s = s.replace('/', "\\");
        // Bỏ separator ở cuối, nhưng giữ gốc ổ đĩa dạng "C:\".
        while s.len() > 3 && s.ends_with('\\') {
            s.pop();
        }
    } else {
        while s.len() > 1 && s.ends_with('/') {
            s.pop();
        }
    }
    s
}

// ---------------------------------------------------------------------------
// workspace.toml
// ---------------------------------------------------------------------------

fn read_workspace_file(root: &Path) -> Result<WorkspaceFile> {
    let path = root.join(WORKSPACE_TOML);
    if !path.exists() {
        return Ok(WorkspaceFile::default());
    }
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

/// Đặt environment đang active (ghi vào workspace.toml).
pub fn set_active_environment(root: &Path, name: Option<String>) -> Result<()> {
    let mut ws = read_workspace_file(root)?;
    ws.active_environment = name;
    write_toml(&root.join(WORKSPACE_TOML), &ws)?;
    Ok(())
}

/// Biến global (workspace scope).
pub fn global_variables(root: &Path) -> Result<Vec<KeyValue>> {
    Ok(read_workspace_file(root)?.variables)
}

/// Environment đang active (đọc nhanh từ workspace.toml).
pub fn active_environment(root: &Path) -> Result<Option<String>> {
    Ok(read_workspace_file(root)?.active_environment)
}

// ---------------------------------------------------------------------------
// Tree
// ---------------------------------------------------------------------------

/// Thông tin workspace đầy đủ cho frontend.
pub fn info(root: &Path) -> Result<WorkspaceInfo> {
    ensure(root)?;
    let ws = read_workspace_file(root)?;
    Ok(WorkspaceInfo {
        path: root.to_string_lossy().to_string(),
        name: ws.name,
        active_environment: ws.active_environment,
        environments: list_environment_names(root)?,
        tree: scan_collections(root)?,
    })
}

fn scan_collections(root: &Path) -> Result<Vec<TreeNode>> {
    let dir = root.join(COLLECTIONS);
    let mut nodes = Vec::new();
    if !dir.exists() {
        return Ok(nodes);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dname = entry.file_name().to_string_lossy().to_string();
        let id = format!("{COLLECTIONS}/{dname}");
        let name = read_collection_meta(root, &id)
            .map(|c| c.name)
            .unwrap_or_else(|_| dname.clone());
        nodes.push(TreeNode {
            id: id.clone(),
            name,
            kind: NodeKind::Collection,
            method: None,
            children: scan_children(root, &id)?,
        });
    }
    nodes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(nodes)
}

fn scan_children(root: &Path, rel: &str) -> Result<Vec<TreeNode>> {
    let dir = root.join(rel);
    let mut folders = Vec::new();
    let mut requests = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        let ftype = entry.file_type()?;
        let id = format!("{rel}/{fname}");
        if ftype.is_dir() {
            folders.push(TreeNode {
                id: id.clone(),
                name: fname,
                kind: NodeKind::Folder,
                method: None,
                children: scan_children(root, &id)?,
            });
        } else if fname.ends_with(".toml") && fname != COLLECTION_TOML {
            let (name, method) = match read_request_file(&entry.path()) {
                Ok(rf) => (rf.name, Some(rf.method)),
                Err(_) => (fname.trim_end_matches(".toml").to_string(), None),
            };
            requests.push(TreeNode {
                id,
                name,
                kind: NodeKind::Request,
                method,
                children: Vec::new(),
            });
        }
    }
    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    requests.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    folders.extend(requests);
    Ok(folders)
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

fn read_request_file(path: &Path) -> Result<RequestFile> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

/// Lưu request. Nếu `target_id` là file `.toml` → ghi đè; nếu là thư mục
/// (collection/folder) → tạo file mới với tên slug hoá từ `name`. Trả về id.
pub fn save_request(root: &Path, target_id: &str, name: &str, spec: &RequestSpec) -> Result<String> {
    let file = RequestFile::from_spec(name, spec);
    if target_id.ends_with(".toml") {
        let path = root.join(target_id);
        write_toml(&path, &file)?;
        Ok(target_id.to_string())
    } else {
        let dir = root.join(target_id);
        std::fs::create_dir_all(&dir)?;
        let fname = unique_name(&dir, &slug(name), "toml");
        write_toml(&dir.join(&fname), &file)?;
        Ok(format!("{target_id}/{fname}"))
    }
}

/// Load một request để mở vào tab.
pub fn load_request(root: &Path, id: &str) -> Result<SavedRequest> {
    let path = root.join(id);
    if !path.exists() {
        return Err(WorkspaceError::NotFound(id.to_string()));
    }
    let (name, spec) = read_request_file(&path)?.into_spec();
    Ok(SavedRequest {
        id: id.to_string(),
        name,
        spec,
        collection_id: collection_root_of(id),
    })
}

/// Xoá một node (file request hoặc thư mục collection/folder).
pub fn delete_node(root: &Path, id: &str) -> Result<()> {
    let path = root.join(id);
    if !path.exists() {
        return Err(WorkspaceError::NotFound(id.to_string()));
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Collections & folders
// ---------------------------------------------------------------------------

/// Tạo collection mới, trả về id.
pub fn create_collection(root: &Path, name: &str) -> Result<String> {
    let base = root.join(COLLECTIONS);
    std::fs::create_dir_all(&base)?;
    let dname = unique_dir(&base, &slug(name));
    let dir = base.join(&dname);
    std::fs::create_dir_all(&dir)?;
    write_toml(&dir.join(COLLECTION_TOML), &CollectionFile::new(name))?;
    Ok(format!("{COLLECTIONS}/{dname}"))
}

/// Tạo folder con trong một collection/folder, trả về id.
pub fn create_folder(root: &Path, parent_id: &str, name: &str) -> Result<String> {
    let parent = root.join(parent_id);
    if !parent.is_dir() {
        return Err(WorkspaceError::Invalid(format!("Parent không phải thư mục: {parent_id}")));
    }
    let dname = unique_dir(&parent, &slug(name));
    std::fs::create_dir_all(parent.join(&dname))?;
    Ok(format!("{parent_id}/{dname}"))
}

fn read_collection_meta(root: &Path, collection_id: &str) -> Result<CollectionFile> {
    let path = root.join(collection_id).join(COLLECTION_TOML);
    if !path.exists() {
        let name = collection_id.rsplit('/').next().unwrap_or(collection_id).to_string();
        return Ok(CollectionFile::new(name));
    }
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

/// Trả về id collection top-level chứa một node bất kỳ.
pub fn collection_root_of(id: &str) -> Option<String> {
    let parts: Vec<&str> = id.split('/').collect();
    if parts.len() >= 2 && parts[0] == COLLECTIONS {
        Some(format!("{COLLECTIONS}/{}", parts[1]))
    } else {
        None
    }
}

/// Auth mặc định + headers mặc định + biến của collection chứa `id` (nếu có).
pub fn collection_defaults(root: &Path, id: &str) -> (Auth, Vec<KeyValue>, Vec<KeyValue>) {
    match collection_root_of(id).and_then(|cid| read_collection_meta(root, &cid).ok()) {
        Some(meta) => (meta.auth, meta.headers, meta.variables),
        None => (Auth::Inherit, Vec::new(), Vec::new()),
    }
}

/// Metadata đầy đủ của một collection (name/description/auth/headers/variables).
pub fn collection_meta(
    root: &Path,
    collection_id: &str,
) -> (String, Option<String>, Auth, Vec<KeyValue>, Vec<KeyValue>) {
    match read_collection_meta(root, collection_id) {
        Ok(m) => (m.name, m.description, m.auth, m.headers, m.variables),
        Err(_) => {
            let name = collection_id.rsplit('/').next().unwrap_or(collection_id).to_string();
            (name, None, Auth::Inherit, Vec::new(), Vec::new())
        }
    }
}

/// Ghi collection.toml (dùng khi import/tạo collection kèm defaults).
pub fn save_collection_meta(
    root: &Path,
    collection_id: &str,
    name: &str,
    description: Option<String>,
    auth: Auth,
    headers: Vec<KeyValue>,
    variables: Vec<KeyValue>,
) -> Result<()> {
    let dir = root.join(collection_id);
    std::fs::create_dir_all(&dir)?;
    let file = CollectionFile {
        schema_version: format::SCHEMA_VERSION,
        name: name.to_string(),
        description,
        auth,
        variables,
        headers,
    };
    write_toml(&dir.join(COLLECTION_TOML), &file)?;
    Ok(())
}

/// Tất cả request dưới một collection/folder, kèm folder path tương đối trong collection.
/// Trả về (folder_rel, SavedRequest). folder_rel = "" nếu request nằm ở gốc collection.
pub fn collection_requests(root: &Path, collection_id: &str) -> Result<Vec<(String, SavedRequest)>> {
    let mut ids = Vec::new();
    collect_request_ids(&scan_children(root, collection_id)?, &mut ids);
    let prefix = format!("{collection_id}/");
    let mut out = Vec::new();
    for id in ids {
        if let Ok(req) = load_request(root, &id) {
            // rel path trong collection = phần sau prefix, bỏ tên file.
            let rel = id.strip_prefix(&prefix).unwrap_or(&id);
            let folder = match rel.rfind('/') {
                Some(i) => rel[..i].to_string(),
                None => String::new(),
            };
            out.push((folder, req));
        }
    }
    Ok(out)
}

fn collect_request_ids(nodes: &[TreeNode], out: &mut Vec<String>) {
    for n in nodes {
        match n.kind {
            NodeKind::Request => out.push(n.id.clone()),
            _ => collect_request_ids(&n.children, out),
        }
    }
}

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

pub fn list_environment_names(root: &Path) -> Result<Vec<String>> {
    let dir = root.join(ENVIRONMENTS);
    let mut names = Vec::new();
    if !dir.exists() {
        return Ok(names);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if let Some(stem) = fname.strip_suffix(".toml") {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub fn load_environment(root: &Path, name: &str) -> Result<Environment> {
    let path = root.join(ENVIRONMENTS).join(format!("{}.toml", slug(name)));
    if !path.exists() {
        return Err(WorkspaceError::NotFound(format!("environment {name}")));
    }
    let file: EnvironmentFile = toml::from_str(&std::fs::read_to_string(path)?)?;
    Ok(file.into())
}

/// Ghi environment ra file. LƯU Ý: giá trị secret nên được caller xoá trước
/// khi gọi (đưa vào keychain) — crate này ghi đúng những gì được đưa.
pub fn save_environment(root: &Path, env: &Environment) -> Result<()> {
    let dir = root.join(ENVIRONMENTS);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.toml", slug(&env.name)));
    write_toml(&path, &EnvironmentFile::from_env(env))?;
    Ok(())
}

pub fn delete_environment(root: &Path, name: &str) -> Result<()> {
    let path = root.join(ENVIRONMENTS).join(format!("{}.toml", slug(name)));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Connections (Ops — P2-M1)
// ---------------------------------------------------------------------------

pub fn list_connections(root: &Path) -> Result<Vec<Connection>> {
    let dir = root.join(CONNECTIONS);
    let mut conns = Vec::new();
    if !dir.exists() {
        return Ok(conns);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) == Some("toml") {
            if let Ok(f) = toml::from_str::<ConnectionFile>(&std::fs::read_to_string(entry.path())?) {
                conns.push(f.into());
            }
        }
    }
    conns.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(conns)
}

pub fn load_connection(root: &Path, id: &str) -> Result<Connection> {
    let path = root.join(CONNECTIONS).join(format!("{}.toml", slug(id)));
    if !path.exists() {
        return Err(WorkspaceError::NotFound(format!("connection {id}")));
    }
    let f: ConnectionFile = toml::from_str(&std::fs::read_to_string(path)?)?;
    Ok(f.into())
}

pub fn save_connection(root: &Path, conn: &Connection) -> Result<()> {
    let dir = root.join(CONNECTIONS);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.toml", slug(&conn.id)));
    write_toml(&path, &ConnectionFile::from_connection(conn))?;
    Ok(())
}

pub fn delete_connection(root: &Path, id: &str) -> Result<()> {
    let path = root.join(CONNECTIONS).join(format!("{}.toml", slug(id)));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Var map builder (merge scopes)
// ---------------------------------------------------------------------------

/// Gộp biến theo thứ tự: global < collection < environment (env thắng).
/// `env_values` là map key→value đã resolve (secret đã lấy từ keychain bởi caller).
pub fn merge_vars(
    global: &[KeyValue],
    collection: &[KeyValue],
    env_values: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for kv in global.iter().filter(|k| k.enabled && !k.key.is_empty()) {
        map.insert(kv.key.clone(), kv.value.clone());
    }
    for kv in collection.iter().filter(|k| k.enabled && !k.key.is_empty()) {
        map.insert(kv.key.clone(), kv.value.clone());
    }
    for (k, v) in env_values {
        map.insert(k.clone(), v.clone());
    }
    map
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_toml<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string_pretty(value)?)?;
    Ok(())
}

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
        "item".to_string()
    } else {
        s
    }
}

fn unique_name(dir: &Path, base: &str, ext: &str) -> String {
    let mut candidate = format!("{base}.{ext}");
    let mut n = 2;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{n}.{ext}");
        n += 1;
    }
    candidate
}

fn unique_dir(parent: &Path, base: &str) -> String {
    let mut candidate = base.to_string();
    let mut n = 2;
    while parent.join(&candidate).exists() {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    candidate
}

// Tránh cảnh báo unused với PathBuf import khi refactor.
#[allow(dead_code)]
fn _touch(_p: PathBuf) {}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::{EnvVar, HttpMethod};

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn normalize_root_dedup() {
        // Trailing separator không tạo khác biệt.
        let a = normalize_root(Path::new("/ws/proj"));
        let b = normalize_root(Path::new("/ws/proj/"));
        assert_eq!(a, b);
        // Idempotent: chuẩn hoá lại chuỗi đã chuẩn hoá → giữ nguyên.
        assert_eq!(normalize_root(Path::new(&a)), a);
    }

    #[cfg(windows)]
    #[test]
    fn normalize_root_windows_seps() {
        // Thống nhất separator về '\' + bỏ trailing; forward-slash và trailing '\'
        // của cùng một path cho ra cùng chuỗi.
        let a = normalize_root(Path::new(r"C:\Ws\Proj"));
        let b = normalize_root(Path::new("C:/Ws/Proj/"));
        assert_eq!(a, b);
        assert!(!a.contains('/'));
    }

    #[test]
    fn create_collection_save_and_load_request() {
        let d = tmp();
        let root = d.path();
        ensure(root).unwrap();

        let cid = create_collection(root, "Orders API").unwrap();
        assert!(cid.starts_with("collections/"));

        let mut spec = RequestSpec::get("{{base}}/order");
        spec.method = HttpMethod::new("POST");
        let id = save_request(root, &cid, "Create Order", &spec).unwrap();
        assert!(id.ends_with(".toml"));

        let loaded = load_request(root, &id).unwrap();
        assert_eq!(loaded.name, "Create Order");
        assert_eq!(loaded.spec.method.as_str(), "POST");
        assert_eq!(loaded.collection_id.as_deref(), Some(cid.as_str()));

        // Tree phản ánh collection + request.
        let info = info(root).unwrap();
        assert_eq!(info.tree.len(), 1);
        assert_eq!(info.tree[0].name, "Orders API");
        assert_eq!(info.tree[0].children.len(), 1);
        assert_eq!(info.tree[0].children[0].kind, NodeKind::Request);
    }

    #[test]
    fn overwrite_request_keeps_id() {
        let d = tmp();
        let root = d.path();
        let cid = create_collection(root, "C").unwrap();
        let id = save_request(root, &cid, "R", &RequestSpec::get("https://a")).unwrap();
        let mut spec = RequestSpec::get("https://b");
        spec.method = HttpMethod::new("PUT");
        let id2 = save_request(root, &id, "R", &spec).unwrap();
        assert_eq!(id, id2);
        assert_eq!(load_request(root, &id).unwrap().spec.url, "https://b");
    }

    #[test]
    fn environments_roundtrip_and_active() {
        let d = tmp();
        let root = d.path();
        ensure(root).unwrap();
        let env = Environment {
            id: "staging".into(),
            name: "staging".into(),
            variables: vec![
                EnvVar { key: "base".into(), value: "https://staging".into(), is_secret: false, description: None },
                EnvVar { key: "token".into(), value: String::new(), is_secret: true, description: None },
            ],
        };
        save_environment(root, &env).unwrap();
        set_active_environment(root, Some("staging".into())).unwrap();

        let info = info(root).unwrap();
        assert_eq!(info.environments, vec!["staging".to_string()]);
        assert_eq!(info.active_environment.as_deref(), Some("staging"));

        let loaded = load_environment(root, "staging").unwrap();
        assert_eq!(loaded.variables.len(), 2);
        assert!(loaded.variables.iter().any(|v| v.is_secret && v.key == "token"));
    }

    #[test]
    fn connections_crud() {
        use ipc_types::{Connection, ConnectionKind};
        let d = tmp();
        let root = d.path();
        ensure(root).unwrap();
        let c = Connection {
            id: "prod-ssh".into(),
            name: "Prod bastion".into(),
            kind: ConnectionKind::Ssh,
            host: "1.2.3.4".into(),
            port: 22,
            username: "deploy".into(),
            db_driver: None,
            database: None,
            auth_method: Some("key".into()),
            key_path: Some("/home/me/.ssh/id".into()),
            has_secret: false,
        };
        save_connection(root, &c).unwrap();
        let list = list_connections(root).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, ConnectionKind::Ssh);
        assert_eq!(load_connection(root, "prod-ssh").unwrap().host, "1.2.3.4");
        delete_connection(root, "prod-ssh").unwrap();
        assert_eq!(list_connections(root).unwrap().len(), 0);
    }

    #[test]
    fn merge_vars_precedence() {
        let global = vec![KeyValue { key: "x".into(), value: "g".into(), enabled: true }];
        let coll = vec![KeyValue { key: "x".into(), value: "c".into(), enabled: true }];
        let mut env = HashMap::new();
        env.insert("x".to_string(), "e".to_string());
        let merged = merge_vars(&global, &coll, &env);
        assert_eq!(merged.get("x").unwrap(), "e");
    }
}
