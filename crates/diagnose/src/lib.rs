//! # diagnose — Chẩn đoán lỗi bằng rule tĩnh (không cần AI)
//!
//! Chạy tức thì khi response ≥ 400 hoặc lỗi mạng — vừa là fallback khi chưa có
//! API key, vừa là lớp "instant" hiện trước khi AI trả lời (PLAN.md §6.3).

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use ipc_types::{Auth, DiagnoseFix, ExchangeRecord, Hypothesis, KeyValue, RequestBody, RequestSpec};

fn h(cause: &str, confidence: &str, evidence: Vec<String>, fix: Option<DiagnoseFix>) -> Hypothesis {
    Hypothesis {
        cause: cause.to_string(),
        evidence,
        confidence: confidence.to_string(),
        fix,
        source: "rule".to_string(),
    }
}

fn header<'a>(rec: &'a ExchangeRecord, name: &str) -> Option<&'a str> {
    rec.response
        .as_ref()?
        .headers
        .iter()
        .find(|h| h.key.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Giải mã `exp` của một JWT (không verify). None nếu không phải JWT.
fn jwt_exp(token: &str) -> Option<i64> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("exp")?.as_i64()
}

/// Chạy toàn bộ rule, trả về danh sách giả thuyết (có thể rỗng).
pub fn diagnose(spec: &RequestSpec, rec: &ExchangeRecord) -> Vec<Hypothesis> {
    let mut out = Vec::new();

    // 0. Biến chưa resolve trong URL.
    if rec.final_url.contains("{{") || spec.url.contains("{{") {
        out.push(h(
            "URL còn biến chưa resolve",
            "high",
            vec![format!("URL: {}", spec.url)],
            Some(DiagnoseFix {
                description: "Chọn environment có biến tương ứng, hoặc khai báo biến trước khi gửi."
                    .into(),
                set_headers: vec![],
            }),
        ));
    }

    // 1. Lỗi mạng (không có response).
    if let Some(err) = &rec.error {
        out.push(h(
            "Không kết nối được tới server",
            "high",
            vec![format!("[{:?}] {}", err.code, err.message)],
            Some(DiagnoseFix {
                description: "Kiểm tra URL/host/port, mạng, hoặc server đang chạy chưa.".into(),
                set_headers: vec![],
            }),
        ));
        return out;
    }

    let Some(resp) = &rec.response else { return out };
    let status = resp.status;
    let body = resp.body.text.as_deref().unwrap_or("");

    // 2. 401/403 — auth.
    if status == 401 || status == 403 {
        match &spec.auth {
            Auth::None | Auth::Inherit => out.push(h(
                "Request thiếu Authorization",
                "high",
                vec![format!("Status {status}, chưa cấu hình auth")],
                Some(DiagnoseFix {
                    description: "Thêm Bearer token / Basic / API key ở tab Auth.".into(),
                    set_headers: vec![],
                }),
            )),
            Auth::Bearer { token } => {
                if let Some(exp) = jwt_exp(token) {
                    if exp < now() {
                        out.push(h(
                            "Bearer token (JWT) đã hết hạn",
                            "high",
                            vec![format!("exp = {exp} (quá khứ so với hiện tại {})", now())],
                            Some(DiagnoseFix {
                                description: "Lấy token mới / refresh rồi thử lại.".into(),
                                set_headers: vec![],
                            }),
                        ));
                    }
                } else {
                    out.push(h(
                        "Token có thể sai hoặc thiếu quyền",
                        "medium",
                        vec![format!("Status {status} với Bearer token")],
                        None,
                    ));
                }
            }
            _ => out.push(h(
                "Xác thực thất bại hoặc thiếu quyền",
                "medium",
                vec![format!("Status {status}")],
                None,
            )),
        }
        if let Some(wa) = header(rec, "www-authenticate") {
            out.push(h(
                "Server yêu cầu xác thực cụ thể",
                "medium",
                vec![format!("WWW-Authenticate: {wa}")],
                None,
            ));
        }
    }

    // 3. 400/415 — thiếu Content-Type khi có body.
    if (status == 400 || status == 415) && !matches!(spec.body, RequestBody::None) {
        let has_ct = spec
            .headers
            .iter()
            .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
        if !has_ct {
            out.push(h(
                "Có body nhưng thiếu header Content-Type",
                "high",
                vec![format!("Status {status}, body kiểu {:?}", body_kind(&spec.body))],
                Some(DiagnoseFix {
                    description: "Thêm Content-Type phù hợp (vd application/json).".into(),
                    set_headers: vec![KeyValue {
                        key: "Content-Type".into(),
                        value: "application/json".into(),
                        enabled: true,
                    }],
                }),
            ));
        }
    }

    // 4. 405 — method không được phép.
    if status == 405 {
        let allow = header(rec, "allow").unwrap_or("(không có header Allow)");
        out.push(h(
            "HTTP method không được endpoint chấp nhận",
            "high",
            vec![format!("Allow: {allow}")],
            None,
        ));
    }

    // 5. 429 — rate limit.
    if status == 429 {
        let retry = header(rec, "retry-after").unwrap_or("?");
        out.push(h(
            "Bị giới hạn tần suất (rate limit)",
            "high",
            vec![format!("Retry-After: {retry}s")],
            None,
        ));
    }

    // 6. 404 — endpoint sai.
    if status == 404 {
        let mut ev = vec![format!("Không tìm thấy: {}", rec.final_url)];
        if rec.final_url.contains("//") && !rec.final_url.contains("://") {
            ev.push("URL có dấu '//' lặp — có thể ghép path sai.".into());
        }
        out.push(h("Endpoint/URL không tồn tại", "medium", ev, None));
    }

    // 7. 5xx — lỗi server, soi body tìm exception.
    if status >= 500 {
        let mut ev = vec![format!("Status {status}")];
        for kw in ["NullPointer", "Exception", "Traceback", "SQLSTATE", "ECONNREFUSED", "timeout"] {
            if body.contains(kw) {
                ev.push(format!("Body chứa '{kw}' → có thể là nguyên nhân"));
            }
        }
        out.push(h(
            "Lỗi phía server",
            if ev.len() > 1 { "medium" } else { "low" },
            ev,
            None,
        ));
    }

    out
}

fn body_kind(b: &RequestBody) -> &'static str {
    match b {
        RequestBody::None => "none",
        RequestBody::Text { .. } => "text",
        RequestBody::Form { .. } => "form",
        RequestBody::Multipart { .. } => "multipart",
        RequestBody::BinaryFile { .. } => "binary",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::{ResponseBody, ResponseRecord, Timings};

    fn rec(status: u16, headers: Vec<(&str, &str)>, body: &str) -> ExchangeRecord {
        ExchangeRecord {
            final_url: "https://x/api".into(),
            method: "POST".into(),
            response: Some(ResponseRecord {
                status,
                status_text: "".into(),
                http_version: "HTTP/1.1".into(),
                headers: headers
                    .into_iter()
                    .map(|(k, v)| KeyValue { key: k.into(), value: v.into(), enabled: true })
                    .collect(),
                body: ResponseBody {
                    text: Some(body.into()),
                    base64: None,
                    size: 0,
                    raw_size: 0,
                    content_encoding: None,
                },
                remote_addr: None,
            }),
            timings: Timings::default(),
            tls: None,
            redirects: vec![],
            error: None,
        }
    }

    #[test]
    fn no_auth_403() {
        let spec = RequestSpec::get("https://x/api");
        let hyp = diagnose(&spec, &rec(403, vec![], ""));
        assert!(hyp.iter().any(|h| h.cause.contains("thiếu Authorization")));
    }

    #[test]
    fn expired_jwt() {
        // exp=1 (đã hết hạn từ 1970)
        let jwt = "h.eyJleHAiOjF9.s"; // {"exp":1}
        let mut spec = RequestSpec::get("https://x/api");
        spec.auth = Auth::Bearer { token: jwt.into() };
        let hyp = diagnose(&spec, &rec(401, vec![], ""));
        assert!(hyp.iter().any(|h| h.cause.contains("hết hạn")));
    }

    #[test]
    fn missing_content_type_on_body() {
        let mut spec = RequestSpec::get("https://x/api");
        spec.body = RequestBody::Text { content: "{}".into(), content_type: None };
        let hyp = diagnose(&spec, &rec(400, vec![], ""));
        let ct = hyp.iter().find(|h| h.cause.contains("Content-Type")).unwrap();
        assert!(ct.fix.as_ref().unwrap().set_headers.iter().any(|h| h.key == "Content-Type"));
    }

    #[test]
    fn server_error_surfaces_exception() {
        let spec = RequestSpec::get("https://x/api");
        let hyp = diagnose(&spec, &rec(500, vec![], "java.lang.NullPointerException at ..."));
        assert!(hyp.iter().any(|h| h.evidence.iter().any(|e| e.contains("NullPointer"))));
    }
}
