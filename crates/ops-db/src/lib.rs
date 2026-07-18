//! # ops-db — Query Database (read-only)
//!
//! An toàn là ưu tiên số 1: mọi câu lệnh phải là SELECT/EXPLAIN (đọc). DML/DDL
//! bị chặn ở tầng parse (sqlparser) TRƯỚC khi chạm database. Dùng sqlx `Any`
//! để hỗ trợ Postgres/MySQL/SQLite qua cùng một API.

use std::time::{Duration, Instant};

use ipc_types::DbQueryResult;
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlx::{Column, Row};

const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Kiểm tra câu lệnh chỉ đọc. Trả Err với lý do nếu có bất kỳ statement ghi.
pub fn is_read_only(sql: &str) -> Result<(), String> {
    let stmts = Parser::parse_sql(&GenericDialect {}, sql)
        .map_err(|e| format!("SQL không parse được: {e}"))?;
    if stmts.is_empty() {
        return Err("Câu lệnh rỗng".to_string());
    }
    for s in &stmts {
        let ok = matches!(s, Statement::Query(_) | Statement::Explain { .. } | Statement::ExplainTable { .. });
        if !ok {
            return Err(format!(
                "Chỉ cho phép SELECT/EXPLAIN (đọc). Bị chặn: {}",
                stmt_kind(s)
            ));
        }
    }
    Ok(())
}

fn stmt_kind(s: &Statement) -> &'static str {
    match s {
        Statement::Insert { .. } => "INSERT",
        Statement::Update { .. } => "UPDATE",
        Statement::Delete { .. } => "DELETE",
        Statement::Drop { .. } => "DROP",
        Statement::Truncate { .. } => "TRUNCATE",
        Statement::AlterTable { .. } => "ALTER",
        Statement::CreateTable { .. } | Statement::CreateView { .. } => "CREATE",
        _ => "lệnh ghi/khác",
    }
}

/// Chạy một truy vấn read-only. Không bao giờ panic — lỗi nằm trong `error`.
pub async fn query(url: &str, sql: &str) -> DbQueryResult {
    let started = Instant::now();
    let mut result = DbQueryResult::default();

    if let Err(e) = is_read_only(sql) {
        result.error = Some(e);
        result.elapsed_ms = ms(started);
        return result;
    }

    match tokio::time::timeout(QUERY_TIMEOUT, run(url, sql)).await {
        Ok(Ok((columns, rows))) => {
            result.row_count = rows.len() as u64;
            result.columns = columns;
            result.rows = rows;
        }
        Ok(Err(e)) => result.error = Some(e),
        Err(_) => result.error = Some(format!("Query quá {}s", QUERY_TIMEOUT.as_secs())),
    }
    result.elapsed_ms = ms(started);
    result
}

async fn run(url: &str, sql: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    sqlx::any::install_default_drivers();
    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .map_err(|e| format!("Kết nối DB lỗi: {e}"))?;

    let rows = sqlx::query(sql)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Query lỗi: {e}"))?;
    pool.close().await;

    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();

    let out: Vec<Vec<String>> = rows
        .iter()
        .map(|row| (0..row.len()).map(|i| cell(row, i)).collect())
        .collect();

    Ok((columns, out))
}

/// Stringify một ô — thử lần lượt các kiểu mà sqlx::Any hỗ trợ.
fn cell(row: &sqlx::any::AnyRow, i: usize) -> String {
    if let Ok(v) = row.try_get::<Option<String>, _>(i) {
        return v.unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return v.map(|b| format!("<{} bytes>", b.len())).unwrap_or_else(|| "NULL".into());
    }
    "<?>".into()
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_select_and_explain() {
        assert!(is_read_only("SELECT * FROM users WHERE id = 1").is_ok());
        assert!(is_read_only("select count(*) from orders").is_ok());
        assert!(is_read_only("EXPLAIN SELECT 1").is_ok());
    }

    #[test]
    fn blocks_writes() {
        assert!(is_read_only("INSERT INTO t VALUES (1)").is_err());
        assert!(is_read_only("UPDATE t SET a=1").is_err());
        assert!(is_read_only("DELETE FROM t").is_err());
        assert!(is_read_only("DROP TABLE t").is_err());
        assert!(is_read_only("TRUNCATE t").is_err());
        // Chèn lệnh ghi lẫn với SELECT cũng bị chặn.
        assert!(is_read_only("SELECT 1; DELETE FROM t").is_err());
    }

    #[tokio::test]
    async fn sqlite_query_pipeline() {
        let r = query("sqlite::memory:", "SELECT 1 AS n, 'hi' AS s").await;
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.columns, vec!["n".to_string(), "s".to_string()]);
        assert_eq!(r.row_count, 1);
        assert_eq!(r.rows[0][1], "hi");
    }

    #[tokio::test]
    async fn write_blocked_before_connect() {
        // url rác — nếu guard chạy trước thì không cần kết nối, vẫn trả lỗi read-only.
        let r = query("postgres://invalid", "DELETE FROM t").await;
        assert!(r.error.as_ref().unwrap().contains("Chỉ cho phép"));
    }
}
