//! # workspace-sync — Team workspace trên MySQL tự dựng
//!
//! Team dựng một MySQL server; mọi thành viên nhập thông tin kết nối là dùng chung
//! một workspace. Cách hoạt động: nội dung workspace vẫn là **file TOML trong một
//! thư mục cache local** (mọi tính năng file-based dùng lại nguyên vẹn), crate này
//! đồng bộ 3 chiều (local / remote / base-snapshot) theo từng file:
//!
//! - Chỉ local đổi → push. Chỉ remote đổi → pull. Xoá cũng vậy (tombstone).
//! - Cả hai cùng đổi → **server thắng**, bản local được giữ lại thành file
//!   `*-conflict-<ts>.toml` (và đẩy lên server để cả team thấy).
//! - `workspace.toml` đặc biệt: `active_environment` là lựa chọn cá nhân — không
//!   tính vào diff và luôn giữ giá trị local khi pull.
//!
//! An toàn dữ liệu hệ thống: setup CHỈ tạo database MỚI (tên user đặt, mặc định
//! `apic_workspace`) và 2 bảng `apic_files`/`apic_meta` bên trong — mọi câu SQL đều
//! qualified `` `db`.`bảng` ``, không bao giờ đụng database khác đang tồn tại.
//!
//! Tương thích server cũ: KHÔNG ép `ENGINE` (server nào chỉ có MyISAM vẫn chạy) và
//! PRIMARY KEY là `path_hash` CHAR(64) thay vì path trực tiếp (MyISAM giới hạn key
//! 1000 bytes, InnoDB format cũ 767 bytes — VARCHAR(500) utf8mb4 = 2000 bytes sẽ fail).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ipc_types::WsSyncReport;
use sha2::{Digest, Sha256};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("mysql: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

type Result<T> = std::result::Result<T, SyncError>;

/// Tên file trạng thái sync (nằm ở gốc workspace cache; không bao giờ được sync).
const STATE_FILE: &str = ".apic-sync.json";
const FILES_TABLE: &str = "apic_files";
const META_TABLE: &str = "apic_meta";

// ---------------------------------------------------------------------------
// URL & validate
// ---------------------------------------------------------------------------

/// Tên database chỉ cho phép chữ/số/underscore (chống injection ở vị trí identifier).
pub fn valid_db_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Percent-encode userinfo cho URL kết nối.
fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// URL kết nối tới SERVER (không kèm database — mọi bảng đều qualified theo tên DB).
pub fn server_url(host: &str, port: u16, username: &str, password: Option<&str>) -> String {
    let userinfo = match password {
        Some(pw) if !pw.is_empty() => format!("{}:{}", enc(username), enc(pw)),
        _ => enc(username),
    };
    format!("mysql://{userinfo}@{host}:{port}")
}

// ---------------------------------------------------------------------------
// Remote: test + init (CHỈ tạo mới database/bảng riêng)
// ---------------------------------------------------------------------------

async fn connect(url: &str) -> Result<MySqlPool> {
    Ok(MySqlPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(url)
        .await?)
}

/// Kiểm tra kết nối tới server (chưa cần database tồn tại).
pub async fn test_server(url: &str) -> Result<()> {
    let pool = connect(url).await?;
    sqlx::query("SELECT 1").fetch_one(&pool).await?;
    pool.close().await;
    Ok(())
}

/// Khởi tạo phía server: tạo database riêng + bảng (idempotent, `IF NOT EXISTS`).
/// KHÔNG đụng bất kỳ database nào khác. User MySQL cần quyền CREATE trên db này.
pub async fn init_remote(url: &str, db: &str, ws_name: &str) -> Result<()> {
    if !valid_db_name(db) {
        return Err(SyncError::Invalid(format!(
            "Tên database không hợp lệ: \"{db}\" (chỉ chữ/số/underscore, ≤64 ký tự)"
        )));
    }
    let pool = connect(url).await?;
    sqlx::query(&format!(
        "CREATE DATABASE IF NOT EXISTS `{db}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
    ))
    .execute(&pool)
    .await?;
    // KHÔNG chỉ định ENGINE — dùng engine mặc định của server để chạy được cả trên
    // MySQL cũ chỉ có MyISAM (lỗi 1286 nếu ép InnoDB). Vì thế PK là `path_hash`
    // (CHAR(64) = 64 bytes): MyISAM giới hạn key 1000 bytes, InnoDB cũ 767 bytes,
    // nên không thể PK trực tiếp trên VARCHAR(500) utf8mb4 (2000 bytes).
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS `{db}`.`{FILES_TABLE}` (
            path_hash  CHAR(64)      NOT NULL PRIMARY KEY,
            path       VARCHAR(500)  NOT NULL,
            content    MEDIUMTEXT    NOT NULL,
            hash       CHAR(64)      NOT NULL,
            deleted    TINYINT(1)    NOT NULL DEFAULT 0,
            updated_at BIGINT        NOT NULL,
            updated_by VARCHAR(128)  NULL
        ) DEFAULT CHARSET=utf8mb4"
    ))
    .execute(&pool)
    .await?;
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS `{db}`.`{META_TABLE}` (
            k VARCHAR(64) NOT NULL PRIMARY KEY,
            v TEXT NULL
        ) DEFAULT CHARSET=utf8mb4"
    ))
    .execute(&pool)
    .await?;
    sqlx::query(&format!(
        "INSERT IGNORE INTO `{db}`.`{META_TABLE}` (k, v) VALUES ('schema_version', '1'), ('name', ?)"
    ))
    .bind(ws_name)
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Local scan + hash + base state
// ---------------------------------------------------------------------------

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())
}

/// Khoá PRIMARY KEY trên server cho một path (sha256 — cố định 64 bytes, an toàn
/// với giới hạn key của MyISAM/InnoDB cũ; xem comment trong `init_remote`).
fn path_key(path: &str) -> String {
    sha256_hex(path)
}

/// Nội dung workspace.toml sau khi bỏ `active_environment` (lựa chọn cá nhân,
/// không sync). Parse hỏng → trả nguyên văn.
fn normalize_ws_toml(content: &str) -> String {
    match content.parse::<toml::Table>() {
        Ok(mut t) => {
            t.remove("active_environment");
            toml::to_string(&t).unwrap_or_else(|_| content.to_string())
        }
        Err(_) => content.to_string(),
    }
}

/// Hash dùng cho diff — với workspace.toml hash bản normalized.
fn norm_hash(path: &str, content: &str) -> String {
    if path == "workspace.toml" {
        sha256_hex(&normalize_ws_toml(content))
    } else {
        sha256_hex(content)
    }
}

/// Ghép `active_environment` local vào nội dung workspace.toml từ remote.
fn merge_ws_toml(remote_content: &str, local_content: Option<&str>) -> String {
    let local_env = local_content
        .and_then(|c| c.parse::<toml::Table>().ok())
        .and_then(|t| t.get("active_environment").cloned());
    match remote_content.parse::<toml::Table>() {
        Ok(mut t) => {
            match local_env {
                Some(v) => {
                    t.insert("active_environment".to_string(), v);
                }
                None => {
                    t.remove("active_environment");
                }
            }
            toml::to_string(&t).unwrap_or_else(|_| remote_content.to_string())
        }
        Err(_) => remote_content.to_string(),
    }
}

/// Quét toàn bộ file sync-được trong workspace: `workspace.toml`,
/// `collections/**/*.toml`, `environments/*.toml`, `connections/*.toml`.
/// Trả map path (forward-slash, tương đối) → nội dung. Bỏ qua dotfiles.
pub fn scan_local(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let ws = root.join("workspace.toml");
    if ws.is_file() {
        out.insert("workspace.toml".to_string(), std::fs::read_to_string(&ws)?);
    }
    for dir in ["collections", "environments", "connections"] {
        collect_toml(root, &root.join(dir), &mut out)?;
    }
    Ok(out)
}

fn collect_toml(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let p = entry.path();
        if entry.file_type()?.is_dir() {
            collect_toml(root, &p, out)?;
        } else if name.ends_with(".toml") {
            let rel = p
                .strip_prefix(root)
                .map_err(|_| SyncError::Invalid("path ngoài workspace".into()))?
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, std::fs::read_to_string(&p)?);
        }
    }
    Ok(())
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct BaseState {
    /// path → hash (normalized) tại thời điểm sync thành công gần nhất.
    #[serde(default)]
    files: BTreeMap<String, String>,
}

fn load_base(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(root.join(STATE_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<BaseState>(&s).ok())
        .map(|b| b.files)
        .unwrap_or_default()
}

fn save_base(root: &Path, files: BTreeMap<String, String>) -> Result<()> {
    let s = serde_json::to_string_pretty(&BaseState { files })?;
    std::fs::write(root.join(STATE_FILE), s)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Plan (pure — unit-test không cần MySQL)
// ---------------------------------------------------------------------------

/// Một dòng trong index phía remote.
#[derive(Debug, Clone)]
pub struct RemoteEntry {
    pub hash: String,
    pub deleted: bool,
}

/// Hành động đồng bộ cho một path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Pull(String),
    Push(String),
    DeleteLocal(String),
    DeleteRemote(String),
    /// Hai bên cùng đổi khác nhau → server thắng, bản local giữ thành conflict copy.
    Conflict(String),
}

/// Diff 3 chiều: local (hash) / remote (hash+tombstone) / base (hash lần sync trước).
pub fn plan(
    local: &BTreeMap<String, String>,
    remote: &BTreeMap<String, RemoteEntry>,
    base: &BTreeMap<String, String>,
) -> Vec<Action> {
    let mut paths: Vec<&String> = local.keys().chain(remote.keys()).chain(base.keys()).collect();
    paths.sort();
    paths.dedup();

    let mut actions = Vec::new();
    for p in paths {
        let l = local.get(p);
        let r = remote.get(p).filter(|e| !e.deleted).map(|e| &e.hash);
        let b = base.get(p);
        match (l, r) {
            (Some(lh), Some(rh)) => {
                if lh == rh {
                    // đã khớp — không làm gì
                } else if Some(lh) == b {
                    actions.push(Action::Pull(p.clone())); // chỉ remote đổi
                } else if Some(rh) == b {
                    actions.push(Action::Push(p.clone())); // chỉ local đổi
                } else {
                    actions.push(Action::Conflict(p.clone())); // cả hai đổi
                }
            }
            (Some(lh), None) => {
                if b.is_none() {
                    actions.push(Action::Push(p.clone())); // file mới local
                } else if Some(lh) == b {
                    actions.push(Action::DeleteLocal(p.clone())); // remote đã xoá
                } else {
                    actions.push(Action::Push(p.clone())); // local sửa > remote xoá
                }
            }
            (None, Some(rh)) => {
                if b.is_none() {
                    actions.push(Action::Pull(p.clone())); // file mới remote
                } else if Some(rh) == b {
                    actions.push(Action::DeleteRemote(p.clone())); // local đã xoá
                } else {
                    actions.push(Action::Pull(p.clone())); // remote sửa > local xoá
                }
            }
            (None, None) => {} // chỉ còn trong base → sẽ tự rơi khỏi base mới
        }
    }
    actions
}

// ---------------------------------------------------------------------------
// Sync (thực thi plan trên fs + MySQL)
// ---------------------------------------------------------------------------

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn who() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".into())
}

async fn fetch_remote_index(pool: &MySqlPool, db: &str) -> Result<BTreeMap<String, RemoteEntry>> {
    let rows = sqlx::query(&format!(
        "SELECT path, hash, deleted FROM `{db}`.`{FILES_TABLE}`"
    ))
    .fetch_all(pool)
    .await?;
    let mut out = BTreeMap::new();
    for row in rows {
        let path: String = row.try_get("path")?;
        let hash: String = row.try_get("hash")?;
        let deleted: i8 = row.try_get("deleted")?;
        out.insert(path, RemoteEntry { hash, deleted: deleted != 0 });
    }
    Ok(out)
}

async fn fetch_content(pool: &MySqlPool, db: &str, path: &str) -> Result<String> {
    let row = sqlx::query(&format!(
        "SELECT content FROM `{db}`.`{FILES_TABLE}` WHERE path_hash = ?"
    ))
    .bind(path_key(path))
    .fetch_one(pool)
    .await?;
    Ok(row.try_get("content")?)
}

async fn push_file(pool: &MySqlPool, db: &str, path: &str, content: &str) -> Result<()> {
    let hash = norm_hash(path, content);
    sqlx::query(&format!(
        "INSERT INTO `{db}`.`{FILES_TABLE}` (path_hash, path, content, hash, deleted, updated_at, updated_by)
         VALUES (?, ?, ?, ?, 0, ?, ?)
         ON DUPLICATE KEY UPDATE content = VALUES(content), hash = VALUES(hash),
             deleted = 0, updated_at = VALUES(updated_at), updated_by = VALUES(updated_by)"
    ))
    .bind(path_key(path))
    .bind(path)
    .bind(content)
    .bind(&hash)
    .bind(now_ms())
    .bind(who())
    .execute(pool)
    .await?;
    Ok(())
}

async fn tombstone(pool: &MySqlPool, db: &str, path: &str) -> Result<()> {
    sqlx::query(&format!(
        "UPDATE `{db}`.`{FILES_TABLE}` SET deleted = 1, content = '', updated_at = ?, updated_by = ? WHERE path_hash = ?"
    ))
    .bind(now_ms())
    .bind(who())
    .bind(path_key(path))
    .execute(pool)
    .await?;
    Ok(())
}

fn write_local(root: &Path, rel: &str, content: &str) -> Result<()> {
    let p = local_path(root, rel)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, content)?;
    Ok(())
}

/// Chặn path traversal từ dữ liệu remote (`..`, path tuyệt đối).
fn local_path(root: &Path, rel: &str) -> Result<PathBuf> {
    let ok = !rel.is_empty()
        && !rel.starts_with('/')
        && !rel.contains(':')
        && rel.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != "..");
    if !ok {
        return Err(SyncError::Invalid(format!("path không hợp lệ từ remote: {rel}")));
    }
    Ok(root.join(rel))
}

/// Tên conflict copy: `a/b/req.toml` → `a/b/req-conflict-<ts>.toml`.
fn conflict_path(rel: &str) -> String {
    let ts = now_ms() / 1000;
    match rel.strip_suffix(".toml") {
        Some(stem) => format!("{stem}-conflict-{ts}.toml"),
        None => format!("{rel}-conflict-{ts}"),
    }
}

/// Xoá đệ quy các thư mục rỗng dưới `collections/` (sau khi remote xoá cả folder).
fn prune_empty_dirs(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }
    let mut empty = true;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if !prune_empty_dirs(&entry.path())? {
                empty = false;
            }
        } else {
            empty = false;
        }
    }
    if empty {
        std::fs::remove_dir(dir)?;
    }
    Ok(empty)
}

/// Đồng bộ hai chiều một lần. `url` là server URL (từ [`server_url`]), `db` là tên
/// database riêng của workspace. Trả về báo cáo số lượng để UI hiển thị.
pub async fn sync(root: &Path, url: &str, db: &str) -> Result<WsSyncReport> {
    if !valid_db_name(db) {
        return Err(SyncError::Invalid(format!("Tên database không hợp lệ: {db}")));
    }
    let pool = connect(url).await?;
    let result = sync_with_pool(root, &pool, db).await;
    pool.close().await;
    result
}

async fn sync_with_pool(root: &Path, pool: &MySqlPool, db: &str) -> Result<WsSyncReport> {
    let local = scan_local(root)?;
    let local_hashes: BTreeMap<String, String> =
        local.iter().map(|(p, c)| (p.clone(), norm_hash(p, c))).collect();
    let remote = fetch_remote_index(pool, db).await?;
    let base = load_base(root);
    let actions = plan(&local_hashes, &remote, &base);

    let mut report = WsSyncReport::default();
    for action in &actions {
        match action {
            Action::Pull(p) => {
                let content = fetch_content(pool, db, p).await?;
                let merged = if p == "workspace.toml" {
                    merge_ws_toml(&content, local.get(p).map(|s| s.as_str()))
                } else {
                    content
                };
                write_local(root, p, &merged)?;
                report.pulled += 1;
            }
            Action::Push(p) => {
                if let Some(content) = local.get(p) {
                    push_file(pool, db, p, content).await?;
                    report.pushed += 1;
                }
            }
            Action::DeleteLocal(p) => {
                let fp = local_path(root, p)?;
                if fp.exists() {
                    std::fs::remove_file(fp)?;
                }
                report.deleted_local += 1;
            }
            Action::DeleteRemote(p) => {
                tombstone(pool, db, p).await?;
                report.deleted_remote += 1;
            }
            Action::Conflict(p) => {
                let remote_content = fetch_content(pool, db, p).await?;
                if p == "workspace.toml" {
                    // Merge giữ active_environment local; biến global lấy theo server.
                    let merged = merge_ws_toml(&remote_content, local.get(p).map(|s| s.as_str()));
                    write_local(root, p, &merged)?;
                    report.pulled += 1;
                } else {
                    // Server thắng; bản local giữ thành conflict copy + đẩy lên server.
                    if let Some(local_content) = local.get(p) {
                        let cp = conflict_path(p);
                        write_local(root, &cp, local_content)?;
                        push_file(pool, db, &cp, local_content).await?;
                        report.pushed += 1;
                    }
                    write_local(root, p, &remote_content)?;
                    report.pulled += 1;
                    report.conflicts.push(p.clone());
                }
            }
        }
    }

    // Dọn thư mục rỗng sau khi xoá file (xoá cả collection/folder phía remote).
    if report.deleted_local > 0 {
        let coll = root.join("collections");
        if coll.is_dir() {
            for entry in std::fs::read_dir(&coll)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let _ = prune_empty_dirs(&entry.path());
                }
            }
        }
    }

    // Base mới = trạng thái local sau khi áp mọi action (== remote cho các path đã đụng).
    let final_local = scan_local(root)?;
    let final_hashes: BTreeMap<String, String> =
        final_local.iter().map(|(p, c)| (p.clone(), norm_hash(p, c))).collect();
    save_base(root, final_hashes)?;
    Ok(report)
}

// ---------------------------------------------------------------------------
// Tests (pure logic — không cần MySQL)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> String {
        sha256_hex(s)
    }

    fn remote_live(hash: &str) -> RemoteEntry {
        RemoteEntry { hash: hash.into(), deleted: false }
    }

    fn remote_dead() -> RemoteEntry {
        RemoteEntry { hash: String::new(), deleted: true }
    }

    #[test]
    fn db_name_validation() {
        assert!(valid_db_name("apic_workspace"));
        assert!(valid_db_name("Team01"));
        assert!(!valid_db_name(""));
        assert!(!valid_db_name("a-b"));
        assert!(!valid_db_name("a b"));
        assert!(!valid_db_name("a`;DROP"));
        assert!(!valid_db_name(&"x".repeat(65)));
    }

    #[test]
    fn server_url_encodes_userinfo() {
        let url = server_url("10.0.0.5", 3306, "u ser", Some("p@ss:w"));
        assert_eq!(url, "mysql://u%20ser:p%40ss%3Aw@10.0.0.5:3306");
        assert_eq!(server_url("db", 3307, "root", None), "mysql://root@db:3307");
    }

    #[test]
    fn plan_fresh_push_and_pull() {
        // Local có file mới (chưa từng sync) → push; remote có file lạ → pull.
        let local = BTreeMap::from([("collections/a/r.toml".to_string(), h("A"))]);
        let remote = BTreeMap::from([("environments/staging.toml".to_string(), remote_live(&h("E")))]);
        let base = BTreeMap::new();
        let actions = plan(&local, &remote, &base);
        assert_eq!(
            actions,
            vec![
                Action::Push("collections/a/r.toml".into()),
                Action::Pull("environments/staging.toml".into()),
            ]
        );
    }

    #[test]
    fn plan_one_side_change() {
        let base = BTreeMap::from([
            ("x.toml".to_string(), h("v1")),
            ("y.toml".to_string(), h("v1")),
        ]);
        // x: chỉ local đổi → Push. y: chỉ remote đổi → Pull.
        let local = BTreeMap::from([
            ("x.toml".to_string(), h("v2")),
            ("y.toml".to_string(), h("v1")),
        ]);
        let remote = BTreeMap::from([
            ("x.toml".to_string(), remote_live(&h("v1"))),
            ("y.toml".to_string(), remote_live(&h("v3"))),
        ]);
        let actions = plan(&local, &remote, &base);
        assert_eq!(actions, vec![Action::Push("x.toml".into()), Action::Pull("y.toml".into())]);
    }

    #[test]
    fn plan_conflict_when_both_change() {
        let base = BTreeMap::from([("r.toml".to_string(), h("v1"))]);
        let local = BTreeMap::from([("r.toml".to_string(), h("local"))]);
        let remote = BTreeMap::from([("r.toml".to_string(), remote_live(&h("remote")))]);
        assert_eq!(plan(&local, &remote, &base), vec![Action::Conflict("r.toml".into())]);
    }

    #[test]
    fn plan_deletes() {
        let base = BTreeMap::from([
            ("gone-local.toml".to_string(), h("v")),
            ("gone-remote.toml".to_string(), h("v")),
        ]);
        // local xoá gone-local (remote không đổi) → DeleteRemote.
        // remote tombstone gone-remote (local không đổi) → DeleteLocal.
        let local = BTreeMap::from([("gone-remote.toml".to_string(), h("v"))]);
        let remote = BTreeMap::from([
            ("gone-local.toml".to_string(), remote_live(&h("v"))),
            ("gone-remote.toml".to_string(), remote_dead()),
        ]);
        let actions = plan(&local, &remote, &base);
        assert_eq!(
            actions,
            vec![
                Action::DeleteRemote("gone-local.toml".into()),
                Action::DeleteLocal("gone-remote.toml".into()),
            ]
        );
    }

    #[test]
    fn plan_edit_beats_delete() {
        let base = BTreeMap::from([
            ("a.toml".to_string(), h("v1")),
            ("b.toml".to_string(), h("v1")),
        ]);
        // a: local sửa + remote xoá → Push (giữ bản sửa).
        // b: local xoá + remote sửa → Pull (khôi phục bản mới).
        let local = BTreeMap::from([("a.toml".to_string(), h("v2"))]);
        let remote = BTreeMap::from([
            ("a.toml".to_string(), remote_dead()),
            ("b.toml".to_string(), remote_live(&h("v2"))),
        ]);
        let actions = plan(&local, &remote, &base);
        assert_eq!(actions, vec![Action::Push("a.toml".into()), Action::Pull("b.toml".into())]);
    }

    #[test]
    fn ws_toml_active_env_ignored_in_diff_and_kept_on_pull() {
        let a = "active_environment = \"dev\"\nname = \"Team\"\n";
        let b = "active_environment = \"prod\"\nname = \"Team\"\n";
        // Đổi mỗi active_environment → hash normalized giống nhau (không sinh diff).
        assert_eq!(norm_hash("workspace.toml", a), norm_hash("workspace.toml", b));
        // Pull: nội dung server + active_environment local.
        let merged = merge_ws_toml("name = \"Team v2\"\n[[variables]]\nkey = \"x\"\nvalue = \"1\"\nenabled = true\n", Some(a));
        assert!(merged.contains("Team v2"));
        assert!(merged.contains("active_environment = \"dev\""));
    }

    #[test]
    fn scan_local_and_base_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("collections/api")).unwrap();
        std::fs::create_dir_all(root.join("environments")).unwrap();
        std::fs::write(root.join("workspace.toml"), "name = \"T\"\n").unwrap();
        std::fs::write(root.join("collections/api/r.toml"), "name = \"R\"\n").unwrap();
        std::fs::write(root.join("environments/dev.toml"), "name = \"dev\"\n").unwrap();
        std::fs::write(root.join(".apic-sync.json"), "{}").unwrap(); // dotfile: bỏ qua

        let scanned = scan_local(root).unwrap();
        let keys: Vec<&String> = scanned.keys().collect();
        assert_eq!(keys, vec!["collections/api/r.toml", "environments/dev.toml", "workspace.toml"]);

        let hashes: BTreeMap<String, String> =
            scanned.iter().map(|(p, c)| (p.clone(), norm_hash(p, c))).collect();
        save_base(root, hashes.clone()).unwrap();
        assert_eq!(load_base(root), hashes);
    }

    #[test]
    fn local_path_blocks_traversal() {
        let root = Path::new("/ws");
        assert!(local_path(root, "collections/a/r.toml").is_ok());
        assert!(local_path(root, "../evil.toml").is_err());
        assert!(local_path(root, "a/../../evil.toml").is_err());
        assert!(local_path(root, "/abs.toml").is_err());
        assert!(local_path(root, "c:/win.toml").is_err());
    }

    #[test]
    fn path_key_stable_and_fixed_len() {
        // PK trên server: sha256 hex — 64 ký tự ASCII, deterministic theo path.
        let k = path_key("collections/api/tạo-đơn.toml");
        assert_eq!(k.len(), 64);
        assert!(k.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(k, path_key("collections/api/tạo-đơn.toml"));
        assert_ne!(k, path_key("collections/api/khac.toml"));
    }

    #[test]
    fn create_table_sql_has_no_engine_clause() {
        // Guard hồi quy: không được ép storage engine (server MyISAM-only sẽ lỗi 1286).
        // Ghép needle để chính literal này không xuất hiện trong source được include.
        let src = include_str!("lib.rs");
        let needle = ["ENGINE", "=InnoDB"].concat();
        assert!(!src.contains(&needle), "không hardcode storage engine trong SQL");
    }

    #[test]
    fn conflict_path_shape() {
        let cp = conflict_path("collections/a/req.toml");
        assert!(cp.starts_with("collections/a/req-conflict-"));
        assert!(cp.ends_with(".toml"));
    }
}
