//! # curl-tools — Import/export lệnh `curl` ↔ `RequestSpec`
//!
//! Logic thuần (không I/O) nên dễ test. Hỗ trợ các cờ curl phổ biến nhất
//! mà dev hay copy từ DevTools/Postman.

use ipc_types::{ApiKeyLocation, Auth, HttpMethod, KeyValue, RequestBody, RequestSpec};

/// Lỗi khi parse curl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurlError {
    Empty,
    NoUrl,
    Syntax(String),
}

impl std::fmt::Display for CurlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurlError::Empty => write!(f, "Lệnh rỗng"),
            CurlError::NoUrl => write!(f, "Không tìm thấy URL trong lệnh curl"),
            CurlError::Syntax(s) => write!(f, "Cú pháp lỗi: {s}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Import: curl → RequestSpec
// ---------------------------------------------------------------------------

/// Parse một lệnh `curl` thành `RequestSpec`.
pub fn parse(input: &str) -> Result<RequestSpec, CurlError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(CurlError::Empty);
    }

    let mut idx = 0;
    // Bỏ qua token "curl" ở đầu nếu có.
    if tokens[0].eq_ignore_ascii_case("curl") {
        idx = 1;
    }

    let mut url: Option<String> = None;
    let mut method: Option<String> = None;
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut data: Vec<String> = Vec::new();
    let mut form_parts: Vec<(String, String)> = Vec::new();
    let mut basic_user: Option<String> = None;
    let mut verify_tls = true;
    let mut timeout_ms: Option<u64> = None;
    let mut head = false;

    while idx < tokens.len() {
        let tok = &tokens[idx];
        let next = |i: &mut usize| -> Option<String> {
            *i += 1;
            tokens.get(*i).cloned()
        };
        match tok.as_str() {
            "-X" | "--request" => {
                method = next(&mut idx);
            }
            "--url" => {
                url = next(&mut idx);
            }
            "-H" | "--header" => {
                if let Some(h) = next(&mut idx) {
                    if let Some((k, v)) = h.split_once(':') {
                        headers.push(kv(k.trim(), v.trim()));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-ascii" | "--data-binary"
            | "--data-urlencode" => {
                if let Some(d) = next(&mut idx) {
                    data.push(d);
                }
            }
            "-F" | "--form" => {
                if let Some(f) = next(&mut idx) {
                    if let Some((k, v)) = f.split_once('=') {
                        form_parts.push((k.to_string(), v.to_string()));
                    }
                }
            }
            "-u" | "--user" => {
                basic_user = next(&mut idx);
            }
            "-b" | "--cookie" => {
                if let Some(c) = next(&mut idx) {
                    headers.push(kv("Cookie", &c));
                }
            }
            "-A" | "--user-agent" => {
                if let Some(a) = next(&mut idx) {
                    headers.push(kv("User-Agent", &a));
                }
            }
            "-e" | "--referer" => {
                if let Some(r) = next(&mut idx) {
                    headers.push(kv("Referer", &r));
                }
            }
            "-m" | "--max-time" | "--connect-timeout" => {
                if let Some(t) = next(&mut idx) {
                    if let Ok(secs) = t.parse::<f64>() {
                        timeout_ms = Some((secs * 1000.0) as u64);
                    }
                }
            }
            "-k" | "--insecure" => verify_tls = false,
            "-I" | "--head" => head = true,
            // Cờ không có tham số — bỏ qua an toàn.
            "-L" | "--location" | "-s" | "--silent" | "-S" | "--show-error" | "-v" | "--verbose"
            | "--compressed" | "-i" | "--include" | "-g" | "-#" | "--progress-bar" | "-f"
            | "--fail" | "-0" | "--http1.0" | "--http1.1" => {}
            other => {
                if other.starts_with('-') {
                    // Cờ lạ có thể kèm giá trị — bỏ qua cờ, giữ nguyên tokenizer.
                } else if url.is_none() {
                    url = Some(other.to_string());
                }
            }
        }
        idx += 1;
    }

    let url = url.ok_or(CurlError::NoUrl)?;

    // Suy ra method.
    let method = method.unwrap_or_else(|| {
        if head {
            "HEAD".into()
        } else if !data.is_empty() || !form_parts.is_empty() {
            "POST".into()
        } else {
            "GET".into()
        }
    });

    // Auth từ -u.
    let auth = match basic_user {
        Some(u) => {
            let (username, password) = match u.split_once(':') {
                Some((a, b)) => (a.to_string(), b.to_string()),
                None => (u, String::new()),
            };
            Auth::Basic { username, password }
        }
        None => Auth::None,
    };

    // Body.
    let has_content_type = headers
        .iter()
        .any(|h| h.key.eq_ignore_ascii_case("content-type"));
    let body = if !form_parts.is_empty() {
        RequestBody::Multipart {
            parts: form_parts
                .into_iter()
                .map(|(name, v)| {
                    if let Some(path) = v.strip_prefix('@') {
                        ipc_types::MultipartPart {
                            name,
                            value: String::new(),
                            file_path: Some(path.to_string()),
                            content_type: None,
                            enabled: true,
                        }
                    } else {
                        ipc_types::MultipartPart {
                            name,
                            value: v,
                            file_path: None,
                            content_type: None,
                            enabled: true,
                        }
                    }
                })
                .collect(),
        }
    } else if !data.is_empty() {
        let content = data.join("&");
        let content_type = if has_content_type {
            None // để header của user quyết định
        } else {
            Some("application/x-www-form-urlencoded".to_string())
        };
        RequestBody::Text { content, content_type }
    } else {
        RequestBody::None
    };

    Ok(RequestSpec {
        method: HttpMethod::new(method),
        url,
        query: Vec::new(),
        headers,
        body,
        auth,
        timeout_ms,
        follow_redirects: true,
        max_redirects: 10,
        verify_tls,
        assertions: Vec::new(),
    })
}

fn kv(k: &str, v: &str) -> KeyValue {
    KeyValue {
        key: k.to_string(),
        value: v.to_string(),
        enabled: true,
    }
}

/// Tách chuỗi thành token, tôn trọng nháy đơn/kép và nối dòng bằng `\`.
fn tokenize(input: &str) -> Result<Vec<String>, CurlError> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut has_token = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Nối dòng: bỏ qua backslash + newline; ngược lại escape ký tự kế.
                match chars.peek() {
                    Some('\n') | Some('\r') => {
                        chars.next();
                    }
                    Some(&next_c) => {
                        cur.push(next_c);
                        has_token = true;
                        chars.next();
                    }
                    None => {}
                }
            }
            '\'' => {
                has_token = true;
                for ic in chars.by_ref() {
                    if ic == '\'' {
                        break;
                    }
                    cur.push(ic);
                }
            }
            '"' => {
                has_token = true;
                while let Some(ic) = chars.next() {
                    match ic {
                        '"' => break,
                        '\\' => {
                            if let Some(&e) = chars.peek() {
                                if e == '"' || e == '\\' || e == '$' || e == '`' {
                                    cur.push(e);
                                    chars.next();
                                } else {
                                    cur.push('\\');
                                }
                            }
                        }
                        _ => cur.push(ic),
                    }
                }
            }
            c if c.is_whitespace() => {
                if has_token {
                    tokens.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            _ => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        tokens.push(cur);
    }
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Export: RequestSpec → curl
// ---------------------------------------------------------------------------

/// Sinh lệnh `curl` từ một `RequestSpec` (nhiều dòng, dễ đọc).
pub fn to_curl(spec: &RequestSpec) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push("curl".to_string());

    if spec.method.as_str() != "GET" {
        parts.push(format!("-X {}", spec.method.as_str()));
    }

    // URL + query.
    let mut url = spec.url.clone();
    let enabled_q: Vec<&KeyValue> = spec.query.iter().filter(|q| q.enabled && !q.key.is_empty()).collect();
    if !enabled_q.is_empty() {
        let sep = if url.contains('?') { '&' } else { '?' };
        let qs: Vec<String> = enabled_q
            .iter()
            .map(|q| format!("{}={}", encode(&q.key), encode(&q.value)))
            .collect();
        url = format!("{url}{sep}{}", qs.join("&"));
    }
    parts.push(format!("'{}'", url.replace('\'', "'\\''")));

    // Headers.
    for h in spec.headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        parts.push(format!("-H '{}: {}'", h.key, h.value));
    }

    // Auth.
    match &spec.auth {
        Auth::Bearer { token } => parts.push(format!("-H 'Authorization: Bearer {token}'")),
        Auth::Basic { username, password } => parts.push(format!("-u '{username}:{password}'")),
        Auth::ApiKey { key, value, location } => {
            if matches!(location, ApiKeyLocation::Header) {
                parts.push(format!("-H '{key}: {value}'"));
            }
        }
        _ => {}
    }

    // Body.
    match &spec.body {
        RequestBody::Text { content, content_type } => {
            if let Some(ct) = content_type {
                if !spec.headers.iter().any(|h| h.key.eq_ignore_ascii_case("content-type")) {
                    parts.push(format!("-H 'Content-Type: {ct}'"));
                }
            }
            parts.push(format!("--data-raw '{}'", content.replace('\'', "'\\''")));
        }
        RequestBody::Form { fields } => {
            for f in fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                parts.push(format!("--data-urlencode '{}={}'", f.key, f.value));
            }
        }
        RequestBody::Multipart { parts: mp } => {
            for p in mp.iter().filter(|p| p.enabled && !p.name.is_empty()) {
                match &p.file_path {
                    Some(path) => parts.push(format!("-F '{}=@{}'", p.name, path)),
                    None => parts.push(format!("-F '{}={}'", p.name, p.value)),
                }
            }
        }
        RequestBody::BinaryFile { path, .. } => {
            parts.push(format!("--data-binary '@{path}'"));
        }
        RequestBody::None => {}
    }

    if !spec.verify_tls {
        parts.push("-k".to_string());
    }

    parts.join(" \\\n  ")
}

fn encode(s: &str) -> String {
    // URL-encode tối thiểu cho query trong export.
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_get() {
        let spec = parse("curl https://example.com").unwrap();
        assert_eq!(spec.method.as_str(), "GET");
        assert_eq!(spec.url, "https://example.com");
    }

    #[test]
    fn parses_post_with_json_and_headers() {
        let cmd = r#"curl -X POST 'https://api.test/login' \
          -H 'Content-Type: application/json' \
          -H "Authorization: Bearer abc123" \
          -d '{"email":"a@b.com","pw":"x"}'"#;
        let spec = parse(cmd).unwrap();
        assert_eq!(spec.method.as_str(), "POST");
        assert_eq!(spec.url, "https://api.test/login");
        assert_eq!(spec.headers.len(), 2);
        match spec.body {
            RequestBody::Text { content, content_type } => {
                assert!(content.contains("a@b.com"));
                // Có Content-Type header rồi nên body content_type = None.
                assert_eq!(content_type, None);
            }
            _ => panic!("expected text body"),
        }
    }

    #[test]
    fn infers_post_from_data() {
        let spec = parse("curl https://x.com -d 'a=1&b=2'").unwrap();
        assert_eq!(spec.method.as_str(), "POST");
        match spec.body {
            RequestBody::Text { content_type, .. } => {
                assert_eq!(content_type.as_deref(), Some("application/x-www-form-urlencoded"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parses_basic_auth_and_insecure() {
        let spec = parse("curl -u user:pass -k https://x.com").unwrap();
        assert!(matches!(spec.auth, Auth::Basic { .. }));
        assert!(!spec.verify_tls);
    }

    #[test]
    fn parses_multipart_form_with_file() {
        let spec = parse("curl -F 'field=value' -F 'doc=@/tmp/a.pdf' https://x.com").unwrap();
        match spec.body {
            RequestBody::Multipart { parts } => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[1].file_path.as_deref(), Some("/tmp/a.pdf"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn no_url_is_error() {
        assert_eq!(parse("curl -X POST").unwrap_err(), CurlError::NoUrl);
    }

    #[test]
    fn roundtrip_export_import() {
        let mut spec = RequestSpec::get("https://api.test/x");
        spec.method = HttpMethod::new("POST");
        spec.headers.push(kv("X-Trace", "1"));
        spec.body = RequestBody::Text {
            content: "{\"a\":1}".into(),
            content_type: Some("application/json".into()),
        };
        let curl = to_curl(&spec);
        assert!(curl.contains("-X POST"));
        assert!(curl.contains("api.test/x"));
        let back = parse(&curl).unwrap();
        assert_eq!(back.method.as_str(), "POST");
        assert!(back.headers.iter().any(|h| h.key == "X-Trace"));
    }
}
