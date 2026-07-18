//! # secrets — Secret của biến environment nằm trong OS keychain, KHÔNG trong file.
//!
//! Secret env có hai cách định danh:
//! - **Legacy** (app-global): `(environment, key)` → account `"env\u{1f}key"`.
//! - **Scoped** (theo workspace, v4): `(scope, environment, key)` → account
//!   `"scope\u{1f}env\u{1f}key"`, với `scope = workspace_id`. Tránh đụng độ khi hai
//!   workspace có env trùng tên. `get_scoped_or_legacy` di trú mượt: đọc scoped trước,
//!   fallback legacy + copy-forward (KHÔNG xoá legacy → rollback-safe).
//!
//! AI key / Postman key giữ định danh legacy có chủ đích (app-global, không theo workspace).

const SERVICE: &str = "com.apicompanion.desktop";
/// Ký tự phân tách các thành phần trong account key (Unit Separator).
const SEP: char = '\u{1f}';

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("keyring: {0}")]
    Keyring(#[from] keyring::Error),
}

type Result<T> = std::result::Result<T, SecretError>;

fn account(env: &str, key: &str) -> String {
    format!("{env}{SEP}{key}")
}

fn scoped_account(scope: &str, env: &str, key: &str) -> String {
    format!("{scope}{SEP}{env}{SEP}{key}")
}

fn entry(env: &str, key: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, &account(env, key))?)
}

fn scoped_entry(scope: &str, env: &str, key: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, &scoped_account(scope, env, key))?)
}

/// Lưu (hoặc cập nhật) giá trị secret.
pub fn set_secret(env: &str, key: &str, value: &str) -> Result<()> {
    entry(env, key)?.set_password(value)?;
    Ok(())
}

/// Lấy giá trị secret; `None` nếu chưa được đặt.
pub fn get_secret(env: &str, key: &str) -> Result<Option<String>> {
    match entry(env, key)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Xoá một secret (bỏ qua nếu không tồn tại).
pub fn delete_secret(env: &str, key: &str) -> Result<()> {
    match entry(env, key)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Scoped theo workspace (v4)
// ---------------------------------------------------------------------------

/// Lưu secret scoped theo `(scope=workspace_id, env, key)`.
pub fn set_scoped(scope: &str, env: &str, key: &str, value: &str) -> Result<()> {
    scoped_entry(scope, env, key)?.set_password(value)?;
    Ok(())
}

/// Lấy secret scoped; `None` nếu chưa đặt (không fallback legacy).
pub fn get_scoped(scope: &str, env: &str, key: &str) -> Result<Option<String>> {
    match scoped_entry(scope, env, key)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Xoá secret scoped (bỏ qua nếu không tồn tại).
pub fn delete_scoped(scope: &str, env: &str, key: &str) -> Result<()> {
    match scoped_entry(scope, env, key)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Đọc secret scoped; nếu chưa có → thử legacy `(env,key)`. Nếu legacy có → copy-forward
/// sang scoped rồi trả (KHÔNG xoá legacy → downgrade/rollback vẫn đọc được).
pub fn get_scoped_or_legacy(scope: &str, env: &str, key: &str) -> Result<Option<String>> {
    if let Some(v) = get_scoped(scope, env, key)? {
        return Ok(Some(v));
    }
    match get_secret(env, key)? {
        Some(v) => {
            let _ = set_scoped(scope, env, key, &v); // best-effort, không chặn việc đọc
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test THUẦN account-key (không đụng keychain — theo convention của crate).
    // Chứng minh tính chất cốt lõi: namespacing loại bỏ đụng độ giữa các workspace.
    #[test]
    fn scoped_accounts_are_collision_free() {
        // Hai workspace cùng (env,key) → account KHÁC nhau.
        let a = scoped_account("ws1", "staging", "token");
        let b = scoped_account("ws2", "staging", "token");
        assert_ne!(a, b);

        // Scoped khác legacy (không đè lên nhau).
        assert_ne!(a, account("staging", "token"));

        // Cùng bộ (scope,env,key) → ổn định.
        assert_eq!(a, scoped_account("ws1", "staging", "token"));

        // Định dạng đúng thứ tự scope,env,key.
        assert_eq!(a, format!("ws1{SEP}staging{SEP}token"));
        assert_eq!(account("staging", "token"), format!("staging{SEP}token"));
    }
}
