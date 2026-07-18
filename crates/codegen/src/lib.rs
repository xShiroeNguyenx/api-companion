//! # codegen — Sinh snippet code request từ `RequestSpec` cho nhiều ngôn ngữ.
//!
//! Logic thuần (không I/O) → dễ golden-test. Target `Curl` uỷ quyền cho `curl-tools`.
//! Body giữ nguyên `{{var}}` (giống hành vi export cURL) — không resolve biến ở đây.

use base64::Engine;
use ipc_types::{
    ApiKeyLocation, Auth, CodegenTarget, CodegenTargetInfo, RequestBody, RequestSpec,
};

/// Danh sách target cho UI (id + nhãn hiển thị).
pub fn targets() -> Vec<CodegenTargetInfo> {
    use CodegenTarget::*;
    [
        (Curl, "cURL"),
        (HttpRaw, "HTTP (raw)"),
        (JsFetch, "JavaScript — fetch"),
        (JsAxios, "JavaScript — axios"),
        (NodeFetch, "Node.js — node-fetch"),
        (PythonRequests, "Python — requests"),
        (PythonHttpx, "Python — httpx"),
        (GoNetHttp, "Go — net/http"),
        (PhpCurl, "PHP — cURL"),
        (RustReqwest, "Rust — reqwest"),
    ]
    .into_iter()
    .map(|(id, label)| CodegenTargetInfo { id, label: label.to_string() })
    .collect()
}

/// Sinh code cho `target` từ `spec`.
pub fn generate(spec: &RequestSpec, target: CodegenTarget) -> String {
    match target {
        CodegenTarget::Curl => curl_tools::to_curl(spec),
        CodegenTarget::HttpRaw => gen_http_raw(&prepare(spec)),
        CodegenTarget::JsFetch => gen_js_fetch(&prepare(spec), false),
        CodegenTarget::NodeFetch => gen_js_fetch(&prepare(spec), true),
        CodegenTarget::JsAxios => gen_js_axios(&prepare(spec)),
        CodegenTarget::PythonRequests => gen_python(&prepare(spec), "requests"),
        CodegenTarget::PythonHttpx => gen_python(&prepare(spec), "httpx"),
        CodegenTarget::GoNetHttp => gen_go(&prepare(spec)),
        CodegenTarget::PhpCurl => gen_php(&prepare(spec)),
        CodegenTarget::RustReqwest => gen_rust(&prepare(spec)),
    }
}

// ---------------------------------------------------------------------------
// Prepared: dạng trung gian target-agnostic
// ---------------------------------------------------------------------------

enum Body {
    None,
    /// Body dạng text (raw/json hoặc form đã urlencode).
    Text(String),
    /// Không biểu diễn được thành text (multipart/binary) → chú thích.
    Note(String),
}

struct Prepared {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Body,
}

fn prepare(spec: &RequestSpec) -> Prepared {
    let method = spec.method.as_str().to_string();

    // URL + query enabled.
    let mut url = spec.url.clone();
    let q: Vec<String> = spec
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .map(|q| format!("{}={}", url_enc(&q.key), url_enc(&q.value)))
        .collect();
    if !q.is_empty() {
        let sep = if url.contains('?') { '&' } else { '?' };
        url = format!("{url}{sep}{}", q.join("&"));
    }

    let mut headers: Vec<(String, String)> = spec
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.is_empty())
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();

    // Auth → header (hoặc query cho api-key location=query).
    match &spec.auth {
        Auth::Bearer { token } => headers.push(("Authorization".into(), format!("Bearer {token}"))),
        Auth::Basic { username, password } => {
            let b64 = base64::engine::general_purpose::STANDARD
                .encode(format!("{username}:{password}"));
            headers.push(("Authorization".into(), format!("Basic {b64}")));
        }
        Auth::ApiKey { key, value, location } => match location {
            ApiKeyLocation::Header => headers.push((key.clone(), value.clone())),
            ApiKeyLocation::Query => {
                let sep = if url.contains('?') { '&' } else { '?' };
                url = format!("{url}{sep}{}={}", url_enc(key), url_enc(value));
            }
        },
        _ => {}
    }

    let has_ct = |hs: &[(String, String)]| hs.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"));

    let body = match &spec.body {
        RequestBody::None => Body::None,
        RequestBody::Text { content, content_type } => {
            if let Some(ct) = content_type {
                if !has_ct(&headers) {
                    headers.push(("Content-Type".into(), ct.clone()));
                }
            }
            Body::Text(content.clone())
        }
        RequestBody::Form { fields } => {
            let enc: Vec<String> = fields
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .map(|f| format!("{}={}", url_enc(&f.key), url_enc(&f.value)))
                .collect();
            if !has_ct(&headers) {
                headers.push(("Content-Type".into(), "application/x-www-form-urlencoded".into()));
            }
            Body::Text(enc.join("&"))
        }
        RequestBody::Multipart { .. } => {
            Body::Note("multipart/form-data — thêm các phần file theo tài liệu thư viện".into())
        }
        RequestBody::BinaryFile { path, .. } => Body::Note(format!("binary file: {path}")),
    };

    Prepared { method, url, headers, body }
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

fn gen_http_raw(p: &Prepared) -> String {
    let (host, path) = split_url(&p.url);
    let mut out = format!("{} {} HTTP/1.1\n", p.method, path);
    out.push_str(&format!("Host: {host}\n"));
    for (k, v) in &p.headers {
        out.push_str(&format!("{k}: {v}\n"));
    }
    out.push('\n');
    match &p.body {
        Body::Text(s) => out.push_str(s),
        Body::Note(n) => out.push_str(&format!("<{n}>")),
        Body::None => {}
    }
    out
}

fn gen_js_fetch(p: &Prepared, node: bool) -> String {
    let mut out = String::new();
    if node {
        out.push_str("const fetch = require(\"node-fetch\");\n\n");
    }
    out.push_str(&format!("const res = await fetch({}, {{\n", jstr(&p.url)));
    out.push_str(&format!("  method: {},\n", jstr(&p.method)));
    if !p.headers.is_empty() {
        out.push_str("  headers: {\n");
        for (k, v) in &p.headers {
            out.push_str(&format!("    {}: {},\n", jstr(k), jstr(v)));
        }
        out.push_str("  },\n");
    }
    match &p.body {
        Body::Text(s) => out.push_str(&format!("  body: {},\n", jstr(s))),
        Body::Note(n) => out.push_str(&format!("  // body: {n}\n")),
        Body::None => {}
    }
    out.push_str("});\nconst data = await res.json();\nconsole.log(data);\n");
    out
}

fn gen_js_axios(p: &Prepared) -> String {
    let mut out = String::from("const res = await axios({\n");
    out.push_str(&format!("  method: {},\n", jstr(&p.method.to_lowercase())));
    out.push_str(&format!("  url: {},\n", jstr(&p.url)));
    if !p.headers.is_empty() {
        out.push_str("  headers: {\n");
        for (k, v) in &p.headers {
            out.push_str(&format!("    {}: {},\n", jstr(k), jstr(v)));
        }
        out.push_str("  },\n");
    }
    match &p.body {
        Body::Text(s) => out.push_str(&format!("  data: {},\n", jstr(s))),
        Body::Note(n) => out.push_str(&format!("  // data: {n}\n")),
        Body::None => {}
    }
    out.push_str("});\nconsole.log(res.data);\n");
    out
}

fn gen_python(p: &Prepared, lib: &str) -> String {
    let mut out = format!("import {lib}\n\n");
    if !p.headers.is_empty() {
        out.push_str("headers = {\n");
        for (k, v) in &p.headers {
            out.push_str(&format!("    {}: {},\n", jstr(k), jstr(v)));
        }
        out.push_str("}\n");
    } else {
        out.push_str("headers = {}\n");
    }
    let data_arg = match &p.body {
        Body::Text(s) => {
            out.push_str(&format!("data = {}\n", jstr(s)));
            "\n    data=data,"
        }
        Body::Note(n) => {
            out.push_str(&format!("# body: {n}\n"));
            ""
        }
        Body::None => "",
    };
    out.push_str(&format!(
        "\nresp = {lib}.request(\n    {},\n    {},\n    headers=headers,{}\n)\nprint(resp.status_code)\nprint(resp.text)\n",
        jstr(&p.method),
        jstr(&p.url),
        data_arg,
    ));
    out
}

fn gen_go(p: &Prepared) -> String {
    let mut out = String::from("package main\n\nimport (\n\t\"fmt\"\n\t\"io\"\n\t\"net/http\"\n");
    let has_body = matches!(p.body, Body::Text(_));
    if has_body {
        out.push_str("\t\"strings\"\n");
    }
    out.push_str(")\n\nfunc main() {\n");
    let body_arg = match &p.body {
        Body::Text(s) => {
            out.push_str(&format!("\tbody := strings.NewReader({})\n", jstr(s)));
            "body"
        }
        Body::Note(n) => {
            out.push_str(&format!("\t// body: {n}\n"));
            "nil"
        }
        Body::None => "nil",
    };
    out.push_str(&format!(
        "\treq, _ := http.NewRequest({}, {}, {})\n",
        jstr(&p.method),
        jstr(&p.url),
        body_arg,
    ));
    for (k, v) in &p.headers {
        out.push_str(&format!("\treq.Header.Set({}, {})\n", jstr(k), jstr(v)));
    }
    out.push_str(
        "\tresp, err := http.DefaultClient.Do(req)\n\tif err != nil {\n\t\tpanic(err)\n\t}\n\tdefer resp.Body.Close()\n\tdata, _ := io.ReadAll(resp.Body)\n\tfmt.Println(string(data))\n}\n",
    );
    out
}

fn gen_php(p: &Prepared) -> String {
    let mut out = String::from("<?php\n$ch = curl_init();\n");
    out.push_str(&format!("curl_setopt($ch, CURLOPT_URL, {});\n", php_str(&p.url)));
    out.push_str(&format!(
        "curl_setopt($ch, CURLOPT_CUSTOMREQUEST, {});\n",
        php_str(&p.method)
    ));
    out.push_str("curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);\n");
    if !p.headers.is_empty() {
        let hs: Vec<String> = p
            .headers
            .iter()
            .map(|(k, v)| php_str(&format!("{k}: {v}")))
            .collect();
        out.push_str(&format!(
            "curl_setopt($ch, CURLOPT_HTTPHEADER, [{}]);\n",
            hs.join(", ")
        ));
    }
    match &p.body {
        Body::Text(s) => out.push_str(&format!(
            "curl_setopt($ch, CURLOPT_POSTFIELDS, {});\n",
            php_str(s)
        )),
        Body::Note(n) => out.push_str(&format!("// body: {n}\n")),
        Body::None => {}
    }
    out.push_str("$response = curl_exec($ch);\ncurl_close($ch);\necho $response;\n");
    out
}

fn gen_rust(p: &Prepared) -> String {
    let mut out = String::from(
        "#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n    let client = reqwest::Client::new();\n    let res = client\n",
    );
    out.push_str(&format!(
        "        .request(reqwest::Method::from_bytes(b{}).unwrap(), {})\n",
        jstr(&p.method),
        jstr(&p.url),
    ));
    for (k, v) in &p.headers {
        out.push_str(&format!("        .header({}, {})\n", jstr(k), jstr(v)));
    }
    match &p.body {
        Body::Text(s) => out.push_str(&format!("        .body({})\n", rust_str(s))),
        Body::Note(n) => out.push_str(&format!("        // body: {n}\n")),
        Body::None => {}
    }
    out.push_str(
        "        .send()\n        .await?;\n    println!(\"{}\", res.text().await?);\n    Ok(())\n}\n",
    );
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// URL-encode tối thiểu (giống curl-tools export).
fn url_enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Chuỗi kiểu JSON — hợp lệ cho JS/JSON/Python/Go (giữ non-ASCII literal UTF-8).
fn jstr(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Chuỗi PHP single-quoted (chỉ escape `\` và `'`, không nội suy biến `$`).
fn php_str(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// Chuỗi Rust: raw string `r#"..."#` nếu an toàn, else escaped (tránh `\uXXXX` không hợp lệ).
fn rust_str(s: &str) -> String {
    if !s.contains("\"#") {
        format!("r#\"{s}\"#")
    } else {
        // Fallback: escaped, non-ASCII giữ literal (Rust chấp nhận UTF-8 trong "...").
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }
}

/// Tách `(host, path_with_query)` từ URL (best-effort, không cần crate url).
fn split_url(url: &str) -> (String, String) {
    let after_scheme = url.splitn(2, "://").nth(1).unwrap_or(url);
    match after_scheme.find('/') {
        Some(i) => (after_scheme[..i].to_string(), after_scheme[i..].to_string()),
        None => (after_scheme.to_string(), "/".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::{HttpMethod, KeyValue};

    fn post_json() -> RequestSpec {
        let mut s = RequestSpec::get("https://api.example.com/users");
        s.method = HttpMethod::new("POST");
        s.headers = vec![KeyValue { key: "X-Api".into(), value: "1".into(), enabled: true }];
        s.auth = Auth::Bearer { token: "TKN".into() };
        s.body = RequestBody::Text {
            content: "{\"name\":\"a\"}".into(),
            content_type: Some("application/json".into()),
        };
        s
    }

    #[test]
    fn targets_count() {
        assert_eq!(targets().len(), 10);
    }

    #[test]
    fn curl_delegates() {
        let s = post_json();
        assert_eq!(generate(&s, CodegenTarget::Curl), curl_tools::to_curl(&s));
    }

    #[test]
    fn fetch_has_essentials() {
        let out = generate(&post_json(), CodegenTarget::JsFetch);
        assert!(out.contains("fetch(\"https://api.example.com/users\""));
        assert!(out.contains("method: \"POST\""));
        assert!(out.contains("\"Authorization\": \"Bearer TKN\""));
        assert!(out.contains("body: \"{\\\"name\\\":\\\"a\\\"}\""));
    }

    #[test]
    fn python_and_go_and_php_and_rust_render() {
        let s = post_json();
        assert!(generate(&s, CodegenTarget::PythonRequests).contains("import requests"));
        assert!(generate(&s, CodegenTarget::PythonHttpx).contains("import httpx"));
        assert!(generate(&s, CodegenTarget::GoNetHttp).contains("http.NewRequest(\"POST\""));
        assert!(generate(&s, CodegenTarget::PhpCurl).contains("curl_setopt($ch, CURLOPT_URL, 'https://api.example.com/users')"));
        assert!(generate(&s, CodegenTarget::RustReqwest).contains("reqwest::Client::new()"));
    }

    #[test]
    fn query_and_apikey_query_appended() {
        let mut s = RequestSpec::get("https://x.com/a");
        s.query = vec![KeyValue { key: "q".into(), value: "hello world".into(), enabled: true }];
        s.auth = Auth::ApiKey {
            key: "token".into(),
            value: "SEC".into(),
            location: ApiKeyLocation::Query,
        };
        let out = generate(&s, CodegenTarget::JsFetch);
        assert!(out.contains("q=hello%20world"));
        assert!(out.contains("token=SEC"));
    }

    #[test]
    fn http_raw_splits_host_path() {
        let out = generate(&post_json(), CodegenTarget::HttpRaw);
        assert!(out.starts_with("POST /users HTTP/1.1\nHost: api.example.com\n"));
    }
}
