//! Secret scrubber — BẮT BUỘC chạy trước mọi payload gửi lên AI provider.
//!
//! Nguyên tắc: không bao giờ gửi giá trị secret. Chỉ gửi tên biến, và redact
//! Authorization/Cookie/API-key. Xem docs/adr/0005 + PLAN.md risk D8.

/// Thay mọi lần xuất hiện của giá trị secret bằng «secret».
/// Chỉ mask chuỗi đủ dài (>=4) để tránh xoá nhầm ký tự vụn.
pub fn scrub_text(input: &str, secret_values: &[String]) -> String {
    let mut out = input.to_string();
    for s in secret_values {
        if s.len() >= 4 {
            out = out.replace(s, "«secret»");
        }
    }
    out
}

/// Trả về giá trị header an toàn để đưa vào context (redact credential).
pub fn redact_header_value(key: &str, value: &str) -> String {
    let k = key.to_ascii_lowercase();
    if k == "authorization" {
        // Giữ scheme (Bearer/Basic), che credential.
        match value.split_once(' ') {
            Some((scheme, _)) => format!("{scheme} «redacted»"),
            None => "«redacted»".to_string(),
        }
    } else if k == "cookie" || k == "set-cookie" || k == "x-api-key" || k == "api-key" {
        "«redacted»".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_value_never_leaks() {
        let secret = "sk-supersecret-token-12345";
        let ctx = format!("Authorization: Bearer {secret}\nbody: {{\"t\":\"{secret}\"}}");
        let scrubbed = scrub_text(&ctx, &[secret.to_string()]);
        assert!(
            !scrubbed.contains(secret),
            "secret bị lọt vào payload gửi AI: {scrubbed}"
        );
        assert!(scrubbed.contains("«secret»"));
    }

    #[test]
    fn redacts_authorization_keeps_scheme() {
        assert_eq!(redact_header_value("Authorization", "Bearer abc.def.ghi"), "Bearer «redacted»");
        assert_eq!(redact_header_value("authorization", "raw-token"), "«redacted»");
        assert_eq!(redact_header_value("Cookie", "sid=xyz"), "«redacted»");
        assert_eq!(redact_header_value("Accept", "application/json"), "application/json");
    }

    #[test]
    fn short_values_not_masked() {
        // Tránh mask nhầm giá trị quá ngắn (vd "1", "ok").
        assert_eq!(scrub_text("value is 1", &["1".to_string()]), "value is 1");
    }
}
