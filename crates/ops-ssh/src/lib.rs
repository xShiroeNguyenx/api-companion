//! # ops-ssh — Chạy lệnh qua SSH
//!
//! Dùng `ssh` binary của hệ thống (OpenSSH có sẵn trên Windows 11/macOS/Linux):
//! tận dụng ssh config, key, agent, known_hosts của user. Key/agent auth chạy
//! non-interactive tốt; password auth cần `sshpass` (Linux/mac).
//!
//! Ghi chú: bản pure-Rust (russh) + PTY streaming là hạng mục nâng cấp (ICEBOX).

use std::process::Stdio;
use std::time::{Duration, Instant};

use ipc_types::{Connection, SshResult};
use tokio::process::Command;

const EXEC_TIMEOUT: Duration = Duration::from_secs(60);

/// Chạy `command` trên host của `conn`. Không panic — lỗi nằm trong `error`.
pub async fn exec(conn: &Connection, password: Option<&str>, command: &str) -> SshResult {
    let started = Instant::now();
    let mut result = SshResult::default();

    if conn.host.trim().is_empty() || conn.username.trim().is_empty() {
        result.error = Some("Thiếu host hoặc username".into());
        result.elapsed_ms = ms(started);
        return result;
    }

    let target = format!("{}@{}", conn.username, conn.host);
    let use_password = conn.auth_method.as_deref() == Some("password") && password.is_some();

    // Args cho ssh.
    let mut ssh_args: Vec<String> = vec![
        "-p".into(),
        conn.port.to_string(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "ConnectTimeout=15".into(),
    ];
    if !use_password {
        // Không hỏi password để tránh treo (dùng key/agent).
        ssh_args.push("-o".into());
        ssh_args.push("BatchMode=yes".into());
    }
    if let Some(kp) = &conn.key_path {
        if !kp.trim().is_empty() {
            ssh_args.push("-i".into());
            ssh_args.push(kp.clone());
        }
    }
    ssh_args.push(target);
    ssh_args.push(command.to_string());

    let mut cmd = if use_password {
        // sshpass -p <pw> ssh <args>
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(password.unwrap()).arg("ssh").args(&ssh_args);
        c
    } else {
        let mut c = Command::new("ssh");
        c.args(&ssh_args);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let fut = cmd.output();
    match tokio::time::timeout(EXEC_TIMEOUT, fut).await {
        Ok(Ok(output)) => {
            result.stdout = String::from_utf8_lossy(&output.stdout).to_string();
            result.stderr = String::from_utf8_lossy(&output.stderr).to_string();
            result.exit_code = output.status.code();
        }
        Ok(Err(e)) => {
            result.error = Some(if e.kind() == std::io::ErrorKind::NotFound {
                if use_password {
                    "Không tìm thấy 'sshpass' (cần cho password auth). Hãy dùng key auth.".into()
                } else {
                    "Không tìm thấy 'ssh' trên hệ thống.".into()
                }
            } else {
                format!("Chạy ssh lỗi: {e}")
            });
        }
        Err(_) => result.error = Some(format!("Lệnh SSH quá {}s", EXEC_TIMEOUT.as_secs())),
    }
    result.elapsed_ms = ms(started);
    result
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::ConnectionKind;

    fn ssh_conn() -> Connection {
        Connection {
            id: "t".into(),
            name: "t".into(),
            kind: ConnectionKind::Ssh,
            host: "".into(),
            port: 22,
            username: "".into(),
            db_driver: None,
            database: None,
            auth_method: Some("key".into()),
            key_path: None,
            has_secret: false,
        }
    }

    #[tokio::test]
    async fn empty_host_is_error() {
        let r = exec(&ssh_conn(), None, "echo hi").await;
        assert!(r.error.is_some());
    }
}
