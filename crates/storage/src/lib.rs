//! # storage — Lưu trữ runtime bằng SQLite
//!
//! M0: chỉ có bảng `history`. Về sau (theo PLAN.md) thêm timeline analytics,
//! cookie jar, schema cache, benchmark/monitoring results.
//!
//! Files (collections/environments) KHÔNG nằm ở đây — chúng là source-of-truth
//! trên đĩa dạng TOML (xem docs/adr/0003).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ipc_types::{ExchangeRecord, HistoryEntry, RequestSpec, WorkspaceKind, WorkspaceMeta};
use rusqlite::Connection;
use uuid::Uuid;

/// Re-export để crate downstream (Tauri shell) không phụ thuộc trực tiếp vào rusqlite.
pub use rusqlite::Connection as SqliteConnection;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("time: {0}")]
    Time(String),
    #[error("không tìm thấy workspace: {0}")]
    NotFound(String),
}

const SCHEMA_VERSION: i64 = 4;

/// Mở/khởi tạo database tại `path`, chạy migration nếu cần.
pub fn init(path: &Path) -> Result<Connection, StorageError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Mở database in-memory (dùng cho test).
pub fn init_in_memory() -> Result<Connection, StorageError> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), StorageError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                method    TEXT NOT NULL,
                url       TEXT NOT NULL,
                status    INTEGER,
                total_ms  REAL,
                sent_at   INTEGER NOT NULL,
                spec_json TEXT NOT NULL,
                error     TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_history_sent_at ON history(sent_at DESC);",
        )?;
    }
    if current < 2 {
        // v2: lưu full ExchangeRecord (nén không cần cho M0 — cap ở tầng gọi) để restore cả response.
        conn.execute_batch("ALTER TABLE history ADD COLUMN record_json TEXT;")?;
    }
    if current < 3 {
        // v3: settings key-value (AI config...).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
    }
    if current < 4 {
        // v4: registry đa-workspace (metadata; nội dung vẫn là file TOML trên đĩa).
        // Partial unique index đảm bảo tối đa MỘT workspace active.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                path           TEXT NOT NULL,
                kind           TEXT NOT NULL DEFAULT 'personal',
                color          TEXT,
                is_active      INTEGER NOT NULL DEFAULT 0,
                created_at     INTEGER NOT NULL,
                last_opened_at INTEGER NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_workspaces_path ON workspaces(path);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_workspaces_active
                ON workspaces(is_active) WHERE is_active = 1;",
        )?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// Đặt một setting (upsert).
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// Lấy một setting.
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, StorageError> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

// ---------------------------------------------------------------------------
// Registry đa-workspace (v4)
// ---------------------------------------------------------------------------

const WS_COLS: &str = "id, name, path, kind, color, is_active, created_at, last_opened_at";

fn row_to_workspace(row: &rusqlite::Row) -> rusqlite::Result<WorkspaceMeta> {
    let kind_str: String = row.get(3)?;
    Ok(WorkspaceMeta {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        kind: WorkspaceKind::from_str_lossy(&kind_str),
        color: row.get(4)?,
        is_active: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        last_opened_at: row.get(7)?,
        available: true, // runtime — tầng command tính lại bằng path.exists()
    })
}

/// Liệt kê mọi workspace, mới mở gần nhất trước.
pub fn list_workspaces(conn: &Connection) -> Result<Vec<WorkspaceMeta>, StorageError> {
    let sql = format!("SELECT {WS_COLS} FROM workspaces ORDER BY last_opened_at DESC, name ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_workspace)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_workspace(conn: &Connection, id: &str) -> Result<Option<WorkspaceMeta>, StorageError> {
    use rusqlite::OptionalExtension;
    let sql = format!("SELECT {WS_COLS} FROM workspaces WHERE id = ?1");
    Ok(conn.query_row(&sql, [id], row_to_workspace).optional()?)
}

pub fn get_active_workspace(conn: &Connection) -> Result<Option<WorkspaceMeta>, StorageError> {
    use rusqlite::OptionalExtension;
    let sql = format!("SELECT {WS_COLS} FROM workspaces WHERE is_active = 1");
    Ok(conn.query_row(&sql, [], row_to_workspace).optional()?)
}

pub fn find_workspace_by_path(
    conn: &Connection,
    norm_path: &str,
) -> Result<Option<WorkspaceMeta>, StorageError> {
    use rusqlite::OptionalExtension;
    let sql = format!("SELECT {WS_COLS} FROM workspaces WHERE path = ?1");
    Ok(conn.query_row(&sql, [norm_path], row_to_workspace).optional()?)
}

/// Thêm workspace theo path (đã chuẩn hoá). Idempotent: nếu path đã có → trả hàng cũ.
/// KHÔNG activate — activation là bước riêng.
pub fn upsert_workspace_by_path(
    conn: &Connection,
    name: &str,
    norm_path: &str,
    kind: WorkspaceKind,
    color: Option<&str>,
) -> Result<WorkspaceMeta, StorageError> {
    if let Some(existing) = find_workspace_by_path(conn, norm_path)? {
        return Ok(existing);
    }
    let id = Uuid::new_v4().to_string();
    let now = now_ms()?;
    conn.execute(
        "INSERT INTO workspaces (id, name, path, kind, color, is_active, created_at, last_opened_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
        rusqlite::params![id, name, norm_path, kind.as_str(), color, now],
    )?;
    Ok(WorkspaceMeta {
        id,
        name: name.to_string(),
        path: norm_path.to_string(),
        kind,
        color: color.map(|s| s.to_string()),
        is_active: false,
        created_at: now,
        last_opened_at: now,
        available: true,
    })
}

/// Đặt workspace `id` làm active (transaction clear-then-set giữ invariant một active).
pub fn set_active_workspace(conn: &mut Connection, id: &str) -> Result<(), StorageError> {
    let now = now_ms()?;
    let tx = conn.transaction()?;
    tx.execute("UPDATE workspaces SET is_active = 0 WHERE is_active = 1", [])?;
    let n = tx.execute(
        "UPDATE workspaces SET is_active = 1, last_opened_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    )?;
    if n == 0 {
        return Err(StorageError::NotFound(id.to_string())); // tx drop = rollback
    }
    tx.commit()?;
    Ok(())
}

/// Cập nhật nhãn workspace. `None` = giữ nguyên; với `color`, `Some(None)` = xoá màu.
pub fn update_workspace(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    kind: Option<WorkspaceKind>,
    color: Option<Option<&str>>,
) -> Result<(), StorageError> {
    if let Some(name) = name {
        conn.execute(
            "UPDATE workspaces SET name = ?2 WHERE id = ?1",
            rusqlite::params![id, name],
        )?;
    }
    if let Some(kind) = kind {
        conn.execute(
            "UPDATE workspaces SET kind = ?2 WHERE id = ?1",
            rusqlite::params![id, kind.as_str()],
        )?;
    }
    if let Some(color) = color {
        conn.execute(
            "UPDATE workspaces SET color = ?2 WHERE id = ?1",
            rusqlite::params![id, color],
        )?;
    }
    Ok(())
}

/// Bump `last_opened_at` = bây giờ (dùng khi mở lại workspace).
pub fn touch_workspace(conn: &Connection, id: &str) -> Result<(), StorageError> {
    let now = now_ms()?;
    conn.execute(
        "UPDATE workspaces SET last_opened_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    )?;
    Ok(())
}

/// Gỡ workspace khỏi registry. KHÔNG xoá thư mục/file trên đĩa.
pub fn remove_workspace(conn: &Connection, id: &str) -> Result<(), StorageError> {
    conn.execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
    Ok(())
}

pub fn count_workspaces(conn: &Connection) -> Result<i64, StorageError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0))?)
}

fn now_ms() -> Result<i64, StorageError> {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| StorageError::Time(e.to_string()))?;
    Ok(d.as_millis() as i64)
}

/// Lưu một exchange vào history; trả về entry đã lưu (kèm id).
pub fn save_exchange(
    conn: &Connection,
    spec: &RequestSpec,
    record: &ExchangeRecord,
) -> Result<HistoryEntry, StorageError> {
    let sent_at = now_ms()?;
    let status = record.response.as_ref().map(|r| r.status);
    let total_ms = record.timings.total_ms;
    let spec_json = serde_json::to_string(spec)?;
    let record_json = serde_json::to_string(record)?;
    let error = record.error.as_ref().map(|e| e.message.clone());

    conn.execute(
        "INSERT INTO history (method, url, status, total_ms, sent_at, spec_json, record_json, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            record.method,
            record.final_url,
            status,
            total_ms,
            sent_at,
            spec_json,
            record_json,
            error,
        ],
    )?;
    let id = conn.last_insert_rowid();

    Ok(HistoryEntry {
        id,
        method: record.method.clone(),
        url: record.final_url.clone(),
        status,
        total_ms,
        sent_at,
        spec_json,
        error,
    })
}

/// Lấy `limit` entry gần nhất (mới → cũ).
pub fn list_history(conn: &Connection, limit: u32) -> Result<Vec<HistoryEntry>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, method, url, status, total_ms, sent_at, spec_json, error
         FROM history ORDER BY sent_at DESC, id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            method: row.get(1)?,
            url: row.get(2)?,
            status: row.get::<_, Option<u16>>(3)?,
            total_ms: row.get::<_, Option<f64>>(4)?,
            sent_at: row.get(5)?,
            spec_json: row.get(6)?,
            error: row.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Lấy full `ExchangeRecord` của một history entry (để xem lại response cũ).
pub fn load_record(conn: &Connection, id: i64) -> Result<Option<ExchangeRecord>, StorageError> {
    use rusqlite::OptionalExtension;
    let json: Option<String> = conn
        .query_row(
            "SELECT record_json FROM history WHERE id = ?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    match json {
        Some(s) if !s.is_empty() => Ok(Some(serde_json::from_str(&s)?)),
        _ => Ok(None),
    }
}

/// Xoá toàn bộ history.
pub fn clear_history(conn: &Connection) -> Result<(), StorageError> {
    conn.execute("DELETE FROM history", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::{RequestSpec, Timings};

    fn dummy_record() -> ExchangeRecord {
        ExchangeRecord {
            final_url: "https://x.com/".into(),
            method: "GET".into(),
            response: None,
            timings: Timings {
                total_ms: Some(12.5),
                ..Default::default()
            },
            tls: None,
            redirects: vec![],
            error: None,
        }
    }

    #[test]
    fn save_and_list_roundtrip() {
        let conn = init_in_memory().unwrap();
        let spec = RequestSpec::get("https://x.com/");
        let saved = save_exchange(&conn, &spec, &dummy_record()).unwrap();
        assert!(saved.id > 0);

        let list = list_history(&conn, 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].url, "https://x.com/");
        assert_eq!(list[0].total_ms, Some(12.5));

        // spec_json restore được.
        let restored: RequestSpec = serde_json::from_str(&list[0].spec_json).unwrap();
        assert_eq!(restored.url, "https://x.com/");

        // load_record trả về full ExchangeRecord.
        let rec = load_record(&conn, saved.id).unwrap().unwrap();
        assert_eq!(rec.final_url, "https://x.com/");
        assert_eq!(rec.timings.total_ms, Some(12.5));
        assert!(load_record(&conn, 99999).unwrap().is_none());
    }

    #[test]
    fn settings_roundtrip() {
        let conn = init_in_memory().unwrap();
        assert_eq!(get_setting(&conn, "ai.provider").unwrap(), None);
        set_setting(&conn, "ai.provider", "anthropic").unwrap();
        set_setting(&conn, "ai.provider", "open_ai").unwrap(); // upsert
        assert_eq!(get_setting(&conn, "ai.provider").unwrap().as_deref(), Some("open_ai"));
    }

    #[test]
    fn migrate_idempotent() {
        // init đã migrate; gọi lại 2 lần nữa vẫn không lỗi.
        let conn = init_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        assert_eq!(count_workspaces(&conn).unwrap(), 0);
    }

    #[test]
    fn workspace_registry_crud() {
        use ipc_types::WorkspaceKind;
        let mut conn = init_in_memory().unwrap();
        assert_eq!(count_workspaces(&conn).unwrap(), 0);

        let a = upsert_workspace_by_path(&conn, "A", "/ws/a", WorkspaceKind::Personal, None).unwrap();
        let b =
            upsert_workspace_by_path(&conn, "B", "/ws/b", WorkspaceKind::Shared, Some("#4f8cff"))
                .unwrap();
        assert_eq!(count_workspaces(&conn).unwrap(), 2);
        assert!(!a.is_active && !b.is_active);

        // Dedup theo path → trả hàng cũ, không tạo mới.
        let a2 =
            upsert_workspace_by_path(&conn, "A-again", "/ws/a", WorkspaceKind::Personal, None)
                .unwrap();
        assert_eq!(a2.id, a.id);
        assert_eq!(a2.name, "A");
        assert_eq!(count_workspaces(&conn).unwrap(), 2);

        // Invariant một active: set b sau a → chỉ b active.
        set_active_workspace(&mut conn, &a.id).unwrap();
        assert_eq!(get_active_workspace(&conn).unwrap().unwrap().id, a.id);
        set_active_workspace(&mut conn, &b.id).unwrap();
        assert_eq!(get_active_workspace(&conn).unwrap().unwrap().id, b.id);
        assert!(!get_workspace(&conn, &a.id).unwrap().unwrap().is_active);

        // list trả đủ cả hai; b là active. (Không assert thứ tự vì last_opened_at
        // có thể trùng ở độ phân giải ms khi hai lần set_active sát nhau.)
        let list = list_workspaces(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|w| w.id == b.id && w.is_active));
        assert!(list.iter().any(|w| w.id == a.id && !w.is_active));

        // update rename + xoá màu.
        update_workspace(&conn, &b.id, Some("B2"), None, Some(None)).unwrap();
        let b3 = get_workspace(&conn, &b.id).unwrap().unwrap();
        assert_eq!(b3.name, "B2");
        assert_eq!(b3.color, None);

        // set active id không tồn tại → lỗi + rollback (b vẫn active).
        assert!(set_active_workspace(&mut conn, "nope").is_err());
        assert_eq!(get_active_workspace(&conn).unwrap().unwrap().id, b.id);

        // remove không đụng active b; gỡ a.
        remove_workspace(&conn, &a.id).unwrap();
        assert_eq!(count_workspaces(&conn).unwrap(), 1);
        assert!(get_workspace(&conn, &a.id).unwrap().is_none());
    }

    #[test]
    fn clear_empties_history() {
        let conn = init_in_memory().unwrap();
        save_exchange(&conn, &RequestSpec::get("https://a"), &dummy_record()).unwrap();
        clear_history(&conn).unwrap();
        assert_eq!(list_history(&conn, 10).unwrap().len(), 0);
    }
}
